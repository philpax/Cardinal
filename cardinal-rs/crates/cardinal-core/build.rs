use std::path::PathBuf;
use std::process::Command;

fn main() {
    // cardinal-rs/crates/cardinal-core -> crates -> cardinal-rs -> Cardinal
    let cardinal_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()  // crates/
        .unwrap()
        .parent()  // cardinal-rs/
        .unwrap()
        .parent()  // Cardinal/
        .unwrap()
        .to_path_buf();

    let rack_dir = cardinal_root.join("src/Rack");
    let rack_src = rack_dir.join("src");
    let rack_include = rack_dir.join("include");
    let rack_dep = rack_dir.join("dep");

    // ── Ensure submodules are initialized ────────────────────────────
    if !rack_include.join("engine/Engine.hpp").exists() {
        eprintln!("VCV Rack submodule not initialized. Running: git submodule update --init --recursive src/Rack");
        let status = Command::new("git")
            .args(["submodule", "update", "--init", "--recursive", "src/Rack"])
            .current_dir(&cardinal_root)
            .status();
        match status {
            Ok(s) if s.success() => {}
            Ok(s) => panic!("git submodule update failed with {s}"),
            Err(e) => panic!("Failed to run git: {e}. Please run: git submodule update --init --recursive src/Rack"),
        }
        assert!(
            rack_include.join("engine/Engine.hpp").exists(),
            "Submodule init succeeded but src/Rack/include/engine/Engine.hpp still missing"
        );
    }

    // ── Collect Rack C++ source files ────────────────────────────────
    let skip_basenames: &[&str] = &[
        "asset", "audio", "common", "dep", "discord", "gamepad", "keyboard",
        "library", "midi", "midiloopback", "network", "rtaudio", "rtmidi",
        "AudioDisplay", "MidiDisplay", "Browser", "MenuBar", "TipWindow", "Scene",
        "Window", // We provide our own stubs for window functions
    ];

    let mut cpp_files: Vec<PathBuf> = Vec::new();

    for entry in walkdir(rack_src.to_str().unwrap()) {
        let path = PathBuf::from(&entry);
        if path.extension().map_or(true, |e| e != "cpp") {
            continue;
        }
        // Skip core/ subdirectory (depends on audio/midi subsystems)
        if path.to_str().unwrap().contains("/core/") {
            continue;
        }
        // Skip top-level plugin.cpp (but not plugin/Plugin.cpp)
        if path.file_name().unwrap() == "plugin.cpp"
            && path.parent().unwrap().file_name().unwrap() == "src"
        {
            continue;
        }
        let stem = path.file_stem().unwrap().to_str().unwrap();
        if skip_basenames.contains(&stem) {
            continue;
        }
        cpp_files.push(path);
    }

    // ── Bridge files ─────────────────────────────────────────────────
    let bridge_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cpp");
    cpp_files.push(bridge_dir.join("bridge.cpp"));
    cpp_files.push(bridge_dir.join("stubs.cpp"));
    cpp_files.push(bridge_dir.join("test_modules.cpp"));

    // ── C dependency files ───────────────────────────────────────────
    let mut c_files: Vec<PathBuf> = Vec::new();
    c_files.push(rack_dep.join("nanovg/src/nanovg.c"));
    c_files.push(rack_dep.join("tinyexpr/tinyexpr.c"));
    c_files.push(rack_dep.join("pffft/pffft.c"));
    c_files.push(rack_dep.join("pffft/fftpack.c"));
    c_files.push(rack_dep.join("oui-blendish/blendish.c"));

    // ── Include paths ────────────────────────────────────────────────
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
        bridge_dir.clone(),
    ];

    // ── Build C++ ────────────────────────────────────────────────────
    let mut cpp_build = cc::Build::new();
    cpp_build
        .cpp(true)
        .std("c++17")
        .define("PRIVATE", "")
        .define("ARCH_X64", None)
        .define("ARCH_LIN", None)
        .warnings(false);

    for dir in &include_dirs {
        cpp_build.include(dir);
    }
    for file in &cpp_files {
        cpp_build.file(file);
    }
    cpp_build.compile("rack_engine");

    // ── Build C deps ─────────────────────────────────────────────────
    let mut c_build = cc::Build::new();
    c_build.warnings(false);
    for dir in &include_dirs {
        c_build.include(dir);
    }
    for file in &c_files {
        c_build.file(file);
    }
    c_build.compile("rack_deps");

    // ── System libraries ─────────────────────────────────────────────
    println!("cargo:rustc-link-lib=jansson");
    println!("cargo:rustc-link-lib=archive");
    println!("cargo:rustc-link-lib=samplerate");
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=dl");
    println!("cargo:rustc-link-lib=GL");

    // ── Rerun triggers ───────────────────────────────────────────────
    println!("cargo:rerun-if-changed=cpp/bridge.h");
    println!("cargo:rerun-if-changed=cpp/bridge.cpp");
    println!("cargo:rerun-if-changed=cpp/stubs.cpp");
    println!("cargo:rerun-if-changed=cpp/test_modules.cpp");
}

/// Simple recursive directory walk (no external dep needed).
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
