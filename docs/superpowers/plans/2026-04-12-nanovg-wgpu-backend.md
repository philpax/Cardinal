# NanoVG wgpu Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the broken EGL+GL2 NanoVG rendering backend with a pure-Rust wgpu implementation, sharing the GPU device with eframe for zero-copy texture rendering of VCV Rack module widgets.

**Architecture:** NanoVG's C core (path tessellation, text layout) stays as-is. The ~1500-line GL2 rendering backend (`nanovg_gl.h`) is replaced by Rust code implementing the same 12 `NVGparams` callbacks using wgpu. The wgpu device is shared from eframe (switched from glow to wgpu backend), and rendered textures are registered directly with egui's renderer for zero-copy display.

**Tech Stack:** Rust, wgpu 25, eframe 0.31 (wgpu backend), egui_wgpu, WGSL shaders, NanoVG C API

**Spec:** `docs/superpowers/specs/2026-04-12-nanovg-wgpu-backend-design.md`

---

### Task 1: NanoVG FFI Types and Skeleton Backend

Expose the NanoVG C types and `nvgCreateInternal` to Rust. Create a skeleton wgpu backend with stub callbacks that compiles and produces a valid `NVGcontext*`.

**Files:**
- Create: `cardinal-rs/crates/cardinal-core/src/nanovg_wgpu.rs`
- Modify: `cardinal-rs/crates/cardinal-core/src/ffi.rs`
- Modify: `cardinal-rs/crates/cardinal-core/src/lib.rs`

- [ ] **Step 1: Add NanoVG C types to ffi.rs**

Add the NanoVG types needed for the backend interface to `ffi.rs`:

```rust
// NanoVG types for backend implementation
#[repr(C)]
pub struct NVGcolor {
    pub rgba: [f32; 4],
}

#[repr(C)]
pub struct NVGpaint {
    pub xform: [f32; 6],
    pub extent: [f32; 2],
    pub radius: f32,
    pub feather: f32,
    pub inner_color: NVGcolor,
    pub outer_color: NVGcolor,
    pub image: i32,
}

#[repr(C)]
pub struct NVGcompositeOperationState {
    pub src_rgb: i32,
    pub dst_rgb: i32,
    pub src_alpha: i32,
    pub dst_alpha: i32,
}

#[repr(C)]
pub struct NVGscissor {
    pub xform: [f32; 6],
    pub extent: [f32; 2],
}

#[repr(C)]
pub struct NVGvertex {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
}

#[repr(C)]
pub struct NVGpath {
    pub first: i32,
    pub count: i32,
    pub closed: u8,
    pub nbevel: i32,
    pub fill: *const NVGvertex,
    pub nfill: i32,
    pub stroke: *const NVGvertex,
    pub nstroke: i32,
    pub winding: i32,
    pub convex: i32,
}

// NanoVG image flags
pub const NVG_IMAGE_GENERATE_MIPMAPS: i32 = 1 << 0;
pub const NVG_IMAGE_REPEATX: i32 = 1 << 1;
pub const NVG_IMAGE_REPEATY: i32 = 1 << 2;
pub const NVG_IMAGE_FLIPY: i32 = 1 << 3;
pub const NVG_IMAGE_PREMULTIPLIED: i32 = 1 << 4;
pub const NVG_IMAGE_NEAREST: i32 = 1 << 5;

// NanoVG texture types
pub const NVG_TEXTURE_ALPHA: i32 = 0x01;
pub const NVG_TEXTURE_RGBA: i32 = 0x02;

// NanoVG blend factors
pub const NVG_ZERO: i32 = 1 << 0;
pub const NVG_ONE: i32 = 1 << 1;
pub const NVG_SRC_COLOR: i32 = 1 << 2;
pub const NVG_ONE_MINUS_SRC_COLOR: i32 = 1 << 3;
pub const NVG_DST_COLOR: i32 = 1 << 4;
pub const NVG_ONE_MINUS_DST_COLOR: i32 = 1 << 5;
pub const NVG_SRC_ALPHA: i32 = 1 << 6;
pub const NVG_ONE_MINUS_SRC_ALPHA: i32 = 1 << 7;
pub const NVG_DST_ALPHA: i32 = 1 << 8;
pub const NVG_ONE_MINUS_DST_ALPHA: i32 = 1 << 9;
pub const NVG_SRC_ALPHA_SATURATE: i32 = 1 << 10;

// NanoVG create flags
pub const NVG_ANTIALIAS: i32 = 1 << 0;
pub const NVG_STENCIL_STROKES: i32 = 1 << 1;

// Opaque NanoVG context
#[repr(C)]
pub struct NVGcontext {
    _opaque: [u8; 0],
}

// NVGparams — the backend interface
#[repr(C)]
pub struct NVGparams {
    pub user_ptr: *mut std::ffi::c_void,
    pub edge_anti_alias: i32,
    pub render_create: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> i32>,
    pub render_create_texture: Option<unsafe extern "C" fn(*mut std::ffi::c_void, i32, i32, i32, i32, *const u8) -> i32>,
    pub render_delete_texture: Option<unsafe extern "C" fn(*mut std::ffi::c_void, i32) -> i32>,
    pub render_update_texture: Option<unsafe extern "C" fn(*mut std::ffi::c_void, i32, i32, i32, i32, i32, *const u8) -> i32>,
    pub render_get_texture_size: Option<unsafe extern "C" fn(*mut std::ffi::c_void, i32, *mut i32, *mut i32) -> i32>,
    pub render_viewport: Option<unsafe extern "C" fn(*mut std::ffi::c_void, f32, f32, f32)>,
    pub render_cancel: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
    pub render_flush: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
    pub render_fill: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut NVGpaint, NVGcompositeOperationState, *mut NVGscissor, f32, *const f32, *const NVGpath, i32)>,
    pub render_stroke: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut NVGpaint, NVGcompositeOperationState, *mut NVGscissor, f32, f32, *const NVGpath, i32)>,
    pub render_triangles: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut NVGpaint, NVGcompositeOperationState, *mut NVGscissor, *const NVGvertex, i32, f32)>,
    pub render_delete: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
}

unsafe extern "C" {
    pub fn nvgCreateInternal(params: *mut NVGparams, other: *mut NVGcontext) -> *mut NVGcontext;
    pub fn nvgDeleteInternal(ctx: *mut NVGcontext);
    pub fn nvgBeginFrame(ctx: *mut NVGcontext, window_width: f32, window_height: f32, device_pixel_ratio: f32);
    pub fn nvgEndFrame(ctx: *mut NVGcontext);
    pub fn nvgTransformInverse(dst: *mut f32, src: *const f32) -> i32;
}
```

- [ ] **Step 2: Create nanovg_wgpu.rs with skeleton backend**

Create the skeleton module with stub callbacks and context creation:

