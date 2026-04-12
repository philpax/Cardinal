//! Skeleton wgpu backend for NanoVG.
//!
//! Provides the 12 `NVGparams` callbacks (currently stubs) and
//! `create_context` / `destroy_context` entry points.

use std::collections::HashMap;
use std::ffi::c_int;
use std::ffi::c_void;
use std::sync::Arc;

use crate::ffi;

// ── Supporting types ─────────────────────────────────────────────────

/// Metadata for a texture stored in the registry.
pub struct TextureEntry {
    pub width: i32,
    pub height: i32,
    pub tex_type: i32,
    pub flags: i32,
    // Will hold a wgpu::Texture + view in later tasks.
}

/// The kind of draw call batched for flush.
#[derive(Debug, Clone, Copy)]
pub enum CallType {
    Fill,
    Stroke,
    Triangles,
}

/// Per-draw blend state (maps from NVGcompositeOperationState).
#[derive(Debug, Clone, Copy, Default)]
pub struct BlendState {
    pub src_rgb: i32,
    pub dst_rgb: i32,
    pub src_alpha: i32,
    pub dst_alpha: i32,
}

/// Cached path vertex data for a draw call.
#[derive(Debug, Clone, Default)]
pub struct PathData {
    pub fill_offset: u32,
    pub fill_count: u32,
    pub stroke_offset: u32,
    pub stroke_count: u32,
}

/// Fragment-shader uniform block (placeholder).
#[derive(Debug, Clone, Copy, Default)]
pub struct FragUniforms {
    // Will be filled in Task 3 / Task 5.
    pub _pad: [f32; 4],
}

/// A single batched draw call.
#[derive(Debug, Clone)]
pub struct DrawCall {
    pub call_type: CallType,
    pub blend: BlendState,
    pub image: i32,
    pub paths: Vec<PathData>,
    pub triangle_offset: u32,
    pub triangle_count: u32,
    pub uniform_offset: u32,
    pub fringe: f32,
    pub stroke_width: f32,
}

// ── Main context ─────────────────────────────────────────────────────

pub struct WgpuNvgContext {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub view_width: f32,
    pub view_height: f32,
    pub device_pixel_ratio: f32,
    pub flags: c_int,
    pub textures: HashMap<i32, TextureEntry>,
    pub next_texture_id: i32,
    pub draw_calls: Vec<DrawCall>,
    pub vertices: Vec<ffi::NVGvertex>,
    pub uniforms: Vec<FragUniforms>,
}

impl WgpuNvgContext {
    fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, flags: c_int) -> Self {
        Self {
            device,
            queue,
            view_width: 0.0,
            view_height: 0.0,
            device_pixel_ratio: 1.0,
            flags,
            textures: HashMap::new(),
            next_texture_id: 1,
            draw_calls: Vec::new(),
            vertices: Vec::new(),
            uniforms: Vec::new(),
        }
    }
}

// ── Helper to recover &mut WgpuNvgContext from the void* ─────────────

unsafe fn ctx_from_uptr<'a>(uptr: *mut c_void) -> &'a mut WgpuNvgContext {
    unsafe { &mut *(uptr as *mut WgpuNvgContext) }
}

// ── Callback stubs ───────────────────────────────────────────────────

unsafe extern "C" fn render_create(_uptr: *mut c_void, _other_uptr: *mut c_void) -> c_int {
    eprintln!("nanovg-wgpu: renderCreate (stub)");
    1 // success
}

unsafe extern "C" fn render_create_texture(
    uptr: *mut c_void,
    tex_type: c_int,
    w: c_int,
    h: c_int,
    image_flags: c_int,
    _data: *const u8,
) -> c_int {
    let ctx = unsafe { ctx_from_uptr(uptr) };
    let id = ctx.next_texture_id;
    ctx.next_texture_id += 1;
    ctx.textures.insert(
        id,
        TextureEntry {
            width: w,
            height: h,
            tex_type,
            flags: image_flags,
        },
    );
    eprintln!("nanovg-wgpu: renderCreateTexture (stub) id={id} {w}x{h}");
    id
}

unsafe extern "C" fn render_delete_texture(uptr: *mut c_void, image: c_int) -> c_int {
    let ctx = unsafe { ctx_from_uptr(uptr) };
    if ctx.textures.remove(&image).is_some() {
        eprintln!("nanovg-wgpu: renderDeleteTexture (stub) id={image}");
        1
    } else {
        0
    }
}

unsafe extern "C" fn render_update_texture(
    _uptr: *mut c_void,
    image: c_int,
    _x: c_int,
    _y: c_int,
    _w: c_int,
    _h: c_int,
    _data: *const u8,
) -> c_int {
    eprintln!("nanovg-wgpu: renderUpdateTexture (stub) id={image}");
    1
}

