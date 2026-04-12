# NanoVG wgpu Backend Design

## Problem

Cardinal-RS module widgets don't render visually — they show as gray rectangles. The root cause is that NanoVG's GL2 backend requires an offscreen OpenGL context via EGL, and NixOS doesn't wire GPU drivers (NVIDIA EGL ICDs) into nix-shell environments properly. Multiple EGL approaches have been tried and all fail on NixOS.

## Solution

Replace the EGL + OpenGL + NanoVG GL2 rendering stack with a pure-Rust NanoVG backend using wgpu. wgpu abstracts over Vulkan/Metal/DX12/GL, and on NVIDIA/Linux uses Vulkan which has better NixOS support than EGL. Headless wgpu device creation works without a display server.

## Key Design Decisions

1. **Pure Rust implementation** — the wgpu backend is written entirely in Rust, exposing `extern "C"` callbacks matching NVGparams signatures
2. **Shared wgpu device from egui** — eframe switches from glow to wgpu backend; the same `Device`/`Queue` is shared with the Cardinal thread via `Arc`
3. **Zero-copy texture sharing** — NanoVG renders to a `wgpu::Texture`, which is registered directly with egui's renderer as a native texture (no pixel readback, no CPU copy)
4. **Keep Cardinal thread architecture** — engine + rendering stay on a dedicated thread, communicating with the UI via mpsc channels. wgpu Device/Queue are `Send + Sync`
5. **Minimal C++ bridge** — `bridge.cpp` render function becomes a thin `widget->draw()` wrapper. All frame management, render targets, and NanoVG context creation move to Rust

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  egui Main Thread (eframe wgpu backend)                     │
│                                                             │
│  - Owns eframe RenderState (Device, Queue, Renderer)        │
│  - Shares Arc<Device> + Arc<Queue> with Cardinal thread     │
│  - Receives wgpu::Texture from Cardinal thread              │
│  - Registers textures via Renderer::register_native_texture │
│  - Paints module widgets as egui images                     │
└──────────────┬──────────────────────────▲───────────────────┘
               │ Command::RenderModule    │ RenderResult { wgpu::Texture }
               ▼                          │