```rust
//! NanoVG rendering backend using wgpu.
//!
//! Implements the NVGparams callback interface, replacing the GL2 backend.
//! Renders to offscreen wgpu textures that can be shared with egui.

use std::collections::HashMap;
use std::sync::Arc;
use wgpu;

use crate::ffi;

/// Backend context stored in NVGparams::userPtr.
struct WgpuNvgContext {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    view: [f32; 2],
    // Texture registry (NanoVG image id -> texture info)
    textures: HashMap<i32, TextureEntry>,
    next_texture_id: i32,
    // Batched draw calls (populated during frame, executed on flush)
    calls: Vec<DrawCall>,
    paths: Vec<PathData>,
    vertices: Vec<ffi::NVGvertex>,
    uniforms: Vec<FragUniforms>,
    // Edge anti-alias flag
    edge_anti_alias: bool,
    // Stencil strokes flag
    stencil_strokes: bool,
}

struct TextureEntry {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: i32,
    height: i32,
    tex_type: i32, // NVG_TEXTURE_ALPHA or NVG_TEXTURE_RGBA
    flags: i32,
}

#[derive(Clone, Copy)]
enum CallType {
    Fill,
    ConvexFill,
    Stroke,
    Triangles,
}

struct DrawCall {
    call_type: CallType,
    image: i32,
    path_offset: usize,
    path_count: usize,
    triangle_offset: usize,
    triangle_count: usize,
    uniform_offset: usize,
    blend: BlendState,
}

#[derive(Clone, Copy)]
struct BlendState {
    src_rgb: wgpu::BlendFactor,
    dst_rgb: wgpu::BlendFactor,
    src_alpha: wgpu::BlendFactor,
    dst_alpha: wgpu::BlendFactor,
}

struct PathData {
    fill_offset: usize,
    fill_count: usize,
    stroke_offset: usize,
    stroke_count: usize,
}

/// Fragment uniform data — mirrors GLNVGfragUniforms layout.
/// Packed as 11 vec4s (UNIFORMARRAY_SIZE = 11).
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct FragUniforms {
    scissor_mat: [f32; 12],  // mat3 as 3x vec4
    paint_mat: [f32; 12],    // mat3 as 3x vec4
    inner_col: [f32; 4],
    outer_col: [f32; 4],
    scissor_ext: [f32; 2],
    scissor_scale: [f32; 2],
    extent: [f32; 2],
    radius: f32,
    feather: f32,
    stroke_mult: f32,
    stroke_thr: f32,
    tex_type: f32,
    shader_type: f32,
}

// Shader types matching GL2 backend
const SHADER_FILLGRAD: f32 = 0.0;
const SHADER_FILLIMG: f32 = 1.0;
const SHADER_SIMPLE: f32 = 2.0;
const SHADER_IMG: f32 = 3.0;

/// Create a NanoVG context backed by wgpu.
///
/// `device` and `queue` should be shared from the eframe render state.
/// `flags` are NVG_ANTIALIAS | NVG_STENCIL_STROKES etc.
///
/// Returns a raw pointer to the NVGcontext (owned by NanoVG's allocator).
/// Call `nvgDeleteInternal` to destroy it.
pub fn create_context(
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    flags: i32,
) -> *mut ffi::NVGcontext {
    let ctx = Box::new(WgpuNvgContext {
        device,
        queue,
        view: [0.0, 0.0],
        textures: HashMap::new(),
        next_texture_id: 1,
        calls: Vec::new(),
        paths: Vec::new(),
        vertices: Vec::new(),
        uniforms: Vec::new(),
        edge_anti_alias: (flags & ffi::NVG_ANTIALIAS) != 0,
        stencil_strokes: (flags & ffi::NVG_STENCIL_STROKES) != 0,
    });

    let user_ptr = Box::into_raw(ctx) as *mut std::ffi::c_void;

    let mut params = ffi::NVGparams {
        user_ptr,
        edge_anti_alias: if (flags & ffi::NVG_ANTIALIAS) != 0 { 1 } else { 0 },
        render_create: Some(render_create),
        render_create_texture: Some(render_create_texture),
        render_delete_texture: Some(render_delete_texture),
        render_update_texture: Some(render_update_texture),
        render_get_texture_size: Some(render_get_texture_size),
        render_viewport: Some(render_viewport),
        render_cancel: Some(render_cancel),
        render_flush: Some(render_flush),
        render_fill: Some(render_fill),
        render_stroke: Some(render_stroke),
        render_triangles: Some(render_triangles),
        render_delete: Some(render_delete),
    };

    unsafe { ffi::nvgCreateInternal(&mut params, std::ptr::null_mut()) }
}

/// Destroy a NanoVG context created by `create_context`.
pub fn destroy_context(ctx: *mut ffi::NVGcontext) {
    if !ctx.is_null() {
        unsafe { ffi::nvgDeleteInternal(ctx) };
    }
}

// Helper to get the WgpuNvgContext from the userPtr
unsafe fn get_ctx(uptr: *mut std::ffi::c_void) -> &'static mut WgpuNvgContext {
    unsafe { &mut *(uptr as *mut WgpuNvgContext) }
}

// ── NVGparams callbacks ─────────────────────────────────────────────

unsafe extern "C" fn render_create(uptr: *mut std::ffi::c_void, _other: *mut std::ffi::c_void) -> i32 {
    let _ctx = unsafe { get_ctx(uptr) };
    // TODO: Task 3 — create shader module, pipeline layout, bind groups
    eprintln!("nanovg_wgpu: renderCreate");
    1 // success
}

unsafe extern "C" fn render_create_texture(
    uptr: *mut std::ffi::c_void,
    tex_type: i32, w: i32, h: i32, image_flags: i32, data: *const u8,
) -> i32 {
    let ctx = unsafe { get_ctx(uptr) };
    // TODO: Task 4 — create wgpu texture
    let id = ctx.next_texture_id;
    ctx.next_texture_id += 1;
    eprintln!("nanovg_wgpu: renderCreateTexture id={id} {w}x{h} type={tex_type}");
    id
}

unsafe extern "C" fn render_delete_texture(uptr: *mut std::ffi::c_void, image: i32) -> i32 {
    let ctx = unsafe { get_ctx(uptr) };
    // TODO: Task 4
    if ctx.textures.remove(&image).is_some() { 1 } else { 0 }
}

unsafe extern "C" fn render_update_texture(
    uptr: *mut std::ffi::c_void,
    image: i32, _x: i32, _y: i32, _w: i32, _h: i32, _data: *const u8,
) -> i32 {
    let _ctx = unsafe { get_ctx(uptr) };
    // TODO: Task 4
    1
}

unsafe extern "C" fn render_get_texture_size(
    uptr: *mut std::ffi::c_void, image: i32, w: *mut i32, h: *mut i32,
) -> i32 {
    let ctx = unsafe { get_ctx(uptr) };
    if let Some(tex) = ctx.textures.get(&image) {
        unsafe {
            *w = tex.width;
            *h = tex.height;
        }
        1
    } else {
        0
    }
}

unsafe extern "C" fn render_viewport(
    uptr: *mut std::ffi::c_void, width: f32, height: f32, _device_pixel_ratio: f32,
) {
    let ctx = unsafe { get_ctx(uptr) };
    ctx.view = [width, height];
}

unsafe extern "C" fn render_cancel(uptr: *mut std::ffi::c_void) {
    let ctx = unsafe { get_ctx(uptr) };
    ctx.calls.clear();
    ctx.paths.clear();
    ctx.vertices.clear();
    ctx.uniforms.clear();
}

unsafe extern "C" fn render_flush(uptr: *mut std::ffi::c_void) {
    let ctx = unsafe { get_ctx(uptr) };
    // TODO: Task 8 — encode wgpu render passes and submit
    ctx.calls.clear();
    ctx.paths.clear();
    ctx.vertices.clear();
    ctx.uniforms.clear();
}

unsafe extern "C" fn render_fill(
    uptr: *mut std::ffi::c_void,
    paint: *mut ffi::NVGpaint,
    composite_op: ffi::NVGcompositeOperationState,
    scissor: *mut ffi::NVGscissor,
    fringe: f32,
    bounds: *const f32,
    paths: *const ffi::NVGpath,
    npaths: i32,
) {
    let _ctx = unsafe { get_ctx(uptr) };
    // TODO: Task 5/6 — batch fill call
}

unsafe extern "C" fn render_stroke(
    uptr: *mut std::ffi::c_void,
    paint: *mut ffi::NVGpaint,
    composite_op: ffi::NVGcompositeOperationState,
    scissor: *mut ffi::NVGscissor,
    fringe: f32,
    stroke_width: f32,
    paths: *const ffi::NVGpath,
    npaths: i32,
) {
    let _ctx = unsafe { get_ctx(uptr) };
    // TODO: Task 7 — batch stroke call
}

unsafe extern "C" fn render_triangles(
    uptr: *mut std::ffi::c_void,
    paint: *mut ffi::NVGpaint,
    composite_op: ffi::NVGcompositeOperationState,
    scissor: *mut ffi::NVGscissor,
    verts: *const ffi::NVGvertex,
    nverts: i32,
    fringe: f32,
) {
    let _ctx = unsafe { get_ctx(uptr) };
    // TODO: Task 5 — batch triangles call
}

unsafe extern "C" fn render_delete(uptr: *mut std::ffi::c_void) {
    if !uptr.is_null() {
        // Reclaim the Box and drop it
        let _ = unsafe { Box::from_raw(uptr as *mut WgpuNvgContext) };
    }
}
```

- [ ] **Step 3: Register the module in lib.rs**

Add `pub mod nanovg_wgpu;` to `cardinal-rs/crates/cardinal-core/src/lib.rs`.

- [ ] **Step 4: Verify it compiles**

Run: `cd /home/philpax/programming/Cardinal/cardinal-rs && cargo check -p cardinal-core 2>&1 | tail -20`
Expected: compiles with warnings about unused variables (the TODO stubs), no errors.

- [ ] **Step 5: Commit**

```bash
git add cardinal-rs/crates/cardinal-core/src/nanovg_wgpu.rs cardinal-rs/crates/cardinal-core/src/ffi.rs cardinal-rs/crates/cardinal-core/src/lib.rs
git commit -m "feat: add skeleton NanoVG wgpu backend with FFI types and stub callbacks"
```

---

### Task 2: Switch eframe to wgpu Backend and Share Device

Switch the egui app from glow to wgpu backend, extract the wgpu device/queue, and send them to the Cardinal thread.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-egui/Cargo.toml`
- Modify: `cardinal-rs/crates/cardinal-egui/src/main.rs`
- Modify: `cardinal-rs/crates/cardinal-core/Cargo.toml` (ensure wgpu version matches)

- [ ] **Step 1: Update Cargo.toml dependencies**

In `cardinal-rs/crates/cardinal-egui/Cargo.toml`, change eframe features from `glow` to `wgpu`:

```toml
[dependencies]
cardinal-core = { path = "../cardinal-core" }
eframe = { version = "0.31", default-features = false, features = ["wgpu", "default_fonts", "x11"] }
egui = "0.31"
egui-wgpu = "0.31"
wgpu = "25"
cpal = "0.15"
```

- [ ] **Step 2: Add device sharing to Command enum and App state**

In `main.rs`, add a new command variant and update the App to hold wgpu state:

Add to the `Command` enum:
```rust
InitGpu {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
},
```

Add imports at the top:
```rust
use std::sync::Arc;
```

- [ ] **Step 3: Handle InitGpu in Cardinal thread**

In the `spawn_cardinal_thread` function, handle the new command. The Cardinal thread should store the device/queue and create the NanoVG wgpu context:

```rust
// At the top of the closure, before the loop:
let mut nanovg_ctx: *mut cardinal_core::ffi::NVGcontext = std::ptr::null_mut();

