{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "cardinal-rs";

  nativeBuildInputs = with pkgs; [
    # Rust toolchain provided by rustup on the host

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

    # Vulkan (for wgpu backend)
    vulkan-loader
    vulkan-headers

    # GL headers (Rack C++ code includes GL/gl.h via stubs)
    libGL

    # X11 (for winit x11 backend)
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    xorg.libXinerama

    # Audio (cpal backend)
    alsa-lib

    # Wayland (alternative backend)
    wayland
    libxkbcommon
  ];

  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
    pkgs.vulkan-loader
    pkgs.wayland
    pkgs.libxkbcommon
    pkgs.xorg.libX11
    pkgs.xorg.libXcursor
    pkgs.xorg.libXrandr
    pkgs.xorg.libXi
    pkgs.alsa-lib
  ];
}
