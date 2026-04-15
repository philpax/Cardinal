fn main() {
    // Allow multiple definitions of single-file header libraries
    // (stb_image, freeverb, etc.) that get compiled into multiple plugin crates
    println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition");
}
