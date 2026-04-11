use std::collections::HashMap;
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
        cardinal_root.join("plugins"),
        cardinal_root.join("dpf/distrho"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cpp"),
    ];

    // ── 1. Build the Rack engine (PRIVATE="") ────────────────────────
    build_rack_engine(&rack_src, &rack_dep, &include_dirs);

    // ── 2. Build NanoVG GL2 + NanoSVG implementations ────────────────
    build_gl_impls(&include_dirs);

    // ── 3. Build all plugins ─────────────────────────────────────────
    build_all_plugins(&cardinal_root, &include_dirs);

    // ── 4. Build C deps ──────────────────────────────────────────────
    build_c_deps(&rack_dep, &include_dirs, &cardinal_root);

    // ── 5. Build bridge ──────────────────────────────────────────────
    build_bridge(&include_dirs);

    // ── System libraries ─────────────────────────────────────────────
    for lib in &["jansson", "archive", "samplerate", "pthread", "dl", "GL", "GLEW", "EGL"] {
        println!("cargo:rustc-link-lib={lib}");
    }

    // ── Rerun triggers ───────────────────────────────────────────────
    for f in &["bridge.h", "bridge.cpp", "stubs.cpp", "plugin_init.cpp"] {
        println!("cargo:rerun-if-changed=cpp/{f}");
    }
}

// ─────────────────────────────────────────────────────────────────────
// Rack engine (internal code, PRIVATE="")
// ─────────────────────────────────────────────────────────────────────

