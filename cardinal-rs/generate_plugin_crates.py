#!/usr/bin/env python3
"""
Generate per-vendor plugin crates for cardinal-rs from plugins/Makefile.

Parses the Makefile to extract:
- Source file patterns per plugin
- Filter-out exclusions
- CUSTOM symbol renames
- pluginInstance renames
- Extra include paths

Then generates:
- crates/cardinal-plugin-{vendor}/Cargo.toml
- crates/cardinal-plugin-{vendor}/build.rs
- crates/cardinal-plugin-{vendor}/src/lib.rs
- crates/cardinal-plugins-registry/ (ties them all together)
"""

import os
import re
import sys
from pathlib import Path
from collections import defaultdict

CARDINAL_ROOT = Path(__file__).parent.parent
PLUGINS_DIR = CARDINAL_ROOT / "plugins"
MAKEFILE = PLUGINS_DIR / "Makefile"
CRATES_DIR = CARDINAL_ROOT / "cardinal-rs" / "crates"

# Plugins that need external deps we can't provide
SKIP_PLUGINS = {
    "Cardinal",              # RTNeural, Carla, JUCE
    "HetrickCV",             # Gamma DSP library
    "surgext",               # Surge synthesizer engine
    "StarlingVia",           # Custom starling submodule
    "ParableInstruments",    # Mutable Instruments eurorack DSP
    "AudibleInstruments",    # Needs eurorack DSP build
    "ArableInstruments",     # Needs eurorack DSP from AudibleInstruments
    "voxglitch",             # Custom vgLib
    "mscHack",               # Missing source files
    "BaconPlugs",            # sst/filters dependency
    "AriaModules",           # QuickJS dependency
    "BogaudioModules-helper", # Needs BogaudioModules cross-crate includes
    "surgext-helper",        # Needs surgext cross-crate includes
    "DHE-Modules",           # Non-standard source layout
    "ValleyAudio",           # SIMDE SSE3 redefinition conflict
    "alefsbits",             # GCC 14 incompatible (begin/end on C arrays)
    "MindMeldModular",       # Cross-dep on ImpromptuModular theme code
    "rcm-modules",           # Needs gverb from Bidoo dep
    "stoermelder-packone",   # Needs custom plugin init handling
    "MockbaModular",         # loadBack utility missing
    "BidooDark",             # Dark theme helper, no plugin init
    "ImpromptuModularDark",  # Dark theme helper, no plugin init
}

def parse_drwav_list(makefile_text):
    """Extract the DRWAV symbol list."""
    symbols = []
    for line in makefile_text.splitlines():
        line = line.strip()
        if line.startswith("DRWAV ") or line.startswith("DRWAV\t"):
            for part in line.split("=", 1)[1].strip().split():
                if not part.startswith("$"):
                    symbols.append(part)
        elif line.startswith("DRWAV +=") or line.startswith("DRWAV\t+="):
            for part in line.split("+=", 1)[1].strip().split():
                if not part.startswith("$"):
                    symbols.append(part)
    return symbols