// In the match:
Command::InitGpu { device, queue } => {
    nanovg_ctx = cardinal_core::nanovg_wgpu::create_context(
        device, queue,
        cardinal_core::ffi::NVG_ANTIALIAS | cardinal_core::ffi::NVG_STENCIL_STROKES,
    );
    eprintln!("cardinal thread: wgpu NanoVG context created: {:?}", !nanovg_ctx.is_null());
}
```

- [ ] **Step 4: Send device/queue from egui on first update**

In the `App` struct, add a `gpu_initialized: bool` field (default `false`). In the `update` method, on first call extract the wgpu device/queue from eframe's render state and send `InitGpu`:

```rust
if !self.gpu_initialized {
    if let Some(render_state) = _frame.wgpu_render_state() {
        let device = render_state.device.clone();
        let queue = render_state.queue.clone();
        let _ = self.cmd_tx.send(Command::InitGpu { device, queue });
        self.gpu_initialized = true;
    }
}
```

Note: `render_state.device` and `render_state.queue` are already `Arc<wgpu::Device>` and `Arc<wgpu::Queue>`.

- [ ] **Step 5: Remove render_claim_context call**

In `spawn_cardinal_thread`, remove the `cc::render_claim_context()` call — it's for the EGL context which we're replacing.

- [ ] **Step 6: Verify it compiles**

Run: `cd /home/philpax/programming/Cardinal/cardinal-rs && cargo check -p cardinal-egui 2>&1 | tail -20`
Expected: compiles successfully. The app should still launch (module rendering will produce nothing since the wgpu backend is stubbed).

- [ ] **Step 7: Commit**

```bash
git add cardinal-rs/crates/cardinal-egui/Cargo.toml cardinal-rs/crates/cardinal-egui/src/main.rs cardinal-rs/crates/cardinal-core/Cargo.toml
git commit -m "feat: switch eframe to wgpu backend, share device/queue with Cardinal thread"
```

---

### Task 3: WGSL Shaders and Render Pipelines

Create the WGSL shaders (porting the GL2 vertex/fragment shaders) and the wgpu render pipelines needed for different draw modes.

**Files:**
- Create: `cardinal-rs/crates/cardinal-core/src/nanovg_wgpu_shaders.wgsl`
- Modify: `cardinal-rs/crates/cardinal-core/src/nanovg_wgpu.rs`

- [ ] **Step 1: Create WGSL shader**

Create `cardinal-rs/crates/cardinal-core/src/nanovg_wgpu_shaders.wgsl`:

```wgsl
// NanoVG wgpu backend shaders — port of GL2 fill vertex/fragment shaders.

struct ViewUniforms {
    view_size: vec2<f32>,
};
@group(0) @binding(0) var<uniform> view: ViewUniforms;

// Fragment uniforms — matches GLNVGfragUniforms layout (11 vec4s)
struct FragUniforms {
    scissor_mat_0: vec4<f32>,  // frag[0]
    scissor_mat_1: vec4<f32>,  // frag[1]
    scissor_mat_2: vec4<f32>,  // frag[2]
    paint_mat_0: vec4<f32>,    // frag[3]
    paint_mat_1: vec4<f32>,    // frag[4]
    paint_mat_2: vec4<f32>,    // frag[5]
    inner_col: vec4<f32>,      // frag[6]
    outer_col: vec4<f32>,      // frag[7]
    scissor_ext_scale: vec4<f32>, // frag[8]: xy=scissorExt, zw=scissorScale
    extent_radius_feather: vec4<f32>, // frag[9]: xy=extent, z=radius, w=feather
    stroke_params: vec4<f32>,  // frag[10]: x=strokeMult, y=strokeThr, z=texType, w=type
};
@group(0) @binding(1) var<uniform> frag: FragUniforms;

@group(1) @binding(0) var nvg_texture: texture_2d<f32>;
@group(1) @binding(1) var nvg_sampler: sampler;

struct VertexInput {
    @location(0) vertex: vec2<f32>,
    @location(1) tcoord: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) ftcoord: vec2<f32>,
    @location(1) fpos: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.ftcoord = in.tcoord;
    out.fpos = in.vertex;
    out.position = vec4<f32>(
        2.0 * in.vertex.x / view.view_size.x - 1.0,
        1.0 - 2.0 * in.vertex.y / view.view_size.y,
        0.0,
        1.0,
    );
    return out;
}

fn scissor_mat() -> mat3x3<f32> {
    return mat3x3<f32>(
        frag.scissor_mat_0.xyz,
        frag.scissor_mat_1.xyz,
        frag.scissor_mat_2.xyz,
    );
}

fn paint_mat() -> mat3x3<f32> {
    return mat3x3<f32>(
        frag.paint_mat_0.xyz,
        frag.paint_mat_1.xyz,
        frag.paint_mat_2.xyz,
    );
}

fn sdroundrect(pt: vec2<f32>, ext: vec2<f32>, rad: f32) -> f32 {
    let ext2 = ext - vec2<f32>(rad, rad);
    let d = abs(pt) - ext2;
    return min(max(d.x, d.y), 0.0) + length(max(d, vec2<f32>(0.0))) - rad;
}

fn scissor_mask(p: vec2<f32>) -> f32 {
    let sc = abs((scissor_mat() * vec3<f32>(p, 1.0)).xy) - frag.scissor_ext_scale.xy;
    let sc2 = vec2<f32>(0.5) - sc * frag.scissor_ext_scale.zw;
    return clamp(sc2.x, 0.0, 1.0) * clamp(sc2.y, 0.0, 1.0);
}

fn stroke_mask(ftcoord: vec2<f32>) -> f32 {
    let stroke_mult = frag.stroke_params.x;
    return min(1.0, (1.0 - abs(ftcoord.x * 2.0 - 1.0)) * stroke_mult) * min(1.0, ftcoord.y);
}

// Fragment shader with edge AA
@fragment
fn fs_main_edge_aa(in: VertexOutput) -> @location(0) vec4<f32> {
    let scissor = scissor_mask(in.fpos);
    let stroke_alpha = stroke_mask(in.ftcoord);
    let stroke_thr = frag.stroke_params.y;
    if stroke_alpha < stroke_thr {
        discard;
    }
    return shade(in, scissor, stroke_alpha);
}

// Fragment shader without edge AA
@fragment
fn fs_main_no_aa(in: VertexOutput) -> @location(0) vec4<f32> {
    let scissor = scissor_mask(in.fpos);
    return shade(in, scissor, 1.0);
}