fn build_rack_engine(rack_src: &PathBuf, _rack_dep: &PathBuf, includes: &[PathBuf]) {
    let skip: &[&str] = &[
        "asset", "audio", "common", "dep", "discord", "gamepad", "keyboard",
        "library", "midi", "midiloopback", "network", "rtaudio", "rtmidi",
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
    build.compile("rack_engine");
}

// ─────────────────────────────────────────────────────────────────────
// NanoVG GL2 + NanoSVG (C++ for proper linkage)
// ─────────────────────────────────────────────────────────────────────

fn build_gl_impls(includes: &[PathBuf]) {
    let bridge_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cpp");
    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").warnings(false)
        .define("PRIVATE", "").define("ARCH_X64", None).define("ARCH_LIN", None);
    // System GLEW before Cardinal's stub
    build.include("/usr/include");
    for d in includes {
        if d.ends_with("include") && d.to_str().unwrap().contains("Cardinal/include") { continue; }
        build.include(d);
    }
    build.file(bridge_dir.join("nanovg_gl_impl.cpp"));
    build.file(bridge_dir.join("nanosvg_impl.cpp"));
    build.compile("rack_gl_impl");
}

// ─────────────────────────────────────────────────────────────────────
// All plugins — parsed from plugins/Makefile
// ─────────────────────────────────────────────────────────────────────

fn build_all_plugins(cardinal_root: &PathBuf, includes: &[PathBuf]) {
    let plugins_dir = cardinal_root.join("plugins");
    let makefile_path = plugins_dir.join("Makefile");
    let bridge_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cpp");

    // Parse the Makefile for plugin build rules
    let makefile = std::fs::read_to_string(&makefile_path)
        .expect("Failed to read plugins/Makefile");

    let rules = parse_makefile_plugin_rules(&makefile);

    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").warnings(false)
        .define("ARCH_X64", None).define("ARCH_LIN", None)
        .define("BUILDING_PLUGIN_MODULES", None);
    for d in includes { build.include(d); }

    // Plugins that need external deps we can't easily provide
    // Plugins that need external deps or custom build steps.
    // These are skipped; their modules won't be available but plugins.cpp
    // will still register what it can.
    let skip_plugins: &[&str] = &[
        "AriaModules",        // quickjs
        "AudibleInstruments", // Mutable Instruments eurorack DSP library
        "Cardinal",           // RTNeural, Carla, JUCE
        "HetrickCV",          // Gamma DSP library
        "mscHack",            // incomplete submodule
        "ParableInstruments", // Mutable Instruments eurorack DSP
        "StarlingVia",        // custom starling submodule
        "surgext",            // Surge synthesizer engine
        "voxglitch",          // custom vgLib
    ];

    // Compile plugins.cpp (the master registration file)
    build.file(plugins_dir.join("plugins.cpp"));

    // Extra per-plugin include paths
    let extra_includes: &[(&str, &[&str])] = &[
        ("DHE-Modules", &["DHE-Modules/src", "DHE-Modules/src/modules"]),
        ("StarlingVia", &["StarlingVia/src/starling"]),
        ("ParableInstruments", &["AudibleInstruments/eurorack"]),
        ("AriaModules", &["AriaModules/src"]),
        ("BogaudioModules", &["BogaudioModules/lib", "BogaudioModules/src/dsp"]),
        ("mscHack", &["mscHack/src"]),
    ];

    for (_, dirs) in extra_includes {
        for d in *dirs {
            let p = plugins_dir.join(d);
            if p.exists() { build.include(p); }
        }
    }

    // Auto-detect include paths: for each plugin, add src/ and dep subdirs
    for rule in &rules {
        let plugin_dir = plugins_dir.join(&rule.dir_name);
        if !plugin_dir.exists() || skip_plugins.contains(&rule.dir_name.as_str()) { continue; }
        // Add plugin's src dir and any dep/include dirs
        let src_dir = plugin_dir.join("src");
        if src_dir.exists() { build.include(&src_dir); }
        for dep_dir in ["src/dep", "src/deps", "dep", "deps"] {
            let dd = plugin_dir.join(dep_dir);
            if dd.exists() {
                build.include(&dd);
                // Also add immediate subdirs (for plugins like Bidoo with dep/gverb/include)
                if let Ok(entries) = std::fs::read_dir(&dd) {
                    for e in entries.flatten() {
                        if e.path().is_dir() {
                            build.include(e.path());
                            let inc = e.path().join("include");
                            if inc.exists() { build.include(inc); }
                        }
                    }
                }
            }
        }
    }

    // For each plugin rule, add source files
    let mut total_files = 0;
    for rule in &rules {
        let plugin_dir = plugins_dir.join(&rule.dir_name);
        if !plugin_dir.exists() { continue; }
        if skip_plugins.contains(&rule.dir_name.as_str()) { continue; }

        for src_pattern in &rule.source_patterns {
            let src_dir = plugins_dir.join(src_pattern);
            if !src_dir.exists() || !src_dir.is_dir() { continue; }

            // Only list direct children (not recursive) unless pattern
            // contains a deeper path like dep/
            let entries: Vec<PathBuf> = if src_pattern.contains("/dep/") || src_pattern.contains("/deps/") {
                walkdir(src_dir.to_str().unwrap()).into_iter().map(PathBuf::from).collect()
            } else {
                std::fs::read_dir(&src_dir).ok()
                    .into_iter().flatten().flatten()
                    .map(|e| e.path())
                    .collect()
            };

            for path in entries {
                if path.extension().map_or(true, |e| e != "cpp") { continue; }

                // Check filter-outs
                let rel = path.strip_prefix(&plugins_dir)
                    .unwrap_or(&path)
                    .to_str().unwrap_or("");
                if rule.filter_out.iter().any(|f| rel.ends_with(f) || rel == *f) {
                    continue;
                }

                build.file(&path);
                total_files += 1;
            }
        }
    }

    // Plugin init file (our StaticPluginLoader replacement for the
    // few symbols not in plugins.cpp)
    build.file(bridge_dir.join("plugin_init.cpp"));

    // drwav implementation
    build.file(bridge_dir.join("drwav_impl.cpp"));

    eprintln!("Compiling {total_files} plugin source files...");
    build.compile("plugins_all");
}

/// Parsed build rule for a single plugin.
struct PluginRule {
    dir_name: String,
    source_patterns: Vec<String>,  // e.g. "Fundamental/src"
    filter_out: Vec<String>,       // files to skip (relative to plugins/)
}

/// Parse plugins/Makefile to extract PLUGIN_FILES lines.
fn parse_makefile_plugin_rules(makefile: &str) -> Vec<PluginRule> {
    let mut rules: HashMap<String, PluginRule> = HashMap::new();

    for line in makefile.lines() {
        let line = line.trim();
        if !line.starts_with("PLUGIN_FILES +=") { continue; }
        let rhs = line.strip_prefix("PLUGIN_FILES +=").unwrap().trim();

        // Pattern 1: $(wildcard Dir/src/*.cpp) or $(filter-out ...,$(wildcard Dir/src/*.cpp))
        if rhs.contains("wildcard") {
            // Extract the wildcard pattern
            let wildcard_pat = extract_between(rhs, "wildcard ", ")");
            if wildcard_pat.is_empty() { continue; }

            // Plugin dir name is the first component
            let dir_name = wildcard_pat.split('/').next().unwrap_or("").to_string();
            if dir_name.is_empty() { continue; }

            // Source directory (strip the *.cpp glob)
            let src_dir = wildcard_pat.rsplit_once('/').map(|(d,_)| d).unwrap_or(&wildcard_pat);

            let rule = rules.entry(dir_name.clone()).or_insert_with(|| PluginRule {
                dir_name: dir_name.clone(),
                source_patterns: Vec::new(),
                filter_out: Vec::new(),
            });

            if !rule.source_patterns.contains(&src_dir.to_string()) {
                rule.source_patterns.push(src_dir.to_string());
            }

            // Extract filter-out files
            if rhs.contains("filter-out") {
                let filtered = extract_between(rhs, "filter-out ", ",");
                for f in filtered.split_whitespace() {
                    rule.filter_out.push(f.to_string());
                }
            }
        }
        // Pattern 2: explicit file like "Cardinal/src/Blank.cpp"
        else if rhs.ends_with(".cpp") {
            let dir_name = rhs.split('/').next().unwrap_or("").to_string();
            if dir_name.is_empty() || dir_name.starts_with('$') { continue; }
            let src_dir = rhs.rsplit_once('/').map(|(d,_)| d).unwrap_or(rhs);
            let rule = rules.entry(dir_name.clone()).or_insert_with(|| PluginRule {
                dir_name: dir_name.clone(),
                source_patterns: Vec::new(),
                filter_out: Vec::new(),
            });
            // For explicit files, add the parent dir
            if !rule.source_patterns.contains(&src_dir.to_string()) {
                rule.source_patterns.push(src_dir.to_string());
            }
        }
    }

    rules.into_values().collect()
}

fn extract_between<'a>(s: &'a str, start: &str, end: &str) -> &'a str {
    if let Some(i) = s.find(start) {
        let rest = &s[i + start.len()..];
        if let Some(j) = rest.find(end) {
            return &rest[..j];
        }
    }
    ""
}

// ─────────────────────────────────────────────────────────────────────
// C dependencies
// ─────────────────────────────────────────────────────────────────────

fn build_c_deps(rack_dep: &PathBuf, includes: &[PathBuf], cardinal_root: &PathBuf) {
    let mut build = cc::Build::new();
    build.warnings(false);
    for d in includes { build.include(d); }
    build.include(cardinal_root.join("plugins/Fundamental/src"));

    build.file(rack_dep.join("nanovg/src/nanovg.c"));
    build.file(rack_dep.join("tinyexpr/tinyexpr.c"));
    build.file(rack_dep.join("pffft/pffft.c"));
    build.file(rack_dep.join("pffft/fftpack.c"));
    build.file(rack_dep.join("oui-blendish/blendish.c"));
    build.compile("rack_deps");
}

// ─────────────────────────────────────────────────────────────────────
// Bridge (our C++ glue code)
// ─────────────────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────
// Utilities
// ─────────────────────────────────────────────────────────────────────

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
