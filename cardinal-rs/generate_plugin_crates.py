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

    # Map CUSTOM names to plugin dirs
    # The naming convention is: UPPERCASE version of dir name
    for vendor, info in plugins.items():
        upper = vendor.upper().replace("-", "_").replace(" ", "_")

        # Try various name patterns
        for key in [upper, vendor.upper(), vendor.replace("-", "").upper()]:
            if key in custom_map:
                info["custom_renames"] = custom_map[key]
                break

        for key in [upper, vendor.upper(), vendor.replace("-", "").upper()]:
            if key in custom_per_file_map:
                info["custom_per_file"] = custom_per_file_map[key]
                break

        # pluginInstance rename
        # Convention: -DpluginInstance=pluginInstance__VendorName
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
    # custom_module_names = -D${1}=${2}${1} -Dmodel${1}=model${2}${1} -D${1}Widget=${2}${1}Widget
    defines_code = ""
    if pi_rename:
        defines_code += f'    build.define("pluginInstance", "pluginInstance__{pi_rename}");\n'
        defines_code += f'    build.define("init", "init__{pi_rename}");\n'

    for sym in custom_renames:
        # Use pi_rename as prefix (matches the Makefile's convention)
        # Falls back to vendor name if no pi_rename
        prefix = pi_rename if pi_rename else vendor
        defines_code += f'    build.define("{sym}", "{prefix}{sym}");\n'
        defines_code += f'    build.define("model{sym}", "model{prefix}{sym}");\n'
        defines_code += f'    build.define("{sym}Widget", "{prefix}{sym}Widget");\n'

    # Source collection code
    source_code_parts = []

    for sd in source_dirs:
        ext = "cpp"
        if sd.endswith(".cc") or "/eurorack/" in sd:
            ext = "cc"
        source_code_parts.append(f"""
    // Glob {sd}/*.{ext}
    if let Ok(entries) = std::fs::read_dir(plugins_dir.join("{sd}")) {{
        for entry in entries.flatten() {{
            let path = entry.path();
            if path.extension().map_or(true, |e| e != "cpp" && e != "cc" && e != "c") {{ continue; }}
            let rel = path.strip_prefix(&plugins_dir).unwrap().to_str().unwrap().to_string();
            if !filter_out.contains(&rel) {{
                build.file(&path);
            }}
        }}
    }}""")

    for ef in explicit_files:
        source_code_parts.append(f'    build.file(plugins_dir.join("{ef}"));')

    filter_out_code = "\n".join(f'        "{f}".to_string(),' for f in filter_out)

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
    let filter_out: Vec<String> = vec![
{filter_out_code}
    ];

    // Source files
{chr(10).join(source_code_parts)}

    build.compile("{crate_name.replace('-', '_')}");
}}
"""

    (crate_dir / "build.rs").write_text(build_rs)
    return crate_name


def generate_registry_crate(vendor_crates, plugins_info):
    """Generate the registry crate that initializes all plugins."""
    crate_dir = CRATES_DIR / "cardinal-plugins-registry"
    crate_dir.mkdir(parents=True, exist_ok=True)

    # Cargo.toml — depends on all vendor crates
    deps = "\n".join(f'{name} = {{ path = "../{name}" }}' for name in sorted(vendor_crates))

    (crate_dir / "Cargo.toml").write_text(f"""[package]
name = "cardinal-plugins-registry"
version = "0.1.0"
edition = "2024"

[dependencies]
{deps}