def parse_makefile():
    """Parse plugins/Makefile into per-plugin build rules."""
    text = MAKEFILE.read_text()
    drwav_symbols = parse_drwav_list(text)

    plugins = {}  # vendor_name -> PluginInfo

    # Parse PLUGIN_FILES lines
    for line in text.splitlines():
        line = line.strip()
        if not line.startswith("PLUGIN_FILES +="):
            continue
        rhs = line.split("+=", 1)[1].strip()

        # Extract vendor name from the path
        if "wildcard" in rhs:
            # $(wildcard Dir/src/*.cpp) or $(filter-out ...,$(wildcard ...))
            m = re.search(r'wildcard\s+(\S+?)/', rhs)
            if not m:
                continue
            vendor = m.group(1)
        elif rhs.endswith(".cpp") or rhs.endswith(".c") or rhs.endswith(".cc"):
            vendor = rhs.split("/")[0]
            if vendor.startswith("$"):
                continue
        else:
            continue

        if vendor not in plugins:
            plugins[vendor] = {
                "source_dirs": set(),
                "explicit_files": [],
                "filter_out": [],
                "custom_renames": [],
                "custom_per_file": [],
                "pi_rename": None,
                "extra_flags": [],
            }

        p = plugins[vendor]

        if "wildcard" in rhs:
            # Extract dir from wildcard pattern
            m2 = re.search(r'wildcard\s+(\S+/)\*\.(cpp|c|cc)', rhs)
            if m2:
                p["source_dirs"].add(m2.group(1).rstrip("/"))

            # Extract filter-out files
            if "filter-out" in rhs:
                m3 = re.search(r'filter-out\s+([^,]+),', rhs)
                if m3:
                    for f in m3.group(1).strip().split():
                        if not f.startswith("-"):
                            p["filter_out"].append(f)
        elif rhs.endswith(".cpp") or rhs.endswith(".c") or rhs.endswith(".cc"):
            p["explicit_files"].append(rhs)

    # Parse CUSTOM renames
    custom_map = {}  # UPPERCASE_NAME -> [symbols]
    for line in text.splitlines():
        line = line.strip()
        m = re.match(r'^([A-Z_]+)_CUSTOM\s*\+?=\s*(.*)', line)
        if m:
            key = m.group(1)
            vals = m.group(2).strip().split()
            if key not in custom_map:
                custom_map[key] = []
            for v in vals:
                if v == "$(DRWAV)":
                    custom_map[key].extend(drwav_symbols)
                else:
                    custom_map[key].append(v)

    # Parse CUSTOM_PER_FILE renames
    custom_per_file_map = {}
    for line in text.splitlines():
        line = line.strip()
        m = re.match(r'^([A-Z_]+)_CUSTOM_PER_FILE\s*\+?=\s*(.*)', line)
        if m:
            key = m.group(1)
            vals = m.group(2).strip().split()
            if key not in custom_per_file_map:
                custom_per_file_map[key] = []
            custom_per_file_map[key].extend(vals)

    # Parse pluginInstance renames
    pi_renames = {}  # dir_name -> rename_suffix
    for line in text.splitlines():
        m = re.search(r'-DpluginInstance=pluginInstance__(\w+)', line)
        if m:
            suffix = m.group(1)
            # Try to figure out which plugin dir this belongs to
            # by looking at the build rule context
            pi_renames[suffix] = suffix

    # Explicit directory→pi_rename mappings for cases where normalization fails
    EXPLICIT_PI_RENAMES = {
        "stoermelder-packone": "stoermelder_p1",
        "forsitan-modulare": "forsitan",
        "ML_modules": "ML",
        "rcm-modules": "RCM",
        "h4n4-modules": "H4N4",
        "myth-modules": "myth_modules",
        "ExpertSleepers-Encoders": "ExpertSleepersEncoders",
        "MindMeldModular": "MindMeld",
        "BogaudioModules": "BogaudioModules",
        "LyraeModules": "Lyrae",
        "LomasModules": "Lomas",
        "MUS-X": "MUS_X",
        "JW-Modules": "JW",
        "GlueTheGiant": "GlueTheGiant",
        "WSTD-Drums": "WSTD_Drums",
        "ImpromptuModular": "ImpromptuModular",
        "BaconPlugs": "Bacon",
        "WhatTheRack": "WhatTheRack",
        "PinkTrombone": "PinkTrombone",
        "unless_modules": "unless_modules",
    }

    # Map CUSTOM names to plugin dirs
    for vendor, info in plugins.items():
        upper = vendor.upper().replace("-", "_").replace(" ", "_")

        for key in [upper, vendor.upper(), vendor.replace("-", "").upper()]:
            if key in custom_map:
                info["custom_renames"] = custom_map[key]
                break

        for key in [upper, vendor.upper(), vendor.replace("-", "").upper()]:
            if key in custom_per_file_map:
                info["custom_per_file"] = custom_per_file_map[key]
                break

        # pluginInstance rename — use explicit map first, then fuzzy match
        if vendor in EXPLICIT_PI_RENAMES:
            info["pi_rename"] = EXPLICIT_PI_RENAMES[vendor]
        else:
            for suffix in pi_renames:
                if suffix.lower().replace("_", "") == vendor.lower().replace("-", "").replace("_", ""):
                    info["pi_rename"] = suffix
                    break

    return plugins, drwav_symbols


