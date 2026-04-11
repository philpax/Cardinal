{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "cardinal-rs";

  nativeBuildInputs = with pkgs; [
    # Rust toolchain
    rustc
    cargo

    # C/C++ compiler (needed by cc crate)
    gcc
    pkg-config
    cmake
  ];

  buildInputs = with pkgs; [
    # Libraries linked by cardinal-core's build.rs
    jansson
    libarchive
    libsamplerate
    speexdsp

    # OpenGL + EGL (for offscreen rendering + egui glow backend)
    libGL
    libGLU
    libglvnd.dev  # provides EGL headers

    # X11 (for egui/winit x11 backend)
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    xorg.libXinerama

    # Wayland (alternative backend)
    wayland
    libxkbcommon
  ];

  # Ensure the linker can find libraries
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
    pkgs.libGL
    pkgs.wayland
    pkgs.libxkbcommon
    pkgs.xorg.libX11
    pkgs.xorg.libXcursor
    pkgs.xorg.libXrandr
    pkgs.xorg.libXi
  ];
}