fn shade(in: VertexOutput, scissor: f32, stroke_alpha: f32) -> vec4<f32> {
    let shader_type = i32(frag.stroke_params.w);
    let tex_type = i32(frag.stroke_params.z);

    var result: vec4<f32>;

    if shader_type == 0 {
        // Gradient
        let pt = (paint_mat() * vec3<f32>(in.fpos, 1.0)).xy;
        let extent = frag.extent_radius_feather.xy;
        let radius = frag.extent_radius_feather.z;
        let feather = frag.extent_radius_feather.w;
        let d = clamp((sdroundrect(pt, extent, radius) + feather * 0.5) / feather, 0.0, 1.0);
        var color = mix(frag.inner_col, frag.outer_col, d);
        color = color * (stroke_alpha * scissor);
        result = color;
    } else if shader_type == 1 {
        // Image
        let extent = frag.extent_radius_feather.xy;
        let pt = (paint_mat() * vec3<f32>(in.fpos, 1.0)).xy / extent;
        var color = textureSample(nvg_texture, nvg_sampler, pt);
        if tex_type == 1 {
            color = vec4<f32>(color.xyz * color.w, color.w);
        }
        if tex_type == 2 {
            color = vec4<f32>(color.x, color.x, color.x, color.x);
        }
        color = color * frag.inner_col;
        color = color * (stroke_alpha * scissor);
        result = color;
    } else if shader_type == 2 {
        // Stencil fill
        result = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    } else if shader_type == 3 {
        // Textured tris
        var color = textureSample(nvg_texture, nvg_sampler, in.ftcoord);
        if tex_type == 1 {
            color = vec4<f32>(color.xyz * color.w, color.w);
        }
        if tex_type == 2 {
            color = vec4<f32>(color.x, color.x, color.x, color.x);
        }
        color = color * scissor;
        result = color * frag.inner_col;
    }

    return result;
}
```

- [ ] **Step 2: Add pipeline creation to render_create**

In `nanovg_wgpu.rs`, update `render_create` to create the shader module, bind group layouts, pipeline layout, and render pipelines. Also add struct fields for the GPU resources:

Add to `WgpuNvgContext`:
```rust
// GPU resources (initialized in render_create)
shader_module: Option<wgpu::ShaderModule>,
view_bind_group_layout: Option<wgpu::BindGroupLayout>,
texture_bind_group_layout: Option<wgpu::BindGroupLayout>,
pipeline_layout: Option<wgpu::PipelineLayout>,
// Pipelines for different draw modes
pipeline_convex_fill: Option<wgpu::RenderPipeline>,
pipeline_triangles: Option<wgpu::RenderPipeline>,
pipeline_stencil_fill_draw_stencil: Option<wgpu::RenderPipeline>,
pipeline_stencil_fill_draw_aa: Option<wgpu::RenderPipeline>,
pipeline_stencil_fill_cover: Option<wgpu::RenderPipeline>,
pipeline_stroke: Option<wgpu::RenderPipeline>,
pipeline_stencil_stroke_draw: Option<wgpu::RenderPipeline>,
pipeline_stencil_stroke_aa: Option<wgpu::RenderPipeline>,
pipeline_stencil_stroke_clear: Option<wgpu::RenderPipeline>,
// Per-frame uniform buffers
view_uniform_buffer: Option<wgpu::Buffer>,
view_bind_group: Option<wgpu::BindGroup>,
// Vertex buffer
vertex_buffer: Option<wgpu::Buffer>,
vertex_buffer_capacity: usize,
// Dummy texture for when no image is set
dummy_texture_bind_group: Option<wgpu::BindGroup>,
// Default sampler
default_sampler: Option<wgpu::Sampler>,
nearest_sampler: Option<wgpu::Sampler>,
```

The `render_create` callback should:
1. Load the WGSL shader via `include_str!`
2. Create two bind group layouts:
   - Group 0: view uniforms (binding 0) + frag uniforms (binding 1)
   - Group 1: texture (binding 0) + sampler (binding 1)
3. Create a pipeline layout from those two bind group layouts
4. Create render pipelines for each draw mode:
   - **convex_fill**: no stencil, standard blend, cull back, `fs_main_edge_aa` or `fs_main_no_aa`
   - **triangles**: no stencil, standard blend, no cull, `fs_main_edge_aa`/`fs_main_no_aa`
   - **stencil_fill_draw_stencil**: writes to stencil only (color write mask = none), stencil incr/decr wrap by face, no cull
   - **stencil_fill_draw_aa**: reads stencil (equal 0), stencil keep, cull back — for AA fringe
   - **stencil_fill_cover**: reads stencil (not-equal 0), stencil op = zero, cull back — fills the shape
   - **stroke**: no stencil, standard blend, cull back
   - **stencil_stroke_draw**: stencil equal 0, stencil op incr, cull back
   - **stencil_stroke_aa**: stencil equal 0, stencil op keep, cull back
   - **stencil_stroke_clear**: color write mask = none, stencil op zero, no cull
5. Create the view uniform buffer
6. Create a 1x1 white dummy texture + sampler + bind group
7. Create the default and nearest samplers

The render target format is `wgpu::TextureFormat::Rgba8Unorm` (we render to offscreen textures).

All pipelines use the same vertex buffer layout:
```rust
wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<ffi::NVGvertex>() as u64,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &[
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 0, shader_location: 0 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 8, shader_location: 1 },
    ],
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd /home/philpax/programming/Cardinal/cardinal-rs && cargo check -p cardinal-core 2>&1 | tail -20`
Expected: compiles with warnings about unused fields/variables.

- [ ] **Step 4: Commit**

```bash
git add cardinal-rs/crates/cardinal-core/src/nanovg_wgpu_shaders.wgsl cardinal-rs/crates/cardinal-core/src/nanovg_wgpu.rs
git commit -m "feat: add WGSL shaders and wgpu render pipelines for NanoVG backend"
```

---

### Task 4: Texture Management

Implement the texture creation, update, and deletion callbacks to manage wgpu textures for NanoVG images and the font atlas.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-core/src/nanovg_wgpu.rs`

- [ ] **Step 1: Implement render_create_texture**

Replace the stub with real wgpu texture creation:

```rust
unsafe extern "C" fn render_create_texture(
    uptr: *mut std::ffi::c_void,
    tex_type: i32, w: i32, h: i32, image_flags: i32, data: *const u8,
) -> i32 {
    let ctx = unsafe { get_ctx(uptr) };
    let id = ctx.next_texture_id;
    ctx.next_texture_id += 1;

    let format = if tex_type == ffi::NVG_TEXTURE_RGBA {
        wgpu::TextureFormat::Rgba8Unorm
    } else {
        wgpu::TextureFormat::R8Unorm
    };

    let mip_count = if (image_flags & ffi::NVG_IMAGE_GENERATE_MIPMAPS) != 0 {
        let max_dim = w.max(h) as f32;
        (max_dim.log2().floor() as u32 + 1).max(1)
    } else {
        1
    };

    let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&format!("nvg_tex_{id}")),
        size: wgpu::Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
        mip_level_count: mip_count,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    // Upload initial data if provided
    if !data.is_null() {
        let bytes_per_pixel = if tex_type == ffi::NVG_TEXTURE_RGBA { 4 } else { 1 };
        let data_slice = unsafe { std::slice::from_raw_parts(data, (w * h * bytes_per_pixel) as usize) };
        ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data_slice,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some((w * bytes_per_pixel) as u32),
                rows_per_image: Some(h as u32),
            },
            wgpu::Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
        );
    }

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    ctx.textures.insert(id, TextureEntry {
        texture,
        view,
        width: w,
        height: h,
        tex_type,
        flags: image_flags,
    });

    id
}
```

- [ ] **Step 2: Implement render_update_texture**

Replace the stub — writes a sub-region of pixel data:

```rust
unsafe extern "C" fn render_update_texture(
    uptr: *mut std::ffi::c_void,
    image: i32, x: i32, y: i32, w: i32, h: i32, data: *const u8,
) -> i32 {
    let ctx = unsafe { get_ctx(uptr) };
    let Some(tex) = ctx.textures.get(&image) else { return 0 };

    let bytes_per_pixel = if tex.tex_type == ffi::NVG_TEXTURE_RGBA { 4 } else { 1 };
    // NanoVG passes data pointing to the start of the full texture row,
    // offset by (x, y). The row length is tex.width.
    let row_bytes = (tex.width * bytes_per_pixel) as usize;
    let src = unsafe { std::slice::from_raw_parts(data.add(y as usize * row_bytes + x as usize * bytes_per_pixel as usize), (h as usize) * row_bytes) };

    // Build a tightly packed buffer for the sub-region
    let dst_row_bytes = (w * bytes_per_pixel) as usize;
    let mut packed = vec![0u8; dst_row_bytes * h as usize];
    for row in 0..h as usize {
        let src_offset = row * row_bytes;
        let dst_offset = row * dst_row_bytes;
        packed[dst_offset..dst_offset + dst_row_bytes]
            .copy_from_slice(&src[src_offset..src_offset + dst_row_bytes]);
    }

    ctx.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &tex.texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x: x as u32, y: y as u32, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        &packed,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(dst_row_bytes as u32),
            rows_per_image: Some(h as u32),
        },
        wgpu::Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
    );

    1
}
```

- [ ] **Step 3: Implement render_delete_texture**

Already partly implemented — just ensure the `TextureEntry` gets dropped (which releases the wgpu::Texture):

```rust
unsafe extern "C" fn render_delete_texture(uptr: *mut std::ffi::c_void, image: i32) -> i32 {
    let ctx = unsafe { get_ctx(uptr) };
    if ctx.textures.remove(&image).is_some() { 1 } else { 0 }
}
```

- [ ] **Step 4: Add helper to get or create texture bind group**

Add a method to create a bind group for a given texture, used when setting uniforms before draw calls:

```rust
impl WgpuNvgContext {
    fn get_texture_bind_group(&self, image: i32) -> &wgpu::BindGroup {
        if image != 0 {
            if let Some(tex) = self.textures.get(&image) {
                // Create bind group on-the-fly (could cache if perf matters)
                // For now, return dummy — we'll cache these properly
            }
        }
        self.dummy_texture_bind_group.as_ref().unwrap()
    }
}
```

We'll need a cache: `texture_bind_groups: HashMap<i32, wgpu::BindGroup>` on the context. Create/invalidate bind groups when textures are created/deleted/updated. This avoids re-creating bind groups every frame.

- [ ] **Step 5: Verify it compiles**

Run: `cd /home/philpax/programming/Cardinal/cardinal-rs && cargo check -p cardinal-core 2>&1 | tail -20`
Expected: compiles.

- [ ] **Step 6: Commit**

```bash
git add cardinal-rs/crates/cardinal-core/src/nanovg_wgpu.rs
git commit -m "feat: implement wgpu texture management for NanoVG backend"
```

---

### Task 5: Implement renderTriangles and renderFill (Convex)