[build-dependencies]
cc = "1"
""")

    # src/lib.rs
    # This just needs to reference the vendor crates so they get linked
    extern_crates = "\n".join(f"extern crate {name.replace('-', '_')};" for name in sorted(vendor_crates))
    src_dir = crate_dir / "src"
    src_dir.mkdir(parents=True, exist_ok=True)
    (src_dir / "lib.rs").write_text(f"""// Auto-generated: references all plugin vendor crates to ensure they're linked.
{extern_crates}
""")

    # Generate plugin_init.cpp — calls initStaticPlugins() for each vendor
    cpp_dir = crate_dir / "cpp"
    cpp_dir.mkdir(exist_ok=True)

    # Collect vendor info for the init code
    init_calls = []
    for vendor, info in sorted(plugins_info.items()):
        if vendor in SKIP_PLUGINS:
            continue
        if not (PLUGINS_DIR / vendor).exists():
            continue
        pi = info.get("pi_rename")
        if pi:
            init_calls.append((vendor, pi))

    # Generate the extern declarations and init calls
    extern_decls = []
    init_stmts = []
    for vendor, pi_name in init_calls:
        extern_decls.append(f'extern "C++" void init__{pi_name}(rack::plugin::Plugin*);')
        init_stmts.append(f"""    {{
        Plugin* const p = new Plugin;
        pluginInstance__{pi_name} = p;
        const StaticPluginLoader spl(p, "{vendor}");
        if (spl.ok()) init__{pi_name}(p);
    }}""")

    extern_pi_decls = "\n".join(
        f"extern rack::plugin::Plugin* pluginInstance__{pi};"
        for _, pi in init_calls
    )
    extern_init_decls = "\n".join(extern_decls)
    init_body = "\n".join(init_stmts)

    plugin_init_cpp = f"""// Auto-generated by generate_plugin_crates.py
// Registers all compiled plugin vendors with the Rack engine.

#include <rack.hpp>
#include <plugin.hpp>

using namespace rack;
using namespace rack::plugin;

// Asset path helpers (match Cardinal's custom/asset.cpp)
namespace rack {{
namespace asset {{
std::string pluginManifest(const std::string& dirname) {{
    return rack::system::join(systemDir, "..", "..", "plugins", dirname, "plugin.json");
}}
std::string pluginPath(const std::string& dirname) {{
    return rack::system::join(systemDir, "..", "..", "plugins", dirname);
}}
}}
}}

// StaticPluginLoader (from Cardinal's plugins.cpp)
struct StaticPluginLoader {{
    Plugin* const plugin;
    FILE* file;
    json_t* rootJ;

    StaticPluginLoader(Plugin* const p, const char* const name)
        : plugin(p), file(nullptr), rootJ(nullptr)
    {{
        p->path = asset::pluginPath(name);
        const std::string manifestFilename = asset::pluginManifest(name);
        if ((file = std::fopen(manifestFilename.c_str(), "r")) == nullptr) {{
            fprintf(stderr, "cardinal: manifest %s not found\\n", manifestFilename.c_str());
            return;
        }}
        json_error_t error;
        if ((rootJ = json_loadf(file, 0, &error)) == nullptr) {{
            fprintf(stderr, "cardinal: JSON error at %s %d:%d %s\\n",
                    manifestFilename.c_str(), error.line, error.column, error.text);
            return;
        }}
        json_t* const versionJ = json_string((APP_VERSION_MAJOR + ".0").c_str());
        json_object_set(rootJ, "version", versionJ);
        json_decref(versionJ);
        p->fromJson(rootJ);
    }}

    ~StaticPluginLoader() {{
        if (rootJ != nullptr) {{
            json_t* const modulesJ = json_object_get(rootJ, "modules");
            plugin->modulesFromJson(modulesJ);
            json_decref(rootJ);
            plugins.push_back(plugin);
        }}
        if (file != nullptr)
            std::fclose(file);
    }}

    bool ok() const noexcept {{ return rootJ != nullptr; }}
}};

// Extern declarations for each vendor's init function and pluginInstance
{extern_pi_decls}

{extern_init_decls}

// Master init function called by the bridge
namespace rack {{
namespace plugin {{

void initStaticPlugins() {{
{init_body}
}}

}}
}}
"""
    (cpp_dir / "plugin_init.cpp").write_text(plugin_init_cpp)

    # Generate build.rs for the registry crate
    (crate_dir / "build.rs").write_text("""use std::path::PathBuf;

fn main() {
    let cardinal_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()  // crates/
        .parent().unwrap()  // cardinal-rs/
        .parent().unwrap()  // Cardinal/
        .to_path_buf();

    let rack_dir = cardinal_root.join("src/Rack");
    let cpp_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cpp");

    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").warnings(false)
        .define("ARCH_X64", None)
        .define("ARCH_LIN", None)
        .define("BUILDING_PLUGIN_MODULES", None);

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

    build.file(cpp_dir.join("plugin_init.cpp"));
    build.compile("cardinal_plugins_registry");

    println!("cargo:rerun-if-changed=cpp/plugin_init.cpp");
}
""")


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
