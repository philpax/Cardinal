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

    # OpenGL + EGL + GLEW (for NanoVG rendering)
    libGL
    libGLU
    glew
    libglvnd.dev  # provides EGL headers

    # X11 (for egui/winit x11 backend)
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

  # Ensure the linker and runtime can find libraries.
  # /run/opengl-driver/lib contains the system GPU driver (e.g. NVIDIA's
  # libEGL_nvidia.so). Without it, EGL falls back to Mesa software rendering.
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
    pkgs.libGL
    pkgs.glew
    pkgs.wayland
    pkgs.libxkbcommon
    pkgs.xorg.libX11
    pkgs.xorg.libXcursor
    pkgs.xorg.libXrandr
    pkgs.xorg.libXi
    pkgs.alsa-lib
  ] + ":/run/opengl-driver/lib";

  # Tell libglvnd to use the system GPU driver for EGL.
  # On NixOS, libglvnd needs explicit direction to find NVIDIA's EGL ICD.
  # We use __EGL_VENDOR_LIBRARY_FILENAMES to bypass directory scanning
  # and directly specify the NVIDIA ICD JSON file.
  shellHook = ''
    if [ -f /run/opengl-driver/share/glvnd/egl_vendor.d/10_nvidia.json ]; then
      export __EGL_VENDOR_LIBRARY_FILENAMES=/run/opengl-driver/share/glvnd/egl_vendor.d/10_nvidia.json
      export LD_LIBRARY_PATH="/run/opengl-driver/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
      echo "NixOS: NVIDIA EGL ICD configured"
    elif [ -d /run/opengl-driver/share/glvnd/egl_vendor.d ]; then
      export __EGL_VENDOR_LIBRARY_DIRS=/run/opengl-driver/share/glvnd/egl_vendor.d
      export LD_LIBRARY_PATH="/run/opengl-driver/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
      echo "NixOS: System GL driver configured"
    fi
  '';
}
