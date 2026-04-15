# cardinal-rs

Converts [Cardinal](https://github.com/DISTRHO/Cardinal), a free and
open-source virtual modular synthesizer based on
[VCV Rack](https://vcvrack.com/), into a reusable Rust library.
Cardinal-rs replaces the original OpenGL/GLFW rendering and windowing
stack with a [wgpu](https://wgpu.rs/)-based pipeline, exposing the
engine, module rendering, and audio processing through a safe Rust API
that any frontend can build on.

## How it works

Cardinal-rs compiles the real VCV Rack C++ engine and 66 plugin vendors
into static libraries, then drives them from Rust. The key pieces:

- **NanoVG wgpu backend** -- a pure-Rust implementation of the NanoVG
  rendering interface (`NVGparams` callbacks) targeting wgpu. Module
  widgets render through NanoVG as usual, but draw calls are flushed as
  wgpu render passes instead of OpenGL. Includes stencil-then-cover
  path fill, triangle fan/strip conversion, and FBO support via
  wgpu render targets.

- **Two NanoVG contexts** (`vg` + `fbVg`) share one `WgpuNvgContext`
  backend. The primary context draws to the screen; the secondary is
  used by `FramebufferWidget` for offscreen caching. Draw state
  (vertices, uniforms, draw calls) is saved/restored on FBO
  bind/unbind so nested rendering works correctly.

- **Zero-copy texture sharing** -- each module is rendered into its own
  wgpu texture, then registered directly with the egui renderer via
  `register_native_texture`. No GPU readback or CPU copies.

- **C++ bridge** (`bridge.cpp`) -- thin C API that creates modules,
  forwards UI events into the Rack widget tree, manages cables, and
  drives per-sample audio processing through a custom `TerminalModule`.
  Stub implementations of Rack globals (`APP->scene`, `APP->window`,
  `APP->event`, `APP->history`) satisfy widget code that assumes a
  full Rack application.

- **Audio** -- cpal opens the system audio device; the audio callback
  does per-sample terminal processing
  (`processTerminalInput` / `stepBlock(1)` / `processTerminalOutput`)
  since the standard Rack engine doesn't know about `TerminalModule`.

## Architecture

```
  cardinal-egui (winit + egui)
       |
       +--- cardinal-wrapper (engine driver thread, audio backend)
       |         |
       |         +--- cardinal-core (Rack engine, NanoVG wgpu, C++ bridge)
       |                   |
       |                   +--- cardinal-plugins-registry
       |                            |
       |                            +--- 66 cardinal-plugin-* crates
       |
       +--- egui / egui-wgpu / winit / wgpu / cpal
```

All Cardinal/Rack state lives on a dedicated thread. The UI
communicates with it via a `Command` channel (module creation,
rendering, event forwarding, cable management). Render results come
back as wgpu textures on a separate channel.

## Project structure

```
cardinal-rs/
  cardinal-egui/           Egui/winit/wgpu binary (the UI)
    src/
      main.rs              Entry point, wiring
      app.rs               App state, egui layout, input forwarding, painting
      wgpu_app.rs          GpuState, WgpuApp, winit ApplicationHandler
  crates/
    cardinal-core/         Rack engine compilation, NanoVG wgpu backend, C++ bridge
      cpp/
        bridge.cpp         C API: modules, events, cables, audio, incomplete cable
        bridge.h           Public C header
        stubs.cpp          Rack subsystem stubs (Scene, Window, GL, FBO, asset, etc.)
        nanosvg_impl.cpp   NanoSVG implementation unit
      src/
        lib.rs             Safe Rust wrappers over FFI
        ffi.rs             Raw extern declarations and NanoVG types
        nanovg_wgpu.rs     wgpu NanoVG backend (~2000 lines)
        nanovg_wgpu_shaders.wgsl  WGSL vertex/fragment shaders
      build.rs             Compiles Rack engine, C deps, and bridge
    cardinal-wrapper/      Reusable engine driver (no UI dependency)
      src/
        cardinal_thread.rs Command/EventResult types, spawn_cardinal_thread()
        audio.rs           cpal audio backend (stream setup + callback)
    plugins/
      cardinal-plugin-*/   66 plugin vendor crates (each compiles C++ via cc)
      cardinal-plugins-registry/  Links all plugins, calls registration functions
```

## Building

Requires a nix-shell (see `shell.nix` in the repo root) for system
dependencies (jansson, libarchive, libsamplerate, speexdsp, Vulkan,
X11/Wayland, ALSA). Rust toolchain is provided by the host via rustup.

```sh
nix-shell
cargo run -p cardinal-egui
```

## Current status

Working: module rendering, widget interaction (knobs, buttons, sliders),
cable creation via port drag, port highlighting during cable drag,
audio I/O, module deletion via double-click, 66 plugin vendors.

Not yet implemented: cable deletion UI, right-click context menus,
port hover highlighting during cable drag target selection.