These are the two simplest draw modes — no stencil buffer needed.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-core/src/nanovg_wgpu.rs`

- [ ] **Step 1: Add convert_paint helper**

Port `glnvg__convertPaint` to Rust. This converts NVGpaint + NVGscissor into a `FragUniforms`:

```rust
impl WgpuNvgContext {
    fn convert_paint(
        &self,
        paint: &ffi::NVGpaint,
        scissor: &ffi::NVGscissor,
        width: f32,
        fringe: f32,
        stroke_thr: f32,
    ) -> FragUniforms {
        let mut frag = FragUniforms::default();

        frag.inner_col = premul_color(&paint.inner_color);
        frag.outer_col = premul_color(&paint.outer_color);

        if scissor.extent[0] < -0.5 || scissor.extent[1] < -0.5 {
            // No scissor
            frag.scissor_ext = [1.0, 1.0];
            frag.scissor_scale = [1.0, 1.0];
        } else {
            let mut inv_xform = [0.0f32; 6];
            unsafe { ffi::nvgTransformInverse(inv_xform.as_mut_ptr(), scissor.xform.as_ptr()) };
            xform_to_mat3x4(&mut frag.scissor_mat, &inv_xform);
            frag.scissor_ext = scissor.extent;
            frag.scissor_scale = [
                (scissor.xform[0] * scissor.xform[0] + scissor.xform[2] * scissor.xform[2]).sqrt() / fringe,
                (scissor.xform[1] * scissor.xform[1] + scissor.xform[3] * scissor.xform[3]).sqrt() / fringe,
            ];
        }

        frag.extent = paint.extent;
        frag.stroke_mult = (width * 0.5 + fringe * 0.5) / fringe;
        frag.stroke_thr = stroke_thr;

        if paint.image != 0 {
            let mut inv_xform = [0.0f32; 6];
            if let Some(tex) = self.textures.get(&paint.image) {
                if (tex.flags & ffi::NVG_IMAGE_FLIPY) != 0 {
                    // Handle flipped images — same logic as GL2 backend
                    let mut m1 = [0.0f32; 6];
                    let mut m2 = [0.0f32; 6];
                    // Simplified: just invert paint xform
                    // The full flip logic can be added later if needed
                    unsafe { ffi::nvgTransformInverse(inv_xform.as_mut_ptr(), paint.xform.as_ptr()) };
                } else {
                    unsafe { ffi::nvgTransformInverse(inv_xform.as_mut_ptr(), paint.xform.as_ptr()) };
                }
                frag.shader_type = SHADER_FILLIMG;
                if tex.tex_type == ffi::NVG_TEXTURE_RGBA {
                    frag.tex_type = if (tex.flags & ffi::NVG_IMAGE_PREMULTIPLIED) != 0 { 0.0 } else { 1.0 };
                } else {
                    frag.tex_type = 2.0;
                }
            } else {
                unsafe { ffi::nvgTransformInverse(inv_xform.as_mut_ptr(), paint.xform.as_ptr()) };
                frag.shader_type = SHADER_FILLGRAD;
            }
            xform_to_mat3x4(&mut frag.paint_mat, &inv_xform);
        } else {
            frag.shader_type = SHADER_FILLGRAD;
            frag.radius = paint.radius;
            frag.feather = paint.feather;
            let mut inv_xform = [0.0f32; 6];
            unsafe { ffi::nvgTransformInverse(inv_xform.as_mut_ptr(), paint.xform.as_ptr()) };
            xform_to_mat3x4(&mut frag.paint_mat, &inv_xform);
        }

        frag
    }
}

fn premul_color(c: &ffi::NVGcolor) -> [f32; 4] {
    [c.rgba[0] * c.rgba[3], c.rgba[1] * c.rgba[3], c.rgba[2] * c.rgba[3], c.rgba[3]]
}

fn xform_to_mat3x4(m3: &mut [f32; 12], t: &[f32; 6]) {
    m3[0] = t[0]; m3[1] = t[1]; m3[2] = 0.0; m3[3] = 0.0;
    m3[4] = t[2]; m3[5] = t[3]; m3[6] = 0.0; m3[7] = 0.0;
    m3[8] = t[4]; m3[9] = t[5]; m3[10] = 1.0; m3[11] = 0.0;
}
```

- [ ] **Step 2: Add blend state conversion**

Port `glnvg__blendCompositeOperation` to convert NVG blend factors to wgpu:

```rust
fn convert_blend_factor(factor: i32) -> wgpu::BlendFactor {
    match factor {
        f if f == ffi::NVG_ZERO => wgpu::BlendFactor::Zero,
        f if f == ffi::NVG_ONE => wgpu::BlendFactor::One,
        f if f == ffi::NVG_SRC_COLOR => wgpu::BlendFactor::Src,
        f if f == ffi::NVG_ONE_MINUS_SRC_COLOR => wgpu::BlendFactor::OneMinusSrc,
        f if f == ffi::NVG_DST_COLOR => wgpu::BlendFactor::Dst,
        f if f == ffi::NVG_ONE_MINUS_DST_COLOR => wgpu::BlendFactor::OneMinusDst,
        f if f == ffi::NVG_SRC_ALPHA => wgpu::BlendFactor::SrcAlpha,
        f if f == ffi::NVG_ONE_MINUS_SRC_ALPHA => wgpu::BlendFactor::OneMinusSrcAlpha,
        f if f == ffi::NVG_DST_ALPHA => wgpu::BlendFactor::DstAlpha,
        f if f == ffi::NVG_ONE_MINUS_DST_ALPHA => wgpu::BlendFactor::OneMinusDstAlpha,
        f if f == ffi::NVG_SRC_ALPHA_SATURATE => wgpu::BlendFactor::SrcAlphaSaturated,
        _ => wgpu::BlendFactor::One, // fallback
    }
}

