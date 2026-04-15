//! Raw FFI bindings to the C bridge API.

use std::ffi::{c_char, c_int, c_uchar, c_void};

// ── NanoVG types ─────────────────────────────────────────────────────

// Creation flags (from nanovg_gl.h)
pub const NVG_ANTIALIAS: c_int = 1 << 0;
pub const NVG_STENCIL_STROKES: c_int = 1 << 1;

// Texture types
pub const NVG_TEXTURE_ALPHA: c_int = 0x01;
pub const NVG_TEXTURE_RGBA: c_int = 0x02;

// Image flags
pub const NVG_IMAGE_GENERATE_MIPMAPS: c_int = 1 << 0;
pub const NVG_IMAGE_REPEATX: c_int = 1 << 1;
pub const NVG_IMAGE_REPEATY: c_int = 1 << 2;
pub const NVG_IMAGE_FLIPY: c_int = 1 << 3;
pub const NVG_IMAGE_PREMULTIPLIED: c_int = 1 << 4;
pub const NVG_IMAGE_NEAREST: c_int = 1 << 5;

// Blend factors
pub const NVG_ZERO: c_int = 1 << 0;
pub const NVG_ONE: c_int = 1 << 1;
pub const NVG_SRC_COLOR: c_int = 1 << 2;
pub const NVG_ONE_MINUS_SRC_COLOR: c_int = 1 << 3;
pub const NVG_DST_COLOR: c_int = 1 << 4;
pub const NVG_ONE_MINUS_DST_COLOR: c_int = 1 << 5;
pub const NVG_SRC_ALPHA: c_int = 1 << 6;
pub const NVG_ONE_MINUS_SRC_ALPHA: c_int = 1 << 7;
pub const NVG_DST_ALPHA: c_int = 1 << 8;
pub const NVG_ONE_MINUS_DST_ALPHA: c_int = 1 << 9;
pub const NVG_SRC_ALPHA_SATURATE: c_int = 1 << 10;

