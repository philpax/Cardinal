use std::path::PathBuf;

fn main() {
    let cardinal_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()  // crates/
        .parent().unwrap()  // cardinal-rs/
        .parent().unwrap()  // Cardinal/
        .to_path_buf();

    let plugins_dir = cardinal_root.join("plugins");
    let plugin_dir = plugins_dir.join("WSTD-Drums");

    if !plugin_dir.exists() {
        eprintln!("Plugin WSTD-Drums not found (submodule not initialized?), skipping");
        return;
    }

    let rack_dir = cardinal_root.join("src/Rack");

    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").warnings(false)
        .define("ARCH_X64", None)
        .define("ARCH_LIN", None)
        .define("BUILDING_PLUGIN_MODULES", None);

    // Rack includes
    for dir in &[
        rack_dir.join("include"),
        rack_dir.join("dep/glfw/include"),
        rack_dir.join("dep/nanovg/src"),
        rack_dir.join("dep/nanosvg/src"),
        rack_dir.join("dep/oui-blendish"),
        rack_dir.join("dep/osdialog"),
        rack_dir.join("dep/simde"),
        rack_dir.join("dep/filesystem/include"),
        rack_dir.join("dep/tinyexpr"),
        rack_dir.join("dep/pffft"),
        cardinal_root.join("include"),
        cardinal_root.join("plugins"),
        cardinal_root.join("dpf/distrho"),
    ] {
        build.include(dir);
    }

    // Plugin-local includes — recursively add src/ and dep subdirs
    fn add_dirs(build: &mut cc::Build, dir: &std::path::Path, depth: u32) {
        if depth > 4 || !dir.exists() { return; }
        build.include(dir);
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                if e.path().is_dir() {
                    add_dirs(build, &e.path(), depth + 1);
                }
            }
        }
    }
    add_dirs(&mut build, &plugin_dir.join("src"), 0);
    for dep_dir in ["dep", "deps", "lib"] {
        add_dirs(&mut build, &plugin_dir.join(dep_dir), 0);
    }

    // Symbol renames to avoid cross-plugin collisions
    build.define("pluginInstance", "pluginInstance__WSTD_Drums");
    build.define("init", "init__WSTD_Drums");
    build.define("ADSR", "WSTD-DrumsADSR");
    build.define("modelADSR", "modelWSTD-DrumsADSR");
    build.define("ADSRWidget", "WSTD-DrumsADSRWidget");
    build.define("Envelope", "WSTD-DrumsEnvelope");
    build.define("modelEnvelope", "modelWSTD-DrumsEnvelope");
    build.define("EnvelopeWidget", "WSTD-DrumsEnvelopeWidget");
    build.define("LowFrequencyOscillator", "WSTD-DrumsLowFrequencyOscillator");
    build.define("modelLowFrequencyOscillator", "modelWSTD-DrumsLowFrequencyOscillator");
    build.define("LowFrequencyOscillatorWidget", "WSTD-DrumsLowFrequencyOscillatorWidget");

    // Filter-out list
    let filter_out: Vec<String> = vec![

    ];

    // Source files

    // Glob WSTD-Drums/deps/*.cpp
    if let Ok(entries) = std::fs::read_dir(plugins_dir.join("WSTD-Drums/deps")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(true, |e| e != "cpp" && e != "cc" && e != "c") { continue; }
            let rel = path.strip_prefix(&plugins_dir).unwrap().to_str().unwrap().to_string();
            if !filter_out.contains(&rel) {
                build.file(&path);
            }
        }
    }

    // Glob WSTD-Drums/deps/SynthDevKit/src/*.cpp
    if let Ok(entries) = std::fs::read_dir(plugins_dir.join("WSTD-Drums/deps/SynthDevKit/src")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(true, |e| e != "cpp" && e != "cc" && e != "c") { continue; }
            let rel = path.strip_prefix(&plugins_dir).unwrap().to_str().unwrap().to_string();
            if !filter_out.contains(&rel) {
                build.file(&path);
            }
        }
    }

    // Glob WSTD-Drums/src/*.cpp
    if let Ok(entries) = std::fs::read_dir(plugins_dir.join("WSTD-Drums/src")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(true, |e| e != "cpp" && e != "cc" && e != "c") { continue; }
            let rel = path.strip_prefix(&plugins_dir).unwrap().to_str().unwrap().to_string();
            if !filter_out.contains(&rel) {
                build.file(&path);
            }
        }
    }

    // Glob WSTD-Drums/src/controller/*.cpp
    if let Ok(entries) = std::fs::read_dir(plugins_dir.join("WSTD-Drums/src/controller")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(true, |e| e != "cpp" && e != "cc" && e != "c") { continue; }
            let rel = path.strip_prefix(&plugins_dir).unwrap().to_str().unwrap().to_string();
            if !filter_out.contains(&rel) {
                build.file(&path);
            }
        }
    }

    // Glob WSTD-Drums/src/model/*.cpp
    if let Ok(entries) = std::fs::read_dir(plugins_dir.join("WSTD-Drums/src/model")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(true, |e| e != "cpp" && e != "cc" && e != "c") { continue; }
            let rel = path.strip_prefix(&plugins_dir).unwrap().to_str().unwrap().to_string();
            if !filter_out.contains(&rel) {
                build.file(&path);
            }
        }
    }

    // Glob WSTD-Drums/src/view/*.cpp
    if let Ok(entries) = std::fs::read_dir(plugins_dir.join("WSTD-Drums/src/view")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(true, |e| e != "cpp" && e != "cc" && e != "c") { continue; }
            let rel = path.strip_prefix(&plugins_dir).unwrap().to_str().unwrap().to_string();
            if !filter_out.contains(&rel) {
                build.file(&path);
            }
        }
    }

    build.compile("cardinal_plugin_wstd_drums");
}