fn convert_blend(op: &ffi::NVGcompositeOperationState) -> BlendState {
    BlendState {
        src_rgb: convert_blend_factor(op.src_rgb),
        dst_rgb: convert_blend_factor(op.dst_rgb),
        src_alpha: convert_blend_factor(op.src_alpha),
        dst_alpha: convert_blend_factor(op.dst_alpha),
    }
}
```

- [ ] **Step 3: Implement render_triangles**

This is the simplest draw call — direct triangle rendering (used for text):

```rust
unsafe extern "C" fn render_triangles(
    uptr: *mut std::ffi::c_void,
    paint: *mut ffi::NVGpaint,
    composite_op: ffi::NVGcompositeOperationState,
    scissor: *mut ffi::NVGscissor,
    verts: *const ffi::NVGvertex,
    nverts: i32,
    fringe: f32,
) {
    let ctx = unsafe { get_ctx(uptr) };
    let paint = unsafe { &*paint };
    let scissor = unsafe { &*scissor };

    let triangle_offset = ctx.vertices.len();
    let verts_slice = unsafe { std::slice::from_raw_parts(verts, nverts as usize) };
    ctx.vertices.extend_from_slice(verts_slice);

    let mut frag = ctx.convert_paint(paint, scissor, 1.0, fringe, -1.0);
    frag.shader_type = SHADER_IMG;
    let uniform_offset = ctx.uniforms.len();
    ctx.uniforms.push(frag);

    ctx.calls.push(DrawCall {
        call_type: CallType::Triangles,
        image: paint.image,
        path_offset: 0,
        path_count: 0,
        triangle_offset,
        triangle_count: nverts as usize,
        uniform_offset,
        blend: convert_blend(&composite_op),
    });
}
```

- [ ] **Step 4: Implement render_fill**

Batch fill commands. Detect convex vs non-convex paths:

```rust
unsafe extern "C" fn render_fill(
    uptr: *mut std::ffi::c_void,
    paint: *mut ffi::NVGpaint,
    composite_op: ffi::NVGcompositeOperationState,
    scissor: *mut ffi::NVGscissor,
    fringe: f32,
    bounds: *const f32,
    paths: *const ffi::NVGpath,
    npaths: i32,
) {
    let ctx = unsafe { get_ctx(uptr) };
    let paint = unsafe { &*paint };
    let scissor = unsafe { &*scissor };
    let paths_slice = unsafe { std::slice::from_raw_parts(paths, npaths as usize) };
    let bounds = unsafe { std::slice::from_raw_parts(bounds, 4) };

    let is_convex = npaths == 1 && paths_slice[0].convex != 0;

    let path_offset = ctx.paths.len();

    // Copy vertices and build path data
    for path in paths_slice {
        let mut pd = PathData {
            fill_offset: 0, fill_count: 0,
            stroke_offset: 0, stroke_count: 0,
        };
        if path.nfill > 0 {
            let fill_verts = unsafe { std::slice::from_raw_parts(path.fill, path.nfill as usize) };
            if is_convex {
                // Convert triangle fan to triangle list for convex fills
                pd.fill_offset = ctx.vertices.len();
                if fill_verts.len() >= 3 {
                    for i in 2..fill_verts.len() {
                        ctx.vertices.push(fill_verts[0]);
                        ctx.vertices.push(fill_verts[i - 1]);
                        ctx.vertices.push(fill_verts[i]);
                    }
                }
                pd.fill_count = if fill_verts.len() >= 3 { (fill_verts.len() - 2) * 3 } else { 0 };
            } else {
                // For stencil fill, also convert fan to list
                pd.fill_offset = ctx.vertices.len();
                if fill_verts.len() >= 3 {
                    for i in 2..fill_verts.len() {
                        ctx.vertices.push(fill_verts[0]);
                        ctx.vertices.push(fill_verts[i - 1]);
                        ctx.vertices.push(fill_verts[i]);
                    }
                }
                pd.fill_count = if fill_verts.len() >= 3 { (fill_verts.len() - 2) * 3 } else { 0 };
            }
        }
        if path.nstroke > 0 {
            let stroke_verts = unsafe { std::slice::from_raw_parts(path.stroke, path.nstroke as usize) };
            pd.stroke_offset = ctx.vertices.len();
            pd.stroke_count = stroke_verts.len();
            ctx.vertices.extend_from_slice(stroke_verts);
        }
        ctx.paths.push(pd);
    }

    let mut triangle_offset = 0;
    let mut triangle_count = 0;
    let uniform_offset = ctx.uniforms.len();

    if is_convex {
        // Single uniform for convex fill
        let frag = ctx.convert_paint(paint, scissor, fringe, fringe, -1.0);
        ctx.uniforms.push(frag);
    } else {
        // Two uniforms: stencil (simple shader) + fill
        // Bounding quad for the cover pass
        triangle_offset = ctx.vertices.len();
        triangle_count = 4;
        // Triangle strip as 2 triangles: [br, tr, bl, tl] -> triangles
        ctx.vertices.push(ffi::NVGvertex { x: bounds[2], y: bounds[3], u: 0.5, v: 1.0 });
        ctx.vertices.push(ffi::NVGvertex { x: bounds[2], y: bounds[1], u: 0.5, v: 1.0 });
        ctx.vertices.push(ffi::NVGvertex { x: bounds[0], y: bounds[3], u: 0.5, v: 1.0 });
        ctx.vertices.push(ffi::NVGvertex { x: bounds[0], y: bounds[1], u: 0.5, v: 1.0 });

        // Stencil uniform (simple shader)
        let mut stencil_frag = FragUniforms::default();
        stencil_frag.stroke_thr = -1.0;
        stencil_frag.shader_type = SHADER_SIMPLE;
        ctx.uniforms.push(stencil_frag);

        // Fill uniform
        let fill_frag = ctx.convert_paint(paint, scissor, fringe, fringe, -1.0);
        ctx.uniforms.push(fill_frag);
    }

    ctx.calls.push(DrawCall {
        call_type: if is_convex { CallType::ConvexFill } else { CallType::Fill },
        image: paint.image,
        path_offset,
        path_count: npaths as usize,
        triangle_offset,
        triangle_count,
        uniform_offset,
        blend: convert_blend(&composite_op),
    });
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cd /home/philpax/programming/Cardinal/cardinal-rs && cargo check -p cardinal-core 2>&1 | tail -20`

- [ ] **Step 6: Commit**

```bash
git add cardinal-rs/crates/cardinal-core/src/nanovg_wgpu.rs
git commit -m "feat: implement renderTriangles and renderFill (convex + stencil) batching"
```

---

### Task 6: Implement renderStroke

Batch stroke draw calls, handling both stencil-strokes and simple strokes.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-core/src/nanovg_wgpu.rs`

- [ ] **Step 1: Implement render_stroke**

```rust
unsafe extern "C" fn render_stroke(
    uptr: *mut std::ffi::c_void,
    paint: *mut ffi::NVGpaint,
    composite_op: ffi::NVGcompositeOperationState,
    scissor: *mut ffi::NVGscissor,
    fringe: f32,
    stroke_width: f32,
    paths: *const ffi::NVGpath,
    npaths: i32,
) {
    let ctx = unsafe { get_ctx(uptr) };
    let paint = unsafe { &*paint };
    let scissor = unsafe { &*scissor };
    let paths_slice = unsafe { std::slice::from_raw_parts(paths, npaths as usize) };

    let path_offset = ctx.paths.len();

    for path in paths_slice {
        let mut pd = PathData {
            fill_offset: 0, fill_count: 0,
            stroke_offset: 0, stroke_count: 0,
        };
        if path.nstroke > 0 {
            let stroke_verts = unsafe { std::slice::from_raw_parts(path.stroke, path.nstroke as usize) };
            pd.stroke_offset = ctx.vertices.len();
            pd.stroke_count = stroke_verts.len();
            ctx.vertices.extend_from_slice(stroke_verts);
        }
        ctx.paths.push(pd);
    }

    let uniform_offset = ctx.uniforms.len();

    if ctx.stencil_strokes {
        // Two uniforms: stroke + AA pass
        let frag1 = ctx.convert_paint(paint, scissor, stroke_width, fringe, -1.0);
        ctx.uniforms.push(frag1);
        let frag2 = ctx.convert_paint(paint, scissor, stroke_width, fringe, 1.0 - 0.5 / 255.0);
        ctx.uniforms.push(frag2);
    } else {
        let frag = ctx.convert_paint(paint, scissor, stroke_width, fringe, -1.0);
        ctx.uniforms.push(frag);
    }

    ctx.calls.push(DrawCall {
        call_type: CallType::Stroke,
        image: paint.image,
        path_offset,
        path_count: npaths as usize,
        triangle_offset: 0,
        triangle_count: 0,
        uniform_offset,
        blend: convert_blend(&composite_op),
    });
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /home/philpax/programming/Cardinal/cardinal-rs && cargo check -p cardinal-core 2>&1 | tail -20`

- [ ] **Step 3: Commit**

```bash
git add cardinal-rs/crates/cardinal-core/src/nanovg_wgpu.rs
git commit -m "feat: implement renderStroke batching for NanoVG wgpu backend"
```

---

### Task 7: Implement renderFlush

The core of the rendering — execute all batched draw calls by encoding wgpu render passes.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-core/src/nanovg_wgpu.rs`

- [ ] **Step 1: Add render target tracking to WgpuNvgContext**

Add fields to track the current render target:

```rust
// Current render target (set before nvgBeginFrame)
render_target_view: Option<wgpu::TextureView>,
render_target_stencil: Option<wgpu::Texture>,
render_target_stencil_view: Option<wgpu::TextureView>,
```

Add a public method to set the render target:

```rust
/// Set the render target texture before calling nvgBeginFrame.
/// The stencil texture is created lazily on first use.
pub fn set_render_target(ctx_ptr: *mut ffi::NVGcontext, target_view: wgpu::TextureView, width: u32, height: u32) {
    // Access the WgpuNvgContext via nvgInternalParams
    let params = unsafe { ffi::nvgInternalParams(ctx_ptr) };
    let ctx = unsafe { get_ctx((*params).user_ptr) };
    ctx.render_target_view = Some(target_view);

    // Create or recreate stencil texture if size changed
    let needs_stencil = ctx.render_target_stencil.as_ref().map_or(true, |t| {
        t.width() != width || t.height() != height
    });
    if needs_stencil {
        let stencil_tex = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("nvg_stencil"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Stencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let stencil_view = stencil_tex.create_view(&Default::default());
        ctx.render_target_stencil = Some(stencil_tex);
        ctx.render_target_stencil_view = Some(stencil_view);
    }
}
```

We also need to expose `nvgInternalParams` in the FFI:
```rust
// In ffi.rs:
unsafe extern "C" {
    pub fn nvgInternalParams(ctx: *mut NVGcontext) -> *mut NVGparams;
}
```

- [ ] **Step 2: Implement render_flush**

This is the longest function. It uploads vertices, creates per-draw uniform buffers/bind groups, and encodes render passes:

```rust
unsafe extern "C" fn render_flush(uptr: *mut std::ffi::c_void) {
    let ctx = unsafe { get_ctx(uptr) };

    if ctx.calls.is_empty() {
        ctx.vertices.clear();
        ctx.paths.clear();
        ctx.uniforms.clear();
        return;
    }

    let Some(target_view) = ctx.render_target_view.as_ref() else {
        eprintln!("nanovg_wgpu: flush with no render target");
        ctx.calls.clear();
        ctx.paths.clear();
        ctx.vertices.clear();
        ctx.uniforms.clear();
        return;
    };

    // Upload vertex data
    let vertex_data = bytemuck_cast_slice(&ctx.vertices);
    // Ensure vertex buffer is large enough
    let needed = vertex_data.len() as u64;
    if needed > 0 {
        let vb = ctx.vertex_buffer.get_or_insert_with(|| {
            ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("nvg_vertices"),
                size: needed.max(4096),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        if vb.size() < needed {
            *vb = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("nvg_vertices"),
                size: needed * 2, // grow 2x
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        ctx.queue.write_buffer(vb, 0, vertex_data);
    }

    let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("nvg_flush"),
    });

    // For each draw call, create a uniform buffer + bind group and encode the draw
    for call in &ctx.calls {
        // Create frag uniform buffer for this call
        let frag = &ctx.uniforms[call.uniform_offset];
        let frag_bytes: &[u8] = bytemuck_cast(frag);
        let frag_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("nvg_frag_uniform"),
            contents: frag_bytes,
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create bind group 0 (view + frag uniforms)
        let bind_group_0 = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nvg_bg0"),
            layout: ctx.view_bind_group_layout.as_ref().unwrap(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ctx.view_uniform_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: frag_buffer.as_entire_binding(),
                },
            ],
        });

        // Get texture bind group (group 1)
        let tex_bg = ctx.get_texture_bind_group(call.image);

        match call.call_type {
            CallType::ConvexFill => {
                // Encode convex fill render pass
                // ... (render pass with convex fill pipeline)
            }
            CallType::Fill => {
                // Encode stencil-then-cover fill
                // Pass 1: draw to stencil
                // Pass 2: AA fringe
                // Pass 3: cover (quad)
            }
            CallType::Stroke => {
                // Encode stroke
            }
            CallType::Triangles => {
                // Encode triangles
            }
        }
    }

    ctx.queue.submit(std::iter::once(encoder.finish()));

    // Reset per-frame state
    ctx.calls.clear();
    ctx.paths.clear();
    ctx.vertices.clear();
    ctx.uniforms.clear();
}
```

Note: `bytemuck_cast_slice` and `bytemuck_cast` should use the `bytemuck` crate or be implemented as unsafe pointer casts for `#[repr(C)]` structs.