unsafe extern "C" fn render_get_texture_size(
    uptr: *mut c_void,
    image: c_int,
    w: *mut c_int,
    h: *mut c_int,
) -> c_int {
    let ctx = unsafe { ctx_from_uptr(uptr) };
    if let Some(entry) = ctx.textures.get(&image) {
        if !w.is_null() {
            unsafe { *w = entry.width };
        }
        if !h.is_null() {
            unsafe { *h = entry.height };
        }
        1
    } else {
        0
    }
}

unsafe extern "C" fn render_viewport(uptr: *mut c_void, width: f32, height: f32, device_pixel_ratio: f32) {
    let ctx = unsafe { ctx_from_uptr(uptr) };
    ctx.view_width = width;
    ctx.view_height = height;
    ctx.device_pixel_ratio = device_pixel_ratio;
    eprintln!("nanovg-wgpu: renderViewport {width}x{height} dpr={device_pixel_ratio}");
}

unsafe extern "C" fn render_cancel(uptr: *mut c_void) {
    let ctx = unsafe { ctx_from_uptr(uptr) };
    ctx.draw_calls.clear();
    ctx.vertices.clear();
    ctx.uniforms.clear();
    eprintln!("nanovg-wgpu: renderCancel (stub)");
}

unsafe extern "C" fn render_flush(_uptr: *mut c_void) {
    eprintln!("nanovg-wgpu: renderFlush (stub)");
}

unsafe extern "C" fn render_fill(
    _uptr: *mut c_void,
    _paint: *mut ffi::NVGpaint,
    _composite_operation: ffi::NVGcompositeOperationState,
    _scissor: *mut ffi::NVGscissor,
    _fringe: f32,
    _bounds: *const f32,
    _paths: *const ffi::NVGpath,
    _npaths: c_int,
) {
    eprintln!("nanovg-wgpu: renderFill (stub)");
}

unsafe extern "C" fn render_stroke(
    _uptr: *mut c_void,
    _paint: *mut ffi::NVGpaint,
    _composite_operation: ffi::NVGcompositeOperationState,
    _scissor: *mut ffi::NVGscissor,
    _fringe: f32,
    _stroke_width: f32,
    _paths: *const ffi::NVGpath,
    _npaths: c_int,
) {
    eprintln!("nanovg-wgpu: renderStroke (stub)");
}

unsafe extern "C" fn render_triangles(
    _uptr: *mut c_void,
    _paint: *mut ffi::NVGpaint,
    _composite_operation: ffi::NVGcompositeOperationState,
    _scissor: *mut ffi::NVGscissor,
    _verts: *const ffi::NVGvertex,
    _nverts: c_int,
    _fringe: f32,
) {
    eprintln!("nanovg-wgpu: renderTriangles (stub)");
}

unsafe extern "C" fn render_delete(uptr: *mut c_void) {
    eprintln!("nanovg-wgpu: renderDelete — freeing WgpuNvgContext");
    if !uptr.is_null() {
        let _ = unsafe { Box::from_raw(uptr as *mut WgpuNvgContext) };
    }
}

// ── Public API ───────────────────────────────────────────────────────

/// Create a new NanoVG context backed by wgpu stub callbacks.
///
/// `flags` should be a combination of `NVG_ANTIALIAS` and `NVG_STENCIL_STROKES`.
/// Returns a raw pointer to the `NVGcontext` (owned by NanoVG internals).
pub fn create_context(
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    flags: c_int,
) -> *mut ffi::NVGcontext {
    let ctx = Box::new(WgpuNvgContext::new(device, queue, flags));
    let user_ptr = Box::into_raw(ctx) as *mut c_void;

    let mut params = ffi::NVGparams {
        user_ptr,
        edge_anti_alias: if flags & ffi::NVG_ANTIALIAS != 0 { 1 } else { 0 },
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

    let nvg_ctx = unsafe { ffi::nvgCreateInternal(&mut params, std::ptr::null_mut()) };
    if nvg_ctx.is_null() {
        // nvgCreateInternal failed — reclaim the box so we don't leak.
        let _ = unsafe { Box::from_raw(user_ptr as *mut WgpuNvgContext) };
        eprintln!("nanovg-wgpu: nvgCreateInternal returned null!");
    }
    nvg_ctx
}

/// Destroy a NanoVG context previously created with `create_context`.
///
/// This calls `nvgDeleteInternal`, which in turn calls `render_delete`
/// to free the `WgpuNvgContext`.
pub fn destroy_context(ctx: *mut ffi::NVGcontext) {
    if !ctx.is_null() {
        unsafe { ffi::nvgDeleteInternal(ctx) };
    }
}