def vendor_crate_name(vendor):
    """Convert vendor name to a valid Rust crate name."""
    name = vendor.lower().replace("_", "-").replace(" ", "-")
    # Crate names can't start with a digit
    if name[0].isdigit():
        name = "p" + name
    return f"cardinal-plugin-{name}"


def generate_vendor_crate(vendor, info, drwav_symbols):
    """Generate a crate for a single plugin vendor."""
    crate_name = vendor_crate_name(vendor)
    crate_dir = CRATES_DIR / crate_name
    crate_dir.mkdir(parents=True, exist_ok=True)

    # Cargo.toml
    (crate_dir / "Cargo.toml").write_text(f"""[package]
name = "{crate_name}"
version = "0.1.0"
edition = "2024"

[build-dependencies]
cc = "1"
""")

    # src/lib.rs (empty — this is a -sys style crate)
    src_dir = crate_dir / "src"
    src_dir.mkdir(exist_ok=True)
    (src_dir / "lib.rs").write_text(f"// Plugin vendor: {vendor}\n// This crate only provides compiled C++ objects.\n")

    # build.rs
    source_dirs = sorted(info["source_dirs"])
    explicit_files = info["explicit_files"]
    filter_out = info["filter_out"]
    custom_renames = info["custom_renames"]
    custom_per_file = info["custom_per_file"]
    pi_rename = info["pi_rename"]

    # Generate the defines for custom_module_names
    defines_code = ""

    if pi_rename:
        defines_code += f'    build.define("pluginInstance", "pluginInstance__{pi_rename}");\n'
        # NOTE: we do NOT globally rename init() — that would break
        # rack::random::init() and other Rack API calls. Instead, we
        # generate a wrapper file that renames init only for the plugin's
        # registration file (see init_wrapper below).

    for sym in custom_renames:
        # Use pi_rename as prefix (matches the Makefile's convention)
        # Falls back to vendor name if no pi_rename
        prefix = pi_rename if pi_rename else vendor
        defines_code += f'    build.define("{sym}", "{prefix}{sym}");\n'
        defines_code += f'    build.define("model{sym}", "model{prefix}{sym}");\n'
        defines_code += f'    build.define("{sym}Widget", "{prefix}{sym}Widget");\n'

    # Source collection — define helper once, call per directory
    source_code_parts = []
    source_code_parts.append("""
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
    }""")

    for sd in source_dirs:
        source_code_parts.append(f'    collect_sources(&plugins_dir.join("{sd}"), &_filter_out, &plugins_dir, &mut build, 0);')

    for ef in explicit_files:
        source_code_parts.append(f'    build.file(plugins_dir.join("{ef}"));')

    # Find the init file — the .cpp that defines `void init(Plugin`
    # First check filter-outs for plugin.cpp / vendor header
    init_files = []
    kept_filter_out = []
    for f in filter_out:
        basename = f.rsplit("/", 1)[-1] if "/" in f else f
        is_init_file = False
        if basename == "plugin.cpp":
            is_init_file = True
        elif not basename.endswith(".hpp"):
            stem = basename.replace(".cpp", "")
            vendor_lower = vendor.lower().replace("-", "").replace("_", "")
            if stem.lower().replace("-", "").replace("_", "") == vendor_lower:
                is_init_file = True
        if is_init_file:
            init_files.append(f)
        else:
            kept_filter_out.append(f)

    # If no init file found in filter-outs, scan source directory
    if not init_files:
        plugin_src = PLUGINS_DIR / vendor / "src"
        if plugin_src.exists():
            for cpp in sorted(plugin_src.iterdir()):
                if cpp.suffix != ".cpp":
                    continue
                try:
                    content = cpp.read_text(errors="ignore")
                    if ("void init(Plugin" in content or
                        "void init(rack::plugin::Plugin" in content or
                        "void init(rack::Plugin" in content):
                        rel = f"{vendor}/src/{cpp.name}"
                        init_files.append(rel)
                        break
                except:
                    pass

    # In per-vendor crates, most Makefile filter-outs are unnecessary
    # (they were for the monolithic plugins.cpp approach). Only keep
    # filter-outs for files known to have external dep issues.
    EXTERNAL_DEP_FILES = {
        "Bidoo/src/ANTN.cpp",         # needs curl
        "Befaco/src/MidiThing.cpp",   # needs MIDI hardware
    }
    kept_filter_out = [f for f in kept_filter_out if f in EXTERNAL_DEP_FILES]

    # Generate registration function — each vendor exports a C function
    # that the Rust registry calls. This avoids C++ cross-archive link
    # ordering issues by routing through Rust's .rlib resolution.
    init_wrapper_code = ""
    safe_name = pi_rename if pi_rename else vendor.replace("-", "_")
    if not pi_rename:
        pi_rename = safe_name
        info["pi_rename"] = safe_name

    if init_files:
        register_path = crate_dir / "register.cpp"
        init_file = init_files[0]
        register_content = f'''// Auto-generated — registration function for {vendor}
// Renames init() only in the included file, not globally
#define init init__{safe_name}
#include "{init_file}"
#undef init

#include <rack.hpp>
#include <plugin.hpp>

// Asset helpers (same as plugin_init.cpp)
namespace rack {{ namespace asset {{
extern std::string pluginManifest(const std::string& dirname);
extern std::string pluginPath(const std::string& dirname);
}} }}

struct StaticPluginLoader {{
    rack::plugin::Plugin* const plugin;
    FILE* file;
    json_t* rootJ;
    StaticPluginLoader(rack::plugin::Plugin* const p, const char* const name)
        : plugin(p), file(nullptr), rootJ(nullptr)
    {{
        p->path = rack::asset::pluginPath(name);
        const std::string mf = rack::asset::pluginManifest(name);
        if ((file = std::fopen(mf.c_str(), "r")) == nullptr) return;
        json_error_t error;
        if ((rootJ = json_loadf(file, 0, &error)) == nullptr) return;
        json_t* const vJ = json_string((rack::APP_VERSION_MAJOR + ".0").c_str());
        json_object_set(rootJ, "version", vJ);
        json_decref(vJ);
        p->fromJson(rootJ);
    }}
    ~StaticPluginLoader() {{
        if (rootJ) {{
            json_t* const mJ = json_object_get(rootJ, "modules");
            plugin->modulesFromJson(mJ);
            json_decref(rootJ);
            rack::plugin::plugins.push_back(plugin);
        }}
        if (file) std::fclose(file);
    }}
    bool ok() const noexcept {{ return rootJ != nullptr; }}
}};

extern "C" void cardinal_register_{safe_name}() {{
    using namespace rack;
    using namespace rack::plugin;
    Plugin* const p = new Plugin;
    pluginInstance__{safe_name} = p;
    const StaticPluginLoader spl(p, "{vendor}");
    if (spl.ok()) init__{safe_name}(p);
}}
'''
        register_path.write_text(register_content)
        init_wrapper_code = f'    build.file(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("register.cpp"));\n'
        kept_filter_out.extend(init_files)
    else:
        # No init file found — vendor has no registration
        pass

    filter_out_code = "\n".join(f'        "{f}".to_string(),' for f in kept_filter_out)

    build_rs = f"""use std::path::PathBuf;

fn main() {{
    let cardinal_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()  // crates/
        .parent().unwrap()  // cardinal-rs/
        .parent().unwrap()  // Cardinal/
        .to_path_buf();

    let plugins_dir = cardinal_root.join("plugins");
    let plugin_dir = plugins_dir.join("{vendor}");

    if !plugin_dir.exists() {{
        eprintln!("Plugin {vendor} not found (submodule not initialized?), skipping");
        return;
    }}

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
    ] {{
        build.include(dir);
    }}

    // Plugin-local includes — recursively add src/ and dep subdirs
    fn add_dirs(build: &mut cc::Build, dir: &std::path::Path, depth: u32) {{
        if depth > 4 || !dir.exists() {{ return; }}
        build.include(dir);
        if let Ok(entries) = std::fs::read_dir(dir) {{
            for e in entries.flatten() {{
                if e.path().is_dir() {{
                    add_dirs(build, &e.path(), depth + 1);
                }}
            }}
        }}
    }}
    add_dirs(&mut build, &plugin_dir.join("src"), 0);
    for dep_dir in ["dep", "deps", "lib"] {{
        add_dirs(&mut build, &plugin_dir.join(dep_dir), 0);
    }}

    // Symbol renames to avoid cross-plugin collisions
{defines_code}
    // Filter-out list
    let _filter_out: Vec<String> = vec![
{filter_out_code}
    ];

    // Source files
{chr(10).join(source_code_parts)}

    // Init wrapper (renames init() only for the plugin registration file)
{init_wrapper_code}
    build.compile("{crate_name.replace('-', '_')}");
}}
"""

    (crate_dir / "build.rs").write_text(build_rs)
    return crate_name