The actual render pass encoding for each call type follows the GL2 backend logic:

**ConvexFill**: Single render pass with `pipeline_convex_fill`. For each path, draw the fill triangles, then the stroke triangles (AA fringe).

**Fill (stencil-then-cover)**: Requires the stencil attachment.
1. Render pass with `pipeline_stencil_fill_draw_stencil` — draw all path fill triangles to stencil (no color write)
2. If edge AA, same pass continues with `pipeline_stencil_fill_draw_aa` — draw stroke triangles (AA fringe) where stencil == 0
3. Render pass with `pipeline_stencil_fill_cover` — draw bounding quad where stencil != 0, reset stencil to 0

**Stroke**: Depends on `stencil_strokes` flag. Without stencil: single pass with `pipeline_stroke`. With stencil: three sub-passes (draw, AA, clear).

**Triangles**: Single render pass with `pipeline_triangles`, draw the triangle array.

Each render pass needs the view uniform updated. Upload `ctx.view` to `view_uniform_buffer` once at the start of flush.

This is a large function. The implementation should closely follow `glnvg__renderFlush` in `nanovg_gl.h` lines 1200-1288, with the GL state management replaced by wgpu pipeline selection.

Important detail: wgpu doesn't support changing pipelines within a single render pass without ending and starting a new one. However, each pipeline that differs only in blend state will need separate passes. An optimization is to use a single render pass where the pipeline doesn't change between consecutive calls.

For the initial implementation, use one render pass per draw call. This is simpler and correct, even if not optimal. Performance can be improved later by batching compatible calls into a single render pass.

- [ ] **Step 3: Add bytemuck dependency**

Add `bytemuck = { version = "1", features = ["derive"] }` to `cardinal-core/Cargo.toml` and add `#[derive(bytemuck::Pod, bytemuck::Zeroable)]` to `FragUniforms` and `NVGvertex` wrapper.

Alternatively, use `wgpu::util::BufferInitDescriptor` with manual byte slicing via `unsafe { std::slice::from_raw_parts(...) }`.

- [ ] **Step 4: Verify it compiles**

Run: `cd /home/philpax/programming/Cardinal/cardinal-rs && cargo check -p cardinal-core 2>&1 | tail -20`

- [ ] **Step 5: Commit**

```bash
git add cardinal-rs/crates/cardinal-core/src/nanovg_wgpu.rs cardinal-rs/crates/cardinal-core/Cargo.toml
git commit -m "feat: implement renderFlush — encode wgpu render passes for all NanoVG draw calls"
```

---

### Task 8: Bridge Changes — Minimal C++ Render Function

Strip the EGL/GL code from bridge.cpp and make the render function accept an NVGcontext from Rust.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-core/cpp/bridge.cpp`
- Modify: `cardinal-rs/crates/cardinal-core/cpp/bridge.h`
- Modify: `cardinal-rs/crates/cardinal-core/src/ffi.rs`
- Modify: `cardinal-rs/crates/cardinal-core/src/lib.rs`

- [ ] **Step 1: Update bridge.h with new render signature**

Replace the old render function declaration:

```c
// Old:
int cardinal_module_render(ModuleHandle h,
                           unsigned char* pixels, int max_width, int max_height,
                           int* out_width, int* out_height);

// New:
/// Render a module widget using the provided NanoVG context.
/// Calls widget->draw() and widget->drawLayer(). The NanoVG frame
/// (nvgBeginFrame/nvgEndFrame) must be managed by the caller.
/// Returns 1 on success, 0 on failure.
int cardinal_module_render(ModuleHandle h, NVGcontext* vg, int width, int height);
```

Remove the render context claims:
```c
// Remove these:
int cardinal_render_claim_context(void);
void cardinal_render_release_context(void);
```

Add forward declaration for NVGcontext at top of bridge.h (inside the extern "C" block):
```c
typedef struct NVGcontext NVGcontext;
```

Also add a new function to set the NanoVG contexts on the Window object:
```c
/// Set the NanoVG contexts used by plugin widgets.
/// Call after creating the NanoVG context but before creating modules.
void cardinal_set_vg(NVGcontext* vg, NVGcontext* fb_vg);
```

- [ ] **Step 2: Update bridge.cpp**

Remove:
- The entire `#include <EGL/egl.h>`, `#include <GL/gl.h>`, GLEW declarations
- The `g_eglDisplay`, `g_eglContext`, `g_eglSurface` globals
- The `g_vg`, `g_fbVg` globals (NanoVG context now managed by Rust)
- The entire `initEGL()` function
- The entire `shutdownEGL()` function
- The entire `initNanoVG()` function
- The `#define NANOVG_GL2` and `#include <nanovg_gl.h>`
- `nvgDeleteGL2` calls in `cardinal_shutdown`
- `cardinal_render_claim_context` and `cardinal_render_release_context`

Add `cardinal_set_vg`:
```cpp
static NVGcontext* g_vg = nullptr;
static NVGcontext* g_fbVg = nullptr;

void cardinal_set_vg(NVGcontext* vg, NVGcontext* fb_vg) {
    g_vg = vg;
    g_fbVg = fb_vg;
    if (g_context && g_context->window) {
        g_context->window->vg = vg;
        g_context->window->fbVg = fb_vg;
    }
}
```

Update `cardinal_init`:
- Remove the EGL init block and NanoVG init block
- Keep `rack::settings::headless = false` — we'll set it to true from Rust if no GPU context is available, or leave it false since wgpu should always work
- Remove the headless fallback logic (EGL/NanoVG failure paths)

Update `cardinal_shutdown`:
- Remove `nvgDeleteGL2` calls — Rust owns the NanoVG context lifetime
- Remove `shutdownEGL()` call
- Keep the `g_context->window->vg = nullptr` / `fbVg = nullptr` cleanup

Rewrite `cardinal_module_render`:
```cpp
int cardinal_module_render(ModuleHandle h, NVGcontext* vg, int width, int height) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return 0;
    if (!it->second.widget) return 0;
    if (!vg) return 0;

    auto* widget = it->second.widget;

    rack::widget::Widget::DrawArgs args;
    args.vg = vg;
    args.clipBox = rack::math::Rect(
        rack::math::Vec(0, 0),
        rack::math::Vec(width, height)
    );
    args.fb = nullptr;

    widget->draw(args);
    widget->drawLayer(args, 1);

    return 1;
}
```

- [ ] **Step 3: Update ffi.rs**

Update the render function signature:
```rust
// Old:
pub fn cardinal_module_render(h: i64, pixels: *mut u8, max_width: i32, max_height: i32, out_width: *mut i32, out_height: *mut i32) -> i32;

// New:
pub fn cardinal_module_render(h: i64, vg: *mut NVGcontext, width: i32, height: i32) -> i32;

// Add:
pub fn cardinal_set_vg(vg: *mut NVGcontext, fb_vg: *mut NVGcontext);
```

Remove:
```rust
pub fn cardinal_render_claim_context() -> i32;
pub fn cardinal_render_release_context();
```

- [ ] **Step 4: Update lib.rs**

Rewrite `module_render` to use the new signature. For now it's a thin wrapper — the actual render target management happens in the egui app:

```rust
/// Render a module widget using the provided NanoVG context.
/// The caller must manage nvgBeginFrame/nvgEndFrame and the render target.
/// Returns true on success.
pub fn module_render(id: ModuleId, vg: *mut ffi::NVGcontext, width: i32, height: i32) -> bool {
    unsafe { ffi::cardinal_module_render(id.0, vg, width, height) != 0 }
}

/// Set the NanoVG contexts on the Window object.
/// Must be called after init() but before creating modules.
pub fn set_vg(vg: *mut ffi::NVGcontext, fb_vg: *mut ffi::NVGcontext) {
    unsafe { ffi::cardinal_set_vg(vg, fb_vg) }
}
```

Remove `render_claim_context` and `render_release_context`.

- [ ] **Step 5: Verify it compiles**

Run: `cd /home/philpax/programming/Cardinal/cardinal-rs && cargo check -p cardinal-core 2>&1 | tail -20`

- [ ] **Step 6: Commit**

