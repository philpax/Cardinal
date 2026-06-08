{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "cardinal-rs";

  nativeBuildInputs = with pkgs; [
    # Rust toolchain provided by rustup on the host

    # C/C++ compiler (needed by cc crate)
    gcc
    pkg-config
    cmake

    # Debugging
    gdb
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
    libX11
    libXcursor
    libXrandr
    libXi
    libXinerama

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
    pkgs.libX11
    pkgs.libXcursor
    pkgs.libXrandr
    pkgs.libXi
    pkgs.alsa-lib
  ];
}