/// Opaque NanoVG context (never instantiated on the Rust side).
#[repr(C)]
pub struct NVGcontext {
    _opaque: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct NVGcolor {
    pub rgba: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NVGpaint {
    pub xform: [f32; 6],
    pub extent: [f32; 2],
    pub radius: f32,
    pub feather: f32,
    pub inner_color: NVGcolor,
    pub outer_color: NVGcolor,
    pub image: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct NVGcompositeOperationState {
    pub src_rgb: c_int,
    pub dst_rgb: c_int,
    pub src_alpha: c_int,
    pub dst_alpha: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NVGscissor {
    pub xform: [f32; 6],
    pub extent: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct NVGvertex {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NVGpath {
    pub first: c_int,
    pub count: c_int,
    pub closed: c_uchar,
    pub nbevel: c_int,
    pub fill: *mut NVGvertex,
    pub nfill: c_int,
    pub stroke: *mut NVGvertex,
    pub nstroke: c_int,
    pub winding: c_int,
    pub convex: c_int,
}

#[repr(C)]
pub struct NVGparams {
    pub user_ptr: *mut c_void,
    pub edge_anti_alias: c_int,
    pub render_create: Option<unsafe extern "C" fn(uptr: *mut c_void, other_uptr: *mut c_void) -> c_int>,
    pub render_create_texture: Option<unsafe extern "C" fn(uptr: *mut c_void, typ: c_int, w: c_int, h: c_int, image_flags: c_int, data: *const c_uchar) -> c_int>,
    pub render_delete_texture: Option<unsafe extern "C" fn(uptr: *mut c_void, image: c_int) -> c_int>,
    pub render_update_texture: Option<unsafe extern "C" fn(uptr: *mut c_void, image: c_int, x: c_int, y: c_int, w: c_int, h: c_int, data: *const c_uchar) -> c_int>,
    pub render_get_texture_size: Option<unsafe extern "C" fn(uptr: *mut c_void, image: c_int, w: *mut c_int, h: *mut c_int) -> c_int>,
    pub render_viewport: Option<unsafe extern "C" fn(uptr: *mut c_void, width: f32, height: f32, device_pixel_ratio: f32)>,
    pub render_cancel: Option<unsafe extern "C" fn(uptr: *mut c_void)>,
    pub render_flush: Option<unsafe extern "C" fn(uptr: *mut c_void)>,
    pub render_fill: Option<unsafe extern "C" fn(uptr: *mut c_void, paint: *mut NVGpaint, composite_operation: NVGcompositeOperationState, scissor: *mut NVGscissor, fringe: f32, bounds: *const f32, paths: *const NVGpath, npaths: c_int)>,
    pub render_stroke: Option<unsafe extern "C" fn(uptr: *mut c_void, paint: *mut NVGpaint, composite_operation: NVGcompositeOperationState, scissor: *mut NVGscissor, fringe: f32, stroke_width: f32, paths: *const NVGpath, npaths: c_int)>,
    pub render_triangles: Option<unsafe extern "C" fn(uptr: *mut c_void, paint: *mut NVGpaint, composite_operation: NVGcompositeOperationState, scissor: *mut NVGscissor, verts: *const NVGvertex, nverts: c_int, fringe: f32)>,
    pub render_delete: Option<unsafe extern "C" fn(uptr: *mut c_void)>,
}

#[repr(C)]
#[derive(Clone)]
pub struct PortInfo {
    pub port_id: i32,
    pub name: *const c_char,
    pub x: f32,
    pub y: f32,
}

impl Default for PortInfo {
    fn default() -> Self {
        Self { port_id: 0, name: std::ptr::null(), x: 0.0, y: 0.0 }
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct ParamInfo {
    pub param_id: i32,
    pub name: *const c_char,
    pub min_value: f32,
    pub max_value: f32,
    pub default_value: f32,
    pub value: f32,
    pub x: f32,
    pub y: f32,
}

impl Default for ParamInfo {
    fn default() -> Self {
        Self {
            param_id: 0, name: std::ptr::null(),
            min_value: 0.0, max_value: 1.0, default_value: 0.0, value: 0.0,
            x: 0.0, y: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct ModuleCatalogEntry {
    pub plugin_slug: *const c_char,
    pub model_slug: *const c_char,
    pub model_name: *const c_char,
}

impl Default for ModuleCatalogEntry {
    fn default() -> Self {
        Self {
            plugin_slug: std::ptr::null(),
            model_slug: std::ptr::null(),
            model_name: std::ptr::null(),
        }
    }
}

unsafe extern "C" {
    pub fn cardinal_init(sample_rate: f32, resource_dir: *const c_char) -> i32;
    pub fn cardinal_shutdown();

    pub fn cardinal_catalog_count() -> i32;
    pub fn cardinal_catalog_list(out: *mut ModuleCatalogEntry, max_entries: i32) -> i32;

    pub fn cardinal_module_create(plugin_slug: *const c_char, model_slug: *const c_char) -> i64;
    pub fn cardinal_module_destroy(h: i64);
    pub fn cardinal_module_get_size(h: i64, width: *mut f32, height: *mut f32);
    pub fn cardinal_module_get_inputs(h: i64, out: *mut PortInfo, max: i32) -> i32;
    pub fn cardinal_module_get_outputs(h: i64, out: *mut PortInfo, max: i32) -> i32;
    pub fn cardinal_module_get_params(h: i64, out: *mut ParamInfo, max: i32) -> i32;
    pub fn cardinal_module_get_param(h: i64, param_id: i32) -> f32;
    pub fn cardinal_module_set_param(h: i64, param_id: i32, value: f32);
    pub fn cardinal_module_get_input_voltage(h: i64, port_id: i32) -> f32;
    pub fn cardinal_module_get_output_voltage(h: i64, port_id: i32) -> f32;

    pub fn cardinal_cable_create(out_module: i64, out_port: i32, in_module: i64, in_port: i32) -> i64;
    pub fn cardinal_cable_destroy(h: i64);

    pub fn cardinal_module_render(h: i64, vg: *mut NVGcontext, width: i32, height: i32) -> i32;

    pub fn cardinal_module_event(
        h: i64, event_type: c_int,
        x: f32, y: f32,
        button: c_int, action: c_int, mods: c_int,
        scroll_x: f32, scroll_y: f32,
    ) -> c_int;

    pub fn cardinal_module_check_port_drag(
        h: i64, port_id: *mut c_int, is_output: *mut c_int,
    ) -> c_int;

    pub fn cardinal_set_vg(vg: *mut NVGcontext, fb_vg: *mut NVGcontext);

    pub fn cardinal_set_incomplete_cable(h: i64, port_id: c_int, is_output: c_int);
    pub fn cardinal_clear_incomplete_cable();

    pub fn cardinal_audio_create() -> i64;
    pub fn cardinal_audio_process(frames: i32, input_buf: *const f32, output_buf: *mut f32);

    pub fn cardinal_process(frames: i32);
    pub fn cardinal_get_sample_rate() -> f32;
}

// ── NanoVG internal API ──────────────────────────────────────────────

unsafe extern "C" {
    pub fn nvgCreateInternal(params: *mut NVGparams, other: *mut NVGcontext) -> *mut NVGcontext;
    pub fn nvgDeleteInternal(ctx: *mut NVGcontext);
    pub fn nvgBeginFrame(ctx: *mut NVGcontext, window_width: f32, window_height: f32, device_pixel_ratio: f32);
    pub fn nvgEndFrame(ctx: *mut NVGcontext);
    pub fn nvgInternalParams(ctx: *mut NVGcontext) -> *mut NVGparams;
    pub fn nvgTransformInverse(dst: *mut f32, src: *const f32) -> c_int;
}