```bash
git add cardinal-rs/crates/cardinal-core/cpp/bridge.cpp cardinal-rs/crates/cardinal-core/cpp/bridge.h cardinal-rs/crates/cardinal-core/src/ffi.rs cardinal-rs/crates/cardinal-core/src/lib.rs
git commit -m "feat: strip EGL/GL from bridge, make render function accept NVGcontext from Rust"
```

---

### Task 9: Build System — Remove GL Dependencies

Remove GL/GLEW/EGL from build.rs, Cargo deps, and system library linking. Remove the nanovg_gl_impl compilation.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-core/build.rs`
- Delete: `cardinal-rs/crates/cardinal-core/cpp/nanovg_gl_impl.cpp`

- [ ] **Step 1: Update build.rs**

Remove `build_gl_impls` function entirely.

Remove the call to `build_gl_impls(&include_dirs);` from `main()`.

Remove GL/GLEW/EGL from the system libraries list:
```rust
// Old:
for lib in &["jansson", "archive", "samplerate", "speexdsp", "pthread", "dl", "GL", "GLEW", "EGL"] {

// New:
for lib in &["jansson", "archive", "samplerate", "speexdsp", "pthread", "dl"] {
```

Remove the GLEW pkg-config probe from `build_bridge`:
```rust
// Remove:
if let Ok(glew) = pkg_config::probe_library("glew") {
    for path in &glew.include_paths {
        build.include(path);
    }
}
```

- [ ] **Step 2: Delete nanovg_gl_impl.cpp**

```bash
rm cardinal-rs/crates/cardinal-core/cpp/nanovg_gl_impl.cpp
```

- [ ] **Step 3: Verify it compiles**

Run: `cd /home/philpax/programming/Cardinal/cardinal-rs && cargo check -p cardinal-core 2>&1 | tail -20`

There may be link errors if bridge.cpp still references GL symbols. Check and fix any remaining GL references in bridge.cpp or stubs.cpp.

- [ ] **Step 4: Commit**

```bash
git add cardinal-rs/crates/cardinal-core/build.rs
git rm cardinal-rs/crates/cardinal-core/cpp/nanovg_gl_impl.cpp
git commit -m "feat: remove GL/GLEW/EGL from build system, delete nanovg_gl_impl.cpp"
```

---

### Task 10: Integrate Rendering in egui App

Wire up the full rendering pipeline: Cardinal thread creates wgpu render target textures, calls NanoVG to render widgets, and sends textures back to egui for zero-copy display.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-egui/src/main.rs`
- Modify: `cardinal-rs/crates/cardinal-core/src/nanovg_wgpu.rs` (add public helpers)
- Modify: `cardinal-rs/crates/cardinal-core/src/lib.rs` (re-export what egui needs)

- [ ] **Step 1: Update RenderResult to carry wgpu::Texture**

```rust
struct RenderResult {
    module_id: ModuleId,
    width: u32,
    height: u32,
    texture: wgpu::Texture,
}
```

- [ ] **Step 2: Update Cardinal thread render handling**

In the Cardinal thread, after receiving `InitGpu`, store the device, queue, and NVGcontext. When handling `RenderModule`:

```rust
Command::RenderModule { module_id, width, height } => {
    if nanovg_ctx.is_null() { continue; }
    let device = gpu_device.as_ref().unwrap();
    let queue = gpu_queue.as_ref().unwrap();

    let w = width as u32;
    let h = height as u32;

    // Create render target texture
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("nvg_render_target"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());

    // Set render target and render
    cardinal_core::nanovg_wgpu::set_render_target(nanovg_ctx, view, w, h);

    unsafe {
        cardinal_core::ffi::nvgBeginFrame(nanovg_ctx, w as f32, h as f32, 1.0);
    }
    cc::module_render(module_id, nanovg_ctx, w as i32, h as i32);
    unsafe {
        cardinal_core::ffi::nvgEndFrame(nanovg_ctx);
    }

    let _ = render_tx.send(RenderResult {
        module_id,
        width: w,
        height: h,
        texture,
    });
}
```

- [ ] **Step 3: Update poll_render_results for zero-copy textures**

Replace the pixel-based texture upload with native wgpu texture registration:

```rust
fn poll_render_results(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    let Some(render_state) = frame.wgpu_render_state() else { return };

    while let Ok(result) = self.render_rx.try_recv() {
        if let Some(m) = self.modules.iter_mut().find(|m| m.id == result.module_id) {
            let view = result.texture.create_view(&wgpu::TextureViewDescriptor::default());

            // Register or update the native wgpu texture with egui
            let mut renderer = render_state.renderer.write();
            if let Some(tex_id) = m.texture_id {
                renderer.update_egui_texture_from_wgpu_texture(
                    &render_state.device,
                    &view,
                    wgpu::FilterMode::Linear,
                    tex_id,
                );
            } else {
                let tex_id = renderer.register_native_texture(
                    &render_state.device,
                    &view,
                    wgpu::FilterMode::Linear,
                );
                m.texture_id = Some(tex_id);
            }

            // Keep the texture alive (egui only holds a view)
            m.render_texture = Some(result.texture);
        }
    }
}
```

Update `PlacedModule` to hold:
```rust
texture_id: Option<egui::TextureId>,
render_texture: Option<wgpu::Texture>,  // keeps the texture alive
```

Remove the old `texture: Option<egui::TextureHandle>` field.

- [ ] **Step 4: Update painting to use texture_id**

In the module painting code, replace `tex.id()` with the stored `TextureId`:

```rust
if let Some(tex_id) = m.texture_id {
    painter.image(
        tex_id,
        mr,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}
```

- [ ] **Step 5: Update the update method signature**

The `poll_render_results` now needs `frame`, so pass it:

```rust
fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    self.poll_render_results(ctx, frame);
    // ...
}
```

- [ ] **Step 6: Call cardinal_set_vg in the Cardinal thread**

After creating the NanoVG context, set it on the Window:

```rust
Command::InitGpu { device, queue } => {
    gpu_device = Some(device.clone());
    gpu_queue = Some(queue.clone());
    nanovg_ctx = cardinal_core::nanovg_wgpu::create_context(
        device, queue,
        cardinal_core::ffi::NVG_ANTIALIAS | cardinal_core::ffi::NVG_STENCIL_STROKES,
    );
    // Set VG contexts on Window so plugin widgets can load fonts/images
    cc::set_vg(nanovg_ctx, nanovg_ctx);
    eprintln!("cardinal thread: wgpu NanoVG context created");
}
```

- [ ] **Step 7: Verify it compiles**

Run: `cd /home/philpax/programming/Cardinal/cardinal-rs && cargo check -p cardinal-egui 2>&1 | tail -20`

- [ ] **Step 8: Commit**

```bash
git add cardinal-rs/crates/cardinal-egui/src/main.rs cardinal-rs/crates/cardinal-core/src/nanovg_wgpu.rs cardinal-rs/crates/cardinal-core/src/lib.rs
git commit -m "feat: integrate wgpu NanoVG rendering with egui zero-copy texture sharing"
```

---

### Task 11: Update shell.nix

Remove OpenGL/EGL dependencies, ensure Vulkan is available for wgpu.

**Files:**
- Modify: `shell.nix`

- [ ] **Step 1: Update shell.nix**

Remove GL/GLEW/EGL packages, add Vulkan:

```nix
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

    # Vulkan (for wgpu backend)
    vulkan-loader
    vulkan-headers

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
```

- [ ] **Step 2: Verify nix-shell enters cleanly**

Run: `nix-shell --run "echo ok"`
Expected: `ok` (no missing dependency errors)

- [ ] **Step 3: Verify full build in nix-shell**

Run: `nix-shell --run "cd cardinal-rs && cargo check -p cardinal-egui"`
Expected: compiles without GL/GLEW/EGL link errors.

- [ ] **Step 4: Commit**

```bash
git add shell.nix
git commit -m "feat: update shell.nix — remove GL/EGL deps, add Vulkan for wgpu"
```

---

### Task 12: End-to-End Test

Verify the full pipeline works: app launches, modules render visually, audio still works.

**Files:** (no changes — this is a verification task)

- [ ] **Step 1: Build the app**

Run: `cd /home/philpax/programming/Cardinal/cardinal-rs && cargo build -p cardinal-egui 2>&1 | tail -30`
Expected: builds successfully.

- [ ] **Step 2: Run the app**

Run: `cd /home/philpax/programming/Cardinal/cardinal-rs && cargo run -p cardinal-egui`

Verify:
- Window opens
- Module browser shows plugins
- Clicking a module adds it to the rack
- Module widget renders visually (not gray rectangle)
- Audio still works (add VCO -> VCA -> Audio, hear sound)
- Console shows "nanovg_wgpu" messages (not EGL/GL messages)

- [ ] **Step 3: Fix any rendering issues**

Common issues to check:
- Textures appearing black — check texture format, upload data
- Upside-down rendering — wgpu coordinate system is top-down (unlike GL), should be correct
- Missing stencil effects — verify stencil pipeline configuration
- Alpha blending wrong — check premultiplied alpha handling

- [ ] **Step 4: Final commit if any fixes were needed**

```bash
git add -A
git commit -m "fix: address rendering issues found during end-to-end testing"
```
