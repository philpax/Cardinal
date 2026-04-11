use std::path::PathBuf;
use std::process::Command;

fn main() {
    // cardinal-rs/crates/cardinal-core -> crates -> cardinal-rs -> Cardinal
    let cardinal_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .unwrap()
        .parent() // cardinal-rs/
        .unwrap()
        .parent() // Cardinal/
        .unwrap()
        .to_path_buf();

    let rack_dir = cardinal_root.join("src/Rack");
    let rack_src = rack_dir.join("src");
    let rack_include = rack_dir.join("include");
    let rack_dep = rack_dir.join("dep");

    // ── Ensure submodules are initialized ────────────────────────────
    if !rack_include.join("engine/Engine.hpp").exists() {
        eprintln!("Initializing git submodules...");
        run_or_panic(
            Command::new("git")
                .args(["submodule", "update", "--init", "--recursive"])
                .current_dir(&cardinal_root),
        );
        assert!(
            rack_include.join("engine/Engine.hpp").exists(),
            "src/Rack submodule still missing after init"
        );
    }

    // ── Shared include paths ─────────────────────────────────────────
    let include_dirs: Vec<PathBuf> = vec![
        rack_include.clone(),
        rack_dep.join("glfw/include"),
        rack_dep.join("nanovg/src"),
        rack_dep.join("nanosvg/src"),
        rack_dep.join("oui-blendish"),
        rack_dep.join("osdialog"),
        rack_dep.join("simde"),
        rack_dep.join("filesystem/include"),
        rack_dep.join("tinyexpr"),
        rack_dep.join("pffft"),
        cardinal_root.join("include"),
        cardinal_root.join("plugins"),
        cardinal_root.join("dpf/distrho"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cpp"),
    ];

    // ── Build the Rack engine (internal code, PRIVATE=empty) ─────────
    build_rack_engine(&rack_src, &rack_dep, &include_dirs);

    // ── Build plugins (plugin code, uses rack.hpp which redefines PRIVATE) ──
    build_plugins(&cardinal_root, &include_dirs);

    // ── Build C deps + bridge files (single archive for link ordering) ──
    build_bridge_and_deps(&rack_dep, &include_dirs);

    // Force the linker to keep all symbols from our static libraries
    // (resolves cross-archive dependencies between rack_engine, rack_gl_impl, etc.)
    println!("cargo:rustc-link-arg=-Wl,--whole-archive");
    // The cc crate archives are automatically linked; this flag ensures
    // they're fully included rather than selectively resolved.
    // We close it after system libs.

    // ── System libraries ─────────────────────────────────────────────
    println!("cargo:rustc-link-lib=jansson");
    println!("cargo:rustc-link-lib=archive");
    println!("cargo:rustc-link-lib=samplerate");
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=dl");
    println!("cargo:rustc-link-lib=GL");
    println!("cargo:rustc-link-lib=GLEW");
    println!("cargo:rustc-link-lib=EGL");

    println!("cargo:rustc-link-arg=-Wl,--no-whole-archive");

    // ── Rerun triggers ───────────────────────────────────────────────
    println!("cargo:rerun-if-changed=cpp/bridge.h");
    println!("cargo:rerun-if-changed=cpp/bridge.cpp");
    println!("cargo:rerun-if-changed=cpp/stubs.cpp");
    println!("cargo:rerun-if-changed=cpp/plugin_init.cpp");
}

fn build_rack_engine(rack_src: &PathBuf, rack_dep: &PathBuf, includes: &[PathBuf]) {
    let skip_basenames: &[&str] = &[
        "asset", "audio", "common", "dep", "discord", "gamepad", "keyboard",
        "library", "midi", "midiloopback", "network", "rtaudio", "rtmidi",
        "AudioDisplay", "MidiDisplay", "Browser", "MenuBar", "TipWindow",
        "Scene", "Window",
    ];

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .define("PRIVATE", "")
        .define("ARCH_X64", None)
        .define("ARCH_LIN", None)
        .warnings(false);

    for dir in includes {
        build.include(dir);
    }
    // Also include nanosvg src for SVG rendering
    build.include(rack_dep.join("nanosvg/src"));

    for entry in walkdir(rack_src.to_str().unwrap()) {
        let path = PathBuf::from(&entry);
        if path.extension().map_or(true, |e| e != "cpp") {
            continue;
        }
        if path.to_str().unwrap().contains("/core/") {
            continue;
        }
        if path.file_name().unwrap() == "plugin.cpp"
            && path.parent().unwrap().file_name().unwrap() == "src"
        {
            continue;
        }
        let stem = path.file_stem().unwrap().to_str().unwrap();
        if skip_basenames.contains(&stem) {
            continue;
        }
        build.file(&path);
    }

    build.compile("rack_engine");

    // NanoVG GL impl and NanoSVG impl need to be compiled as C, not C++,
    // but linked into the same archive order. We compile them separately
    // but they'll be linked after rack_engine.
    {
        let bridge_cpp_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cpp");
        let mut gl_impl = cc::Build::new();
        gl_impl.cpp(true).std("c++17").warnings(false);
        gl_impl.define("PRIVATE", "");
        gl_impl.define("ARCH_X64", None);
        gl_impl.define("ARCH_LIN", None);
        // System GLEW must come BEFORE Cardinal's stub glew.h
        gl_impl.include("/usr/include");
        for dir in includes {
            // Skip Cardinal's include/ dir which has a stub GL/glew.h
            if dir.ends_with("include") && dir.to_str().unwrap().contains("Cardinal/include") {
                continue;
            }
            gl_impl.include(dir);
        }
        gl_impl.file(bridge_cpp_dir.join("nanovg_gl_impl.cpp"));
        gl_impl.file(bridge_cpp_dir.join("nanosvg_impl.cpp"));
        gl_impl.compile("rack_gl_impl");
    }
}

fn build_plugins(cardinal_root: &PathBuf, includes: &[PathBuf]) {
    let plugins_dir = cardinal_root.join("plugins");

    // Use Cardinal's Makefile to build plugins.a — it handles the massive
    // complexity of per-plugin includes, #define renames, special flags, etc.
    // This is invoked once; the .a is cached by make.
    let headless = std::env::var("CARDINAL_HEADLESS").unwrap_or_default() == "1";
    let target_suffix = if headless { "-headless" } else { "" };
    let lib_name = format!("plugins{target_suffix}.a");
    let lib_path = plugins_dir.join(&lib_name);

    if !lib_path.exists() {
        eprintln!("Building plugins via Cardinal Makefile (this may take a while)...");
        let status = Command::new("make")
            .args(["-C", plugins_dir.to_str().unwrap(), &lib_name])
            .arg(format!("-j{}", num_cpus()))
            .env("HEADLESS", if headless { "true" } else { "" })
            .status();

        match status {
            Ok(s) if s.success() => {}
            Ok(s) => {
                // Make failed — fall back to compiling just plugins.cpp + Fundamental
                eprintln!(
                    "Cardinal plugin Makefile failed ({s}), falling back to minimal plugin build"
                );
                build_plugins_minimal(cardinal_root, includes);
                return;
            }
            Err(e) => {
                eprintln!("Could not run make ({e}), falling back to minimal plugin build");
                build_plugins_minimal(cardinal_root, includes);
                return;
            }
        }
    }

    if lib_path.exists() {
        // Tell cargo to link the pre-built plugins.a
        println!(
            "cargo:rustc-link-search=native={}",
            plugins_dir.display()
        );
        println!(
            "cargo:rustc-link-lib=static=plugins{}",
            target_suffix
        );
    } else {
        build_plugins_minimal(cardinal_root, includes);
    }
}

/// Fallback: compile just plugins.cpp + Fundamental sources directly.
fn build_plugins_minimal(cardinal_root: &PathBuf, includes: &[PathBuf]) {
    eprintln!("Building minimal plugin set (Fundamental only)...");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .define("ARCH_X64", None)
        .define("ARCH_LIN", None)
        .define("BUILDING_PLUGIN_MODULES", None)
        .warnings(false);

    for dir in includes {
        build.include(dir);
    }

    // Use our own plugin_init.cpp for the initStaticPlugins() entry point
    let bridge_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cpp");
    build.file(bridge_dir.join("plugin_init.cpp"));

    // Compile all Fundamental source files (including plugin.cpp which
    // defines the `pluginInstance` global and `init()` function)
    let fundamental_src = cardinal_root.join("plugins/Fundamental/src");
    if fundamental_src.exists() {
        for entry in walkdir(fundamental_src.to_str().unwrap()) {
            let path = PathBuf::from(&entry);
            if path.extension().map_or(true, |e| e != "cpp") {
                continue;
            }
            build.file(&path);
        }
    }

    build.compile("plugins_minimal");

    // drwav implementation (separate archive, needs whole-archive to ensure it's linked)
    {
        let mut drwav = cc::Build::new();
        drwav.cpp(true).std("c++17").warnings(false);
        drwav.include(cardinal_root.join("plugins"));
        drwav.file(bridge_dir.join("drwav_impl.cpp"));
        drwav.compile("drwav_impl");
    }
}

fn build_bridge_and_deps(rack_dep: &PathBuf, includes: &[PathBuf]) {
    let cardinal_root = rack_dep.parent().unwrap().parent().unwrap().parent().unwrap();
    let bridge_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cpp");

    // C++ bridge files
    let mut cpp_build = cc::Build::new();
    cpp_build
        .cpp(true)
        .std("c++17")
        .define("PRIVATE", "")
        .define("ARCH_X64", None)
        .define("ARCH_LIN", None)
        .warnings(false);
    for dir in includes {
        cpp_build.include(dir);
    }
    cpp_build.file(bridge_dir.join("bridge.cpp"));
    cpp_build.file(bridge_dir.join("stubs.cpp"));
    cpp_build.compile("bridge");

    // C dependency files (single archive so linker resolves cross-refs)
    let mut c_build = cc::Build::new();
    c_build.warnings(false);
    for dir in includes {
        c_build.include(dir);
    }
    c_build.include(cardinal_root.join("plugins/Fundamental/src"));

    c_build.file(rack_dep.join("nanovg/src/nanovg.c"));
    c_build.file(rack_dep.join("tinyexpr/tinyexpr.c"));
    c_build.file(rack_dep.join("pffft/pffft.c"));
    c_build.file(rack_dep.join("pffft/fftpack.c"));
    c_build.file(rack_dep.join("oui-blendish/blendish.c"));
    c_build.file(bridge_dir.join("drwav_impl.cpp"));
    c_build.compile("rack_deps");
}

fn walkdir(dir: &str) -> Vec<String> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(walkdir(path.to_str().unwrap()));
            } else {
                results.push(path.to_str().unwrap().to_string());
            }
        }
    }
    results
}

fn num_cpus() -> String {
    std::thread::available_parallelism()
        .map(|n| n.get().to_string())
        .unwrap_or_else(|_| "4".into())
}

fn run_or_panic(cmd: &mut Command) {
    let status = cmd.status().expect("failed to run command");
    assert!(status.success(), "command failed: {status}");
}
