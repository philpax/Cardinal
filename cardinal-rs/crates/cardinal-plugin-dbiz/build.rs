use std::path::PathBuf;

fn main() {
    let cardinal_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()  // crates/
        .parent().unwrap()  // cardinal-rs/
        .parent().unwrap()  // Cardinal/
        .to_path_buf();

    let plugins_dir = cardinal_root.join("plugins");
    let plugin_dir = plugins_dir.join("dBiz");

    if !plugin_dir.exists() {
        eprintln!("Plugin dBiz not found (submodule not initialized?), skipping");
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
    build.define("pluginInstance", "pluginInstance__dBiz");
    build.define("Chord", "dBizChord");
    build.define("modelChord", "modeldBizChord");
    build.define("ChordWidget", "dBizChordWidget");
    build.define("DarkDefaultItem", "dBizDarkDefaultItem");
    build.define("modelDarkDefaultItem", "modeldBizDarkDefaultItem");
    build.define("DarkDefaultItemWidget", "dBizDarkDefaultItemWidget");
    build.define("Divider", "dBizDivider");
    build.define("modelDivider", "modeldBizDivider");
    build.define("DividerWidget", "dBizDividerWidget");
    build.define("FourSeq", "dBizFourSeq");
    build.define("modelFourSeq", "modeldBizFourSeq");
    build.define("FourSeqWidget", "dBizFourSeqWidget");
    build.define("FourSeqWidget", "dBizFourSeqWidget");
    build.define("modelFourSeqWidget", "modeldBizFourSeqWidget");
    build.define("FourSeqWidgetWidget", "dBizFourSeqWidgetWidget");
    build.define("OrangeLight", "dBizOrangeLight");
    build.define("modelOrangeLight", "modeldBizOrangeLight");
    build.define("OrangeLightWidget", "dBizOrangeLightWidget");
    build.define("VCA4", "dBizVCA4");
    build.define("modelVCA4", "modeldBizVCA4");
    build.define("VCA4Widget", "dBizVCA4Widget");
    build.define("Wavefolder", "dBizWavefolder");
    build.define("modelWavefolder", "modeldBizWavefolder");
    build.define("WavefolderWidget", "dBizWavefolderWidget");
    build.define("darkPanelID", "dBizdarkPanelID");
    build.define("modeldarkPanelID", "modeldBizdarkPanelID");
    build.define("darkPanelIDWidget", "dBizdarkPanelIDWidget");
    build.define("lightPanelID", "dBizlightPanelID");
    build.define("modellightPanelID", "modeldBizlightPanelID");
    build.define("lightPanelIDWidget", "dBizlightPanelIDWidget");

    // Filter-out list
    let filter_out: Vec<String> = vec![
        "dBiz/src/plugin.cpp".to_string(),
    ];

    // Source files

    // Glob dBiz/src/*.cpp
    if let Ok(entries) = std::fs::read_dir(plugins_dir.join("dBiz/src")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(true, |e| e != "cpp" && e != "cc" && e != "c") { continue; }
            let rel = path.strip_prefix(&plugins_dir).unwrap().to_str().unwrap().to_string();
            if !filter_out.contains(&rel) {
                build.file(&path);
            }
        }
    }

    build.compile("cardinal_plugin_dbiz");
}
