use std::path::PathBuf;

fn main() {
    let cardinal_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()  // plugins/
        .parent().unwrap()  // crates/
        .parent().unwrap()  // cardinal-rs/
        .parent().unwrap()  // Cardinal/
        .to_path_buf();

    let plugins_dir = cardinal_root.join("plugins");
    let plugin_dir = plugins_dir.join("AS");

    if !plugin_dir.exists() {
        eprintln!("Plugin AS not found (submodule not initialized?), skipping");
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
    build.define("pluginInstance", "pluginInstance__AS");
    build.define("ADSR", "ASADSR");
    build.define("modelADSR", "modelASADSR");
    build.define("ADSRWidget", "ASADSRWidget");
    build.define("BpmDisplayWidget", "ASBpmDisplayWidget");
    build.define("modelBpmDisplayWidget", "modelASBpmDisplayWidget");
    build.define("BpmDisplayWidgetWidget", "ASBpmDisplayWidgetWidget");
    build.define("LabelDisplayWidget", "ASLabelDisplayWidget");
    build.define("modelLabelDisplayWidget", "modelASLabelDisplayWidget");
    build.define("LabelDisplayWidgetWidget", "ASLabelDisplayWidgetWidget");
    build.define("LedLight", "ASLedLight");
    build.define("modelLedLight", "modelASLedLight");
    build.define("LedLightWidget", "ASLedLightWidget");
    build.define("LowFrequencyOscillator", "ASLowFrequencyOscillator");
    build.define("modelLowFrequencyOscillator", "modelASLowFrequencyOscillator");
    build.define("LowFrequencyOscillatorWidget", "ASLowFrequencyOscillatorWidget");
    build.define("SigDisplayWidget", "ASSigDisplayWidget");
    build.define("modelSigDisplayWidget", "modelASSigDisplayWidget");
    build.define("SigDisplayWidgetWidget", "ASSigDisplayWidgetWidget");
    build.define("VCA", "ASVCA");
    build.define("modelVCA", "modelASVCA");
    build.define("VCAWidget", "ASVCAWidget");
    build.define("WaveShaper", "ASWaveShaper");
    build.define("modelWaveShaper", "modelASWaveShaper");
    build.define("WaveShaperWidget", "ASWaveShaperWidget");
    build.define("YellowRedLight", "ASYellowRedLight");
    build.define("modelYellowRedLight", "modelASYellowRedLight");
    build.define("YellowRedLightWidget", "ASYellowRedLightWidget");
    build.define("allpass", "ASallpass");
    build.define("modelallpass", "modelASallpass");
    build.define("allpassWidget", "ASallpassWidget");
    build.define("comb", "AScomb");
    build.define("modelcomb", "modelAScomb");
    build.define("combWidget", "AScombWidget");
    build.define("revmodel", "ASrevmodel");
    build.define("modelrevmodel", "modelASrevmodel");
    build.define("revmodelWidget", "ASrevmodelWidget");

    // Filter-out list
    let _filter_out: Vec<String> = vec![
        "AS/src/AS.cpp".to_string(),
    ];

    // Source files

    // Recursively collect source files, skipping test/template dirs
    #[allow(dead_code)]
    fn collect_sources(dir: &std::path::Path, filter_out: &[String], plugins_dir: &std::path::Path, build: &mut cc::Build, depth: u32) {
        if depth > 5 || !dir.exists() { return; }
        // Skip directories that contain test/template/example files
        if let Some(name) = dir.file_name().and_then(|n| n.to_str()) {
            let lower = name.to_lowercase();
            if lower == "test" || lower == "tests" || lower == "template"
                || lower == "templates" || lower == "examples" || lower == "doc"
                || lower == "docs" || lower == "benchmark" || lower == "benchmarks" {
                return;
            }
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    collect_sources(&path, filter_out, plugins_dir, build, depth + 1);
                } else if path.extension().map_or(false, |e| e == "cpp" || e == "cc" || e == "c") {
                    // Skip test files by name
                    let fname = path.file_name().unwrap_or_default().to_str().unwrap_or("");
                    if fname.contains("_test") || fname.contains("test_") || fname == "main.cpp" || fname == "main.c" {
                        continue;
                    }
                    let rel = path.strip_prefix(plugins_dir).unwrap_or(&path).to_str().unwrap_or("").to_string();
                    if !filter_out.contains(&rel) {
                        build.file(&path);
                    }
                }
            }
        }
    }
    collect_sources(&plugins_dir.join("AS/src"), &_filter_out, &plugins_dir, &mut build, 0);
    build.file(plugins_dir.join("AS/freeverb/revmodel.cpp"));

    // Init wrapper (renames init() only for the plugin registration file)
    build.file(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("register.cpp"));

    println!("cargo:rerun-if-changed=register.cpp");
    build.compile("cardinal_plugin_as");
}
