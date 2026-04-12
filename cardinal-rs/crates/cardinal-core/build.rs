use std::path::PathBuf;
use std::process::Command;

fn main() {
    let cardinal_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .parent().unwrap()
        .to_path_buf();

    let rack_dir = cardinal_root.join("src/Rack");
    let rack_src = rack_dir.join("src");
    let rack_dep = rack_dir.join("dep");

    // ── Ensure submodules ────────────────────────────────────────────
    if !rack_dir.join("include/engine/Engine.hpp").exists() {
        eprintln!("Initializing git submodules...");
        run_or_panic(
            Command::new("git")
                .args(["submodule", "update", "--init", "--recursive"])
                .current_dir(&cardinal_root),
        );
    }

    // ── Shared include paths ─────────────────────────────────────────
    let include_dirs: Vec<PathBuf> = vec![
        rack_dir.join("include"),
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
        cardinal_root.join("dpf/distrho"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cpp"),
    ];

    // ── 1. Rack engine ───────────────────────────────────────────────
    build_rack_engine(&rack_src, &include_dirs);

    // ── 2. NanoVG GL2 + NanoSVG implementations ──────────────────────
    build_gl_impls(&include_dirs);

    // ── 3. Plugins are built by per-vendor crates via
    //       cardinal-plugins-registry (parallel compilation) ──────────

    // ── 4. C deps ────────────────────────────────────────────────────
    build_c_deps(&rack_dep, &include_dirs, &cardinal_root);

    // ── 5. Bridge ────────────────────────────────────────────────────
    build_bridge(&include_dirs);

    // Allow multiple definitions of stb_image, freeverb, and other
    // single-file libraries that get compiled into multiple plugin crates.
    // The linker picks the first definition.
    println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition");

    // ── System libraries ─────────────────────────────────────────────
    for lib in &["jansson", "archive", "samplerate", "speexdsp", "pthread", "dl", "GL", "GLEW", "EGL"] {
        println!("cargo:rustc-link-lib={lib}");
    }

    // ── Rerun triggers ───────────────────────────────────────────────
    for f in &["bridge.h", "bridge.cpp", "stubs.cpp", "plugin_init.cpp"] {
        println!("cargo:rerun-if-changed=cpp/{f}");
    }
}

fn build_rack_engine(rack_src: &PathBuf, includes: &[PathBuf]) {
    let skip: &[&str] = &[
        "asset", "audio", "common", "dep", "discord", "gamepad", "keyboard",
        "library", "midiloopback", "network", "rtaudio", "rtmidi",
        "AudioDisplay", "MidiDisplay", "Browser", "MenuBar", "TipWindow",
        "Scene", "Window",
    ];

    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").warnings(false)
        .define("PRIVATE", "").define("ARCH_X64", None).define("ARCH_LIN", None);
    for d in includes { build.include(d); }

    for entry in walkdir(rack_src.to_str().unwrap()) {
        let path = PathBuf::from(&entry);
        if path.extension().map_or(true, |e| e != "cpp") { continue; }
        if path.to_str().unwrap().contains("/core/") { continue; }
        if path.file_name().unwrap() == "plugin.cpp"
            && path.parent().unwrap().file_name().unwrap() == "src" { continue; }
        let stem = path.file_stem().unwrap().to_str().unwrap();
        if skip.contains(&stem) { continue; }
        build.file(&path);
    }
    // Use cargo_metadata=false to prevent cc from emitting its own
    // cargo:rustc-link-lib directive. We emit it manually with +whole-archive.
    build.cargo_metadata(false);
    build.compile("rack_engine");

    // Emit whole-archive link so all Rack symbols are available to plugin crates
    let out_dir = std::env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static:+whole-archive=rack_engine");
}

fn build_gl_impls(includes: &[PathBuf]) {
    let bridge_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cpp");
    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").warnings(false)
        .define("PRIVATE", "").define("ARCH_X64", None).define("ARCH_LIN", None);

    // Find system GLEW via pkg-config (works on NixOS and normal distros)
    if let Ok(glew) = pkg_config::probe_library("glew") {
        for path in &glew.include_paths {
            build.include(path);
        }
    } else {
        build.include("/usr/include");
    }

    for d in includes {
        // Skip Cardinal's stub GL/glew.h
        if d.ends_with("include") && d.to_str().unwrap().contains("Cardinal/include") { continue; }
        build.include(d);
    }
    build.file(bridge_dir.join("nanovg_gl_impl.cpp"));
    build.file(bridge_dir.join("nanosvg_impl.cpp"));
    build.cargo_metadata(false);
    build.compile("rack_gl_impl");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static:+whole-archive=rack_gl_impl");
}

fn build_c_deps(rack_dep: &PathBuf, includes: &[PathBuf], _cardinal_root: &PathBuf) {
    let mut build = cc::Build::new();
    build.warnings(false);
    for d in includes { build.include(d); }

    build.file(rack_dep.join("nanovg/src/nanovg.c"));
    build.file(rack_dep.join("tinyexpr/tinyexpr.c"));
    build.file(rack_dep.join("pffft/pffft.c"));
    build.file(rack_dep.join("pffft/fftpack.c"));
    build.file(rack_dep.join("oui-blendish/blendish.c"));
    build.cargo_metadata(false);
    build.compile("rack_deps");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static:+whole-archive=rack_deps");
}

fn build_bridge(includes: &[PathBuf]) {
    let bridge_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cpp");
    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").warnings(false)
        .define("PRIVATE", "").define("ARCH_X64", None).define("ARCH_LIN", None);
    for d in includes { build.include(d); }
    build.file(bridge_dir.join("bridge.cpp"));
    build.file(bridge_dir.join("stubs.cpp"));
    build.compile("bridge");
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

fn run_or_panic(cmd: &mut Command) {
    let status = cmd.status().expect("failed to run command");
    assert!(status.success(), "command failed: {status}");
}