def generate_registry_crate(vendor_crates, plugins_info):
    """Generate the registry crate that initializes all plugins."""
    crate_dir = CRATES_DIR / "cardinal-plugins-registry"
    crate_dir.mkdir(parents=True, exist_ok=True)

    # Cargo.toml — depends on all vendor crates (no build-dependencies needed)
    deps = "\n".join(f'{name} = {{ path = "../{name}" }}' for name in sorted(vendor_crates))

    (crate_dir / "Cargo.toml").write_text(f"""[package]
name = "cardinal-plugins-registry"
version = "0.1.0"
edition = "2024"

[dependencies]
{deps}
""")

    # Collect vendor info for registration
    init_calls = []
    for vendor, info in sorted(plugins_info.items()):
        if vendor in SKIP_PLUGINS:
            continue
        if not (PLUGINS_DIR / vendor).exists():
            continue
        pi = info.get("pi_rename")
        if pi:
            init_calls.append((vendor, pi))

    # src/lib.rs — declares extern "C" registration functions and calls them
    extern_crates = "\n".join(
        f"extern crate {name.replace('-', '_')};"
        for name in sorted(vendor_crates)
    )

    extern_fns = "\n".join(
        f"    fn cardinal_register_{pi}();"
        for _, pi in init_calls
    )

    register_calls = "\n".join(
        f"        cardinal_register_{pi}();"
        for _, pi in init_calls
    )

    src_dir = crate_dir / "src"
    src_dir.mkdir(parents=True, exist_ok=True)
    (src_dir / "lib.rs").write_text(f"""// Auto-generated: plugin registration via Rust FFI.
// Each vendor crate exports a C function cardinal_register_<name>().
// Calling them from Rust ensures the linker resolves cross-archive refs.

// Reference vendor crates so their native libs get linked
{extern_crates}

unsafe extern "C" {{
{extern_fns}
}}

/// Register all compiled plugin vendors with the Rack engine.
/// Called from cardinal_core::init().
pub fn register_all_plugins() {{
    unsafe {{
{register_calls}
    }}
}}
""")

    # No build.rs needed — no C++ to compile in the registry anymore
    # Remove old build artifacts
    for old_file in [crate_dir / "build.rs", crate_dir / "cpp" / "plugin_init.cpp"]:
        if old_file.exists():
            old_file.unlink()


