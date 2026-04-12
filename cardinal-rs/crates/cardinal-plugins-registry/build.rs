use std::path::PathBuf;

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
    build.cargo_metadata(false);
    build.compile("cardinal_plugins_registry");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static:+whole-archive=cardinal_plugins_registry");
    println!("cargo:rerun-if-changed=cpp/plugin_init.cpp");
}