┌─────────────────────────────────────────────────────────────┐
│  Cardinal Thread                                            │
│                                                             │
│  - Holds Arc<Device> + Arc<Queue>                           │
│  - Owns NVGcontext (created via Rust wgpu backend)          │
│  - Render flow:                                             │
│    1. Rust: create/reuse offscreen wgpu::Texture            │
│    2. Rust: nvgBeginFrame()                                 │
│    3. C++: widget->draw(args)  (only C++ involvement)       │
│    4. Rust: nvgEndFrame() -> wgpu command encode + submit   │
│    5. Send Texture back to egui thread                      │
└─────────────────────────────────────────────────────────────┘
```

## Component Details

### 1. NanoVG wgpu Backend (`nanovg_wgpu.rs`)

Implements NanoVG's `NVGparams` backend interface (12 `extern "C"` callbacks) using wgpu.

**Backend context struct** holds:
- `Arc<wgpu::Device>`, `Arc<wgpu::Queue>`
- Render pipelines for different stencil/blend states (fill convex, fill stencil, stroke, triangles)
- Dynamic vertex buffer (re-uploaded each frame)
- Uniform bind group layout + per-draw bind groups
- Texture registry (`HashMap<i32, wgpu::Texture>`) for images + font atlas
- Current render target `wgpu::TextureView`
- Batched draw calls (same approach as GL2: fill/stroke/triangles accumulate during frame, flush executes all)

**Callbacks**:
- `renderCreate` — create WGSL shader module, pipeline layout, bind group layouts
- `renderCreateTexture` / `deleteTexture` / `updateTexture` / `getTextureSize` — manage wgpu textures in the registry
- `renderViewport` — store current dimensions
- `renderCancel` — discard batched calls
- `renderFill` — batch fill command (convex path vs stencil-then-cover)
- `renderStroke` — batch stroke command
- `renderTriangles` — batch triangle command (used for text rendering)
- `renderFlush` — encode all batched draws into a wgpu render pass and submit to queue
- `renderDelete` — cleanup all GPU resources

**Shaders (WGSL)** — port of the GL2 vertex + fragment shaders:
- Vertex: transform position, pass through UV coordinates
- Fragment: sample texture, apply paint uniforms (inner/outer color, scissor transform, stroke params)
- Same uniform structure as GL2's `GLNVGfragUniforms` mapped to a WGSL uniform buffer

**Stencil-then-cover fill** (for non-convex paths):
- Pass 1: Draw fill triangles to stencil buffer (increment/decrement by winding rule)
- Pass 2: Draw bounding quad with stencil test to keep filled pixels
- Pass 3: Reset stencil to zero
- Requires ~4 render pipelines with different `DepthStencilState` configurations

**Triangle fan conversion**:
- NanoVG emits triangle fans for fill paths
- wgpu only supports triangle lists
- Convert fan `[0,1,2,3,4,...]` to list `[0,1,2, 0,2,3, 0,3,4,...]` during batching

**Context creation** (public Rust API):
```rust
pub fn create_nanovg_context(
    device: Arc<Device>,
    queue: Arc<Queue>,
) -> *mut NVGcontext
```

Allocates the backend state, populates an `NVGparams` struct with function pointers, and calls `nvgCreateInternal`.

### 2. C++ Bridge Changes

**Removed from `bridge.cpp`**:
- EGL initialization/shutdown (~100 lines)
- NanoVG GL2 context creation (`nvgCreateGL2`)
- FBO creation/management (`nvgluCreateFramebuffer`)
- `glReadPixels` + vertical flip
- GL state management, GLEW initialization
- `cardinal_render_claim_context` / `cardinal_render_release_context`

**New render function**:
```c
// Receives NVGcontext from Rust, just calls widget->draw()
bool cardinal_module_render(ModuleHandle handle, NVGcontext* vg, int width, int height);
```

Implementation:
1. Look up ModuleWidget from handle in `g_modules`
2. Construct `rack::widget::Widget::DrawArgs` with the provided NVGcontext
3. Call `widget->draw(args)` and `widget->drawLayer(args, 1)`
4. Return success/failure

**Build system changes (`build.rs`)**:
- Remove: `nanovg_gl_impl.cpp` compilation, GLEW pkg-config probe, GL/EGL system library linking
- Keep: `nanovg.c` (core tessellation), `nanosvg_impl.cpp` (SVG loading), bridge.cpp, stubs.cpp
- Add: expose `nvgCreateInternal` and NanoVG C types to Rust FFI

### 3. egui App Changes

**eframe backend switch**:
- Change eframe Cargo feature from `glow` to `wgpu`
- Update `NativeOptions` to use `wgpu::WgpuConfiguration`

**Device sharing**:
- Extract `Device` and `Queue` from eframe's `RenderState` during first `update()` call
- Wrap in `Arc` and send to Cardinal thread via the command channel
- Cardinal thread stores them, creates NanoVG wgpu backend context

**Texture sharing flow**:
1. Cardinal thread renders module to a `wgpu::Texture` (created via shared device)
2. Sends `wgpu::Texture` back via mpsc in `RenderResult`
3. egui main thread creates a `TextureView` from received texture
4. Registers with `egui_wgpu::Renderer::register_native_texture()` to get `egui::TextureId`
5. Uses `TextureId` in `painter.image()` calls
6. On re-render: updates existing registration via `update_egui_texture_from_wgpu_texture()` instead of re-registering

**RenderResult**:
```rust
// Old
struct RenderResult { module_id: u64, width: u32, height: u32, pixels: Vec<u8> }

// New
struct RenderResult { module_id: u64, width: u32, height: u32, texture: wgpu::Texture }
```

**Module texture cache**:
- Each module's state holds `Option<egui::TextureId>` for its registered texture
- First render: `register_native_texture()`
- Subsequent renders: `update_egui_texture_from_wgpu_texture()`
- Module removal: `free_texture()`

### 4. shell.nix Changes

**Remove**:
- `libGL`, `libGLU` (OpenGL)
- `glew` (GLEW)
- `libglvnd` (GL dispatch)
- Mesa/EGL-specific packages

**Keep**:
- All non-graphics dependencies (jansson, libarchive, libsamplerate, speexdsp, etc.)

**Add** (if not present):
- `vulkan-loader` — runtime Vulkan dispatch (`libvulkan.so`)
- `vulkan-headers` — compile-time (if needed)
- Ensure `VK_ICD_FILENAMES` or `LD_LIBRARY_PATH` points to NVIDIA's Vulkan ICD on NixOS

## Implementation Order

1. FFI types + skeleton callbacks — verify `nvgCreateInternal` produces a valid `NVGcontext`
2. wgpu device sharing — eframe wgpu backend, `Arc<Device>`/`Arc<Queue>` to Cardinal thread
3. Shader module + bind group layout + pipeline layout
4. Texture management (`createTexture` / `deleteTexture` / `updateTexture`)
5. `renderTriangles` (simplest draw call — text rendering)
6. `renderFill` convex path (no stencil)
7. `renderFill` non-convex (stencil-then-cover)
8. `renderStroke`
9. `renderFlush` (full pipeline execution)
10. Zero-copy texture integration with egui
11. Bridge cleanup — strip EGL/GL from bridge.cpp and build.rs
12. shell.nix — remove GL deps, verify Vulkan environment