def main():
    plugins, drwav_symbols = parse_makefile()

    print(f"Parsed {len(plugins)} plugins from Makefile")
    print(f"DRWAV has {len(drwav_symbols)} symbols")
    print(f"Skipping: {', '.join(sorted(SKIP_PLUGINS))}")

    vendor_crates = []

    for vendor, info in sorted(plugins.items()):
        if vendor in SKIP_PLUGINS:
            print(f"  SKIP {vendor}")
            continue

        if not (PLUGINS_DIR / vendor).exists():
            print(f"  MISS {vendor} (submodule not initialized)")
            continue

        crate_name = generate_vendor_crate(vendor, info, drwav_symbols)
        vendor_crates.append(crate_name)
        n_dirs = len(info["source_dirs"])
        n_files = len(info["explicit_files"])
        n_renames = len(info["custom_renames"])
        print(f"  OK   {vendor} -> {crate_name} ({n_dirs} dirs, {n_files} files, {n_renames} renames)")

    generate_registry_crate(vendor_crates, plugins)
    print(f"\nGenerated {len(vendor_crates)} vendor crates + registry")

    # Print workspace members to add
    print("\n# Add to cardinal-rs/Cargo.toml [workspace] members:")
    for name in sorted(vendor_crates):
        print(f'    "crates/{name}",')
    print(f'    "crates/cardinal-plugins-registry",')


if __name__ == "__main__":
    main()
