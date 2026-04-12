//! wgpu backend for NanoVG.
//!
//! Provides the 12 `NVGparams` callbacks and
//! `create_context` / `destroy_context` entry points.

use std::collections::HashMap;
use std::ffi::c_int;
use std::ffi::c_void;
use std::sync::Arc;

use wgpu::util::DeviceExt;

use crate::ffi;

// ── Supporting types ─────────────────────────────────────────────────

/// Metadata and GPU resources for a texture stored in the registry.
pub struct TextureEntry {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: i32,
    pub height: i32,
    pub tex_type: i32,
    pub flags: i32,
}

/// The kind of draw call batched for flush.
#[derive(Debug, Clone, Copy)]
pub enum CallType {
    Fill,
    ConvexFill,
    Stroke,
    Triangles,
}

/// Per-draw blend state using wgpu blend factors.
#[derive(Debug, Clone, Copy)]
pub struct BlendState {
    pub src_rgb: wgpu::BlendFactor,
    pub dst_rgb: wgpu::BlendFactor,
    pub src_alpha: wgpu::BlendFactor,
    pub dst_alpha: wgpu::BlendFactor,
}

impl Default for BlendState {
    fn default() -> Self {
        Self {
            src_rgb: wgpu::BlendFactor::One,
            dst_rgb: wgpu::BlendFactor::OneMinusSrcAlpha,
            src_alpha: wgpu::BlendFactor::One,
            dst_alpha: wgpu::BlendFactor::OneMinusSrcAlpha,
        }
    }
}

/// Cached path vertex data for a draw call.
#[derive(Debug, Clone, Default)]
pub struct PathData {
    pub fill_offset: u32,
    pub fill_count: u32,
    pub stroke_offset: u32,
    pub stroke_count: u32,
}

/// Fragment-shader uniform block — 11 vec4s = 176 bytes, matching the WGSL layout.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FragUniforms {
    pub scissor_mat: [f32; 12],       // 3 vec4s (mat3 rows, padded to vec4)
    pub paint_mat: [f32; 12],         // 3 vec4s
    pub inner_col: [f32; 4],          // vec4
    pub outer_col: [f32; 4],          // vec4
    pub scissor_ext_scale: [f32; 4],  // xy=scissorExt, zw=scissorScale
    pub extent_radius_feather: [f32; 4], // xy=extent, z=radius, w=feather
    pub stroke_params: [f32; 4],      // x=strokeMult, y=strokeThr, z=texType, w=type
}

impl Default for FragUniforms {
    fn default() -> Self {
        Self {
            scissor_mat: [0.0; 12],
            paint_mat: [0.0; 12],
            inner_col: [0.0; 4],
            outer_col: [0.0; 4],
            scissor_ext_scale: [0.0; 4],
            extent_radius_feather: [0.0; 4],
            stroke_params: [0.0; 4],
        }
    }
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

// ── Render target format constants ──────────────────────────────────

const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Stencil8;

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

    // GPU resources
    pub shader_module: Option<wgpu::ShaderModule>,
    pub view_bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub texture_bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub pipeline_layout: Option<wgpu::PipelineLayout>,

    // Pipelines for different draw modes
    pub pipeline_convex_fill: Option<wgpu::RenderPipeline>,
    pub pipeline_triangles: Option<wgpu::RenderPipeline>,

    // Stencil fill pipelines
    pub pipeline_stencil_fill_draw_stencil: Option<wgpu::RenderPipeline>,
    pub pipeline_stencil_fill_draw_aa: Option<wgpu::RenderPipeline>,
    pub pipeline_stencil_fill_cover: Option<wgpu::RenderPipeline>,

    // Stroke pipelines
    pub pipeline_stroke: Option<wgpu::RenderPipeline>,
    pub pipeline_stencil_stroke_draw: Option<wgpu::RenderPipeline>,
    pub pipeline_stencil_stroke_aa: Option<wgpu::RenderPipeline>,
    pub pipeline_stencil_stroke_clear: Option<wgpu::RenderPipeline>,

    // Uniform buffers
    pub view_uniform_buffer: Option<wgpu::Buffer>,
    pub view_bind_group: Option<wgpu::BindGroup>,

    // Vertex buffer
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub vertex_buffer_capacity: usize,

    // Dummy texture + samplers
    pub dummy_texture_bind_group: Option<wgpu::BindGroup>,
    pub default_sampler: Option<wgpu::Sampler>,
    pub nearest_sampler: Option<wgpu::Sampler>,

    // Texture bind group cache (keyed by texture id)
    pub texture_bind_groups: HashMap<i32, wgpu::BindGroup>,

    // Render target
    pub render_target_view: Option<wgpu::TextureView>,
    pub render_target_stencil: Option<wgpu::Texture>,
    pub render_target_stencil_view: Option<wgpu::TextureView>,
    pub render_target_width: u32,
    pub render_target_height: u32,

    // Frame state
    pub first_pass_of_frame: bool,

    // Render target stack for FBO bind/unbind
    pub render_target_stack: Vec<RenderTargetState>,
}

/// Saved render target state for FBO push/pop.
pub struct RenderTargetState {
    pub view: Option<wgpu::TextureView>,
    pub stencil: Option<wgpu::Texture>,
    pub stencil_view: Option<wgpu::TextureView>,
    pub width: u32,
    pub height: u32,
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
            shader_module: None,
            view_bind_group_layout: None,
            texture_bind_group_layout: None,
            pipeline_layout: None,
            pipeline_convex_fill: None,
            pipeline_triangles: None,
            pipeline_stencil_fill_draw_stencil: None,
            pipeline_stencil_fill_draw_aa: None,
            pipeline_stencil_fill_cover: None,
            pipeline_stroke: None,
            pipeline_stencil_stroke_draw: None,
            pipeline_stencil_stroke_aa: None,
            pipeline_stencil_stroke_clear: None,
            view_uniform_buffer: None,
            view_bind_group: None,
            vertex_buffer: None,
            vertex_buffer_capacity: 0,
            dummy_texture_bind_group: None,
            default_sampler: None,
            nearest_sampler: None,
            texture_bind_groups: HashMap::new(),
            render_target_view: None,
            render_target_stencil: None,
            render_target_stencil_view: None,
            render_target_width: 0,
            render_target_height: 0,
            first_pass_of_frame: true,
            render_target_stack: Vec::new(),
        }
    }
}

impl WgpuNvgContext {
    /// Get or create a bind group for the given texture id.
    /// Returns the dummy bind group for id 0 or missing textures.
    fn ensure_texture_bind_group(&mut self, image: i32) -> &wgpu::BindGroup {
        if image == 0 {
            return self.dummy_texture_bind_group.as_ref().unwrap();
        }

        if !self.texture_bind_groups.contains_key(&image) {
            let (view, flags) = if let Some(tex) = self.textures.get(&image) {
                (&tex.view as *const wgpu::TextureView, tex.flags)
            } else {
                return self.dummy_texture_bind_group.as_ref().unwrap();
            };

            let sampler = if (flags & ffi::NVG_IMAGE_NEAREST) != 0 {
                self.nearest_sampler.as_ref().unwrap()
            } else {
                self.default_sampler.as_ref().unwrap()
            };

            // SAFETY: view pointer is valid because we checked textures.get above
            // and we don't modify textures between the get and this usage.
            let view_ref = unsafe { &*view };

            let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("nvg_tex_bg_{image}")),
                layout: self.texture_bind_group_layout.as_ref().unwrap(),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view_ref),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            });
            self.texture_bind_groups.insert(image, bg);
        }
        self.texture_bind_groups.get(&image).unwrap()
    }
}

// ── Helper to recover &mut WgpuNvgContext from the void* ─────────────

unsafe fn ctx_from_uptr<'a>(uptr: *mut c_void) -> &'a mut WgpuNvgContext {
    unsafe { &mut *(uptr as *mut WgpuNvgContext) }
}

// ── Pipeline creation helper ────────────────────────────────────────

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    vertex_layout: wgpu::VertexBufferLayout<'_>,
    fs_entry: &str,
    blend: Option<wgpu::BlendState>,
    color_writes: wgpu::ColorWrites,
    depth_stencil: Option<wgpu::DepthStencilState>,
    cull_mode: Option<wgpu::Face>,
    label: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[vertex_layout],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs_entry),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend,
                write_mask: color_writes,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// Standard premultiplied-alpha blend state.
fn premultiplied_blend() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

/// Vertex buffer layout for NVGvertex (x, y, u, v — 4 floats, 16 bytes).
fn vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: 16,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            // location 0: vertex position (x, y)
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            },
            // location 1: texture coords (u, v)
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 8,
                shader_location: 1,
            },
        ],
    }
}

/// Build a `DepthStencilState` for the Stencil8 format with given front/back stencil ops.
fn stencil_state(
    front: wgpu::StencilFaceState,
    back: wgpu::StencilFaceState,
) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: STENCIL_FORMAT,
        depth_write_enabled: Some(false),
        depth_compare: Some(wgpu::CompareFunction::Always),
        stencil: wgpu::StencilState {
            front,
            back,
            read_mask: 0xFF,
            write_mask: 0xFF,
        },
        bias: wgpu::DepthBiasState::default(),
    }
}

fn stencil_face(
    compare: wgpu::CompareFunction,
    pass_op: wgpu::StencilOperation,
) -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op,
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn convert_blend_factor(f: i32) -> wgpu::BlendFactor {
    match f {
        x if x == ffi::NVG_ZERO => wgpu::BlendFactor::Zero,
        x if x == ffi::NVG_ONE => wgpu::BlendFactor::One,
        x if x == ffi::NVG_SRC_COLOR => wgpu::BlendFactor::Src,
        x if x == ffi::NVG_ONE_MINUS_SRC_COLOR => wgpu::BlendFactor::OneMinusSrc,
        x if x == ffi::NVG_DST_COLOR => wgpu::BlendFactor::Dst,
        x if x == ffi::NVG_ONE_MINUS_DST_COLOR => wgpu::BlendFactor::OneMinusDst,
        x if x == ffi::NVG_SRC_ALPHA => wgpu::BlendFactor::SrcAlpha,
        x if x == ffi::NVG_ONE_MINUS_SRC_ALPHA => wgpu::BlendFactor::OneMinusSrcAlpha,
        x if x == ffi::NVG_DST_ALPHA => wgpu::BlendFactor::DstAlpha,
        x if x == ffi::NVG_ONE_MINUS_DST_ALPHA => wgpu::BlendFactor::OneMinusDstAlpha,
        x if x == ffi::NVG_SRC_ALPHA_SATURATE => wgpu::BlendFactor::SrcAlphaSaturated,
        _ => wgpu::BlendFactor::One,
    }
}

fn blend_composite_operation(op: &ffi::NVGcompositeOperationState) -> BlendState {
    let b = BlendState {
        src_rgb: convert_blend_factor(op.src_rgb),
        dst_rgb: convert_blend_factor(op.dst_rgb),
        src_alpha: convert_blend_factor(op.src_alpha),
        dst_alpha: convert_blend_factor(op.dst_alpha),
    };
    // Fallback to premultiplied alpha if any factor is invalid (shouldn't happen with our mapping)
    b
}

fn premul_color(c: &ffi::NVGcolor) -> [f32; 4] {
    let r = c.rgba[0];
    let g = c.rgba[1];
    let b = c.rgba[2];
    let a = c.rgba[3];
    [r * a, g * a, b * a, a]
}

fn xform_to_mat3x4(dst: &mut [f32; 12], src: &[f32; 6]) {
    dst[0] = src[0];
    dst[1] = src[1];
    dst[2] = 0.0;
    dst[3] = 0.0;
    dst[4] = src[2];
    dst[5] = src[3];
    dst[6] = 0.0;
    dst[7] = 0.0;
    dst[8] = src[4];
    dst[9] = src[5];
    dst[10] = 1.0;
    dst[11] = 0.0;
}

fn convert_paint(
    ctx: &WgpuNvgContext,
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
        // Disable scissor
        frag.scissor_ext_scale[0] = 1.0;
        frag.scissor_ext_scale[1] = 1.0;
        frag.scissor_ext_scale[2] = 1.0;
        frag.scissor_ext_scale[3] = 1.0;
    } else {
        let mut inv_xform = [0.0f32; 6];
        unsafe { ffi::nvgTransformInverse(inv_xform.as_mut_ptr(), scissor.xform.as_ptr()) };
        xform_to_mat3x4(&mut frag.scissor_mat, &inv_xform);
        frag.scissor_ext_scale[0] = scissor.extent[0];
        frag.scissor_ext_scale[1] = scissor.extent[1];
        frag.scissor_ext_scale[2] = (scissor.xform[0] * scissor.xform[0]
            + scissor.xform[2] * scissor.xform[2])
            .sqrt()
            / fringe;
        frag.scissor_ext_scale[3] = (scissor.xform[1] * scissor.xform[1]
            + scissor.xform[3] * scissor.xform[3])
            .sqrt()
            / fringe;
    }

    frag.extent_radius_feather[0] = paint.extent[0];
    frag.extent_radius_feather[1] = paint.extent[1];
    frag.stroke_params[0] = (width * 0.5 + fringe * 0.5) / fringe;
    frag.stroke_params[1] = stroke_thr;

    let mut inv_xform = [0.0f32; 6];

    if paint.image != 0 {
        let tex = ctx.textures.get(&paint.image);
        unsafe { ffi::nvgTransformInverse(inv_xform.as_mut_ptr(), paint.xform.as_ptr()) };
        // type = FILLIMG = 1
        frag.stroke_params[3] = 1.0;

        if let Some(tex) = tex {
            if tex.tex_type == ffi::NVG_TEXTURE_RGBA {
                frag.stroke_params[2] =
                    if (tex.flags & ffi::NVG_IMAGE_PREMULTIPLIED) != 0 { 0.0 } else { 1.0 };
            } else {
                frag.stroke_params[2] = 2.0;
            }
        }
    } else {
        // type = FILLGRAD = 0
        frag.stroke_params[3] = 0.0;
        frag.extent_radius_feather[2] = paint.radius;
        frag.extent_radius_feather[3] = paint.feather;
        unsafe { ffi::nvgTransformInverse(inv_xform.as_mut_ptr(), paint.xform.as_ptr()) };
    }

    xform_to_mat3x4(&mut frag.paint_mat, &inv_xform);

    frag
}

/// Convert triangle fan vertices to triangle list.
/// Fan [0,1,2,3,...,n] -> list [0,1,2, 0,2,3, 0,3,4, ...]
fn fan_to_triangles(fan: &[ffi::NVGvertex]) -> Vec<ffi::NVGvertex> {
    if fan.len() < 3 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity((fan.len() - 2) * 3);
    for i in 2..fan.len() {
        out.push(fan[0]);
        out.push(fan[i - 1]);
        out.push(fan[i]);
    }
    out
}

/// Convert triangle strip vertices to triangle list.
/// Strip [0,1,2,3,...] -> list [0,1,2, 2,1,3, 2,3,4, 4,3,5, ...]
fn strip_to_triangles(strip: &[ffi::NVGvertex]) -> Vec<ffi::NVGvertex> {
    if strip.len() < 3 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity((strip.len() - 2) * 3);
    for i in 2..strip.len() {
        if i % 2 == 0 {
            out.push(strip[i - 2]);
            out.push(strip[i - 1]);
            out.push(strip[i]);
        } else {
            out.push(strip[i - 1]);
            out.push(strip[i - 2]);
            out.push(strip[i]);
        }
    }
    out
}

/// Set the render target for NanoVG rendering.
/// Must be called before nvgBeginFrame.
pub fn set_render_target(
    ctx_ptr: *mut ffi::NVGcontext,
    target_view: wgpu::TextureView,
    width: u32,
    height: u32,
) {
    if ctx_ptr.is_null() {
        return;
    }
    let params = unsafe { ffi::nvgInternalParams(ctx_ptr) };
    if params.is_null() {
        return;
    }
    let uptr = unsafe { (*params).user_ptr };
    let ctx = unsafe { ctx_from_uptr(uptr) };

    // Recreate stencil texture if size changed
    if ctx.render_target_width != width || ctx.render_target_height != height {
        let stencil_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("nvg_stencil_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: STENCIL_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let stencil_view = stencil_texture.create_view(&wgpu::TextureViewDescriptor::default());
        ctx.render_target_stencil = Some(stencil_texture);
        ctx.render_target_stencil_view = Some(stencil_view);
        ctx.render_target_width = width;
        ctx.render_target_height = height;
    }

    ctx.render_target_view = Some(target_view);
    ctx.first_pass_of_frame = true;
}

/// Called from C++ (nvgluBindFramebuffer) to switch the wgpu render target
/// to the texture backing a NanoVG image (for FBO-based caching).
///
/// - `image >= 0`: push current target, switch to this image's texture
/// - `image < 0`: pop and restore the previous target
#[unsafe(no_mangle)]
pub extern "C" fn nvg_wgpu_bind_framebuffer(
    ctx_ptr: *mut ffi::NVGcontext,
    image: i32,
    width: i32,
    height: i32,
) {
    if ctx_ptr.is_null() {
        return;
    }
    let params = unsafe { ffi::nvgInternalParams(ctx_ptr) };
    if params.is_null() {
        return;
    }
    let uptr = unsafe { (*params).user_ptr };
    let ctx = unsafe { ctx_from_uptr(uptr) };

    if image >= 0 {
        // Push current render target state
        ctx.render_target_stack.push(RenderTargetState {
            view: ctx.render_target_view.take(),
            stencil: ctx.render_target_stencil.take(),
            stencil_view: ctx.render_target_stencil_view.take(),
            width: ctx.render_target_width,
            height: ctx.render_target_height,
        });

        // Look up the texture for this NanoVG image
        if let Some(tex_entry) = ctx.textures.get(&image) {
            let view = tex_entry.texture.create_view(&wgpu::TextureViewDescriptor::default());
            let w = width as u32;
            let h = height as u32;

            // Create stencil for this FBO size
            let stencil_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("nvg_fbo_stencil"),
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: STENCIL_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let stencil_view = stencil_texture.create_view(&wgpu::TextureViewDescriptor::default());

            ctx.render_target_view = Some(view);
            ctx.render_target_stencil = Some(stencil_texture);
            ctx.render_target_stencil_view = Some(stencil_view);
            ctx.render_target_width = w;
            ctx.render_target_height = h;
            ctx.first_pass_of_frame = true;
        } else {
            eprintln!("nvg_wgpu_bind_framebuffer: image {image} not found in texture registry");
        }
    } else {
        // Pop: restore previous render target
        if let Some(state) = ctx.render_target_stack.pop() {
            ctx.render_target_view = state.view;
            ctx.render_target_stencil = state.stencil;
            ctx.render_target_stencil_view = state.stencil_view;
            ctx.render_target_width = state.width;
            ctx.render_target_height = state.height;
        }
    }
}

// ── NVGparams callbacks ─────────────────────────────────────────────

unsafe extern "C" fn render_create(uptr: *mut c_void, _other_uptr: *mut c_void) -> c_int {
    let ctx = unsafe { ctx_from_uptr(uptr) };
    let device = &ctx.device;
    let edge_anti_alias = ctx.flags & ffi::NVG_ANTIALIAS != 0;

    // 1. Load shader
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("nanovg_wgpu_shader"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("nanovg_wgpu_shaders.wgsl").into(),
        ),
    });

    // 2. Bind group layouts
    let view_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nvg_view_bind_group_layout"),
            entries: &[
                // binding 0: ViewUniforms (vertex + fragment)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 1: FragUniforms (fragment only)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

    let texture_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nvg_texture_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

    // 3. Pipeline layout
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("nvg_pipeline_layout"),
        bind_group_layouts: &[Some(&view_bind_group_layout), Some(&texture_bind_group_layout)],
        immediate_size: 0,
    });

    // 4. Choose fragment entry point based on AA flag
    let fs_aa = if edge_anti_alias {
        "fs_main_edge_aa"
    } else {
        "fs_main_no_aa"
    };

    let blend = Some(premultiplied_blend());
    let vbl = vertex_buffer_layout;

    // 5. Create all render pipelines

    // a) convex fill
    let pipeline_convex_fill = create_pipeline(
        device,
        &pipeline_layout,
        &shader,
        vbl(),
        fs_aa,
        blend,
        wgpu::ColorWrites::ALL,
        None,
        Some(wgpu::Face::Back),
        "nvg_pipeline_convex_fill",
    );

    // b) triangles
    let pipeline_triangles = create_pipeline(
        device,
        &pipeline_layout,
        &shader,
        vbl(),
        fs_aa,
        blend,
        wgpu::ColorWrites::ALL,
        None,
        None,
        "nvg_pipeline_triangles",
    );

    // c) stencil fill — draw stencil (no color, both faces for winding)
    let pipeline_stencil_fill_draw_stencil = create_pipeline(
        device,
        &pipeline_layout,
        &shader,
        vbl(),
        "fs_main_no_aa",
        None,
        wgpu::ColorWrites::empty(),
        Some(stencil_state(
            stencil_face(wgpu::CompareFunction::Always, wgpu::StencilOperation::IncrementWrap),
            stencil_face(wgpu::CompareFunction::Always, wgpu::StencilOperation::DecrementWrap),
        )),
        None,
        "nvg_pipeline_stencil_fill_draw_stencil",
    );

    // d) stencil fill — draw AA fringes
    let pipeline_stencil_fill_draw_aa = create_pipeline(
        device,
        &pipeline_layout,
        &shader,
        vbl(),
        "fs_main_edge_aa",
        blend,
        wgpu::ColorWrites::ALL,
        Some(stencil_state(
            stencil_face(wgpu::CompareFunction::Equal, wgpu::StencilOperation::Keep),
            stencil_face(wgpu::CompareFunction::Equal, wgpu::StencilOperation::Keep),
        )),
        Some(wgpu::Face::Back),
        "nvg_pipeline_stencil_fill_draw_aa",
    );

    // e) stencil fill — cover (reset stencil)
    let pipeline_stencil_fill_cover = create_pipeline(
        device,
        &pipeline_layout,
        &shader,
        vbl(),
        fs_aa,
        blend,
        wgpu::ColorWrites::ALL,
        Some(stencil_state(
            stencil_face(wgpu::CompareFunction::NotEqual, wgpu::StencilOperation::Zero),
            stencil_face(wgpu::CompareFunction::NotEqual, wgpu::StencilOperation::Zero),
        )),
        Some(wgpu::Face::Back),
        "nvg_pipeline_stencil_fill_cover",
    );

    // f) stroke (no stencil)
    let pipeline_stroke = create_pipeline(
        device,
        &pipeline_layout,
        &shader,
        vbl(),
        fs_aa,
        blend,
        wgpu::ColorWrites::ALL,
        None,
        Some(wgpu::Face::Back),
        "nvg_pipeline_stroke",
    );

    // g) stencil stroke — draw
    let pipeline_stencil_stroke_draw = create_pipeline(
        device,
        &pipeline_layout,
        &shader,
        vbl(),
        "fs_main_edge_aa",
        blend,
        wgpu::ColorWrites::ALL,
        Some(stencil_state(
            stencil_face(wgpu::CompareFunction::Equal, wgpu::StencilOperation::IncrementClamp),
            stencil_face(wgpu::CompareFunction::Equal, wgpu::StencilOperation::IncrementClamp),
        )),
        Some(wgpu::Face::Back),
        "nvg_pipeline_stencil_stroke_draw",
    );

    // h) stencil stroke — AA
    let pipeline_stencil_stroke_aa = create_pipeline(
        device,
        &pipeline_layout,
        &shader,
        vbl(),
        "fs_main_edge_aa",
        blend,
        wgpu::ColorWrites::ALL,
        Some(stencil_state(
            stencil_face(wgpu::CompareFunction::Equal, wgpu::StencilOperation::Keep),
            stencil_face(wgpu::CompareFunction::Equal, wgpu::StencilOperation::Keep),
        )),
        Some(wgpu::Face::Back),
        "nvg_pipeline_stencil_stroke_aa",
    );

    // i) stencil stroke — clear
    let pipeline_stencil_stroke_clear = create_pipeline(
        device,
        &pipeline_layout,
        &shader,
        vbl(),
        "fs_main_no_aa",
        None,
        wgpu::ColorWrites::empty(),
        Some(stencil_state(
            stencil_face(wgpu::CompareFunction::Always, wgpu::StencilOperation::Zero),
            stencil_face(wgpu::CompareFunction::Always, wgpu::StencilOperation::Zero),
        )),
        None,
        "nvg_pipeline_stencil_stroke_clear",
    );

    // 6. View uniform buffer (2 floats = 8 bytes, but uniform buffers
    //    must be at least 16 bytes on some backends, so use 16)
    let view_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("nvg_view_uniform_buffer"),
        size: 16,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // 7. View bind group — need a frag uniform buffer placeholder too.
    //    We'll create a small buffer for the frag uniforms (will be replaced at flush time).
    let frag_uniform_placeholder = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("nvg_frag_uniform_placeholder"),
        size: 11 * 16, // 11 vec4s = 176 bytes
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("nvg_view_bind_group"),
        layout: &view_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: view_uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &frag_uniform_placeholder,
                    offset: 0,
                    size: std::num::NonZeroU64::new(11 * 16),
                }),
            },
        ],
    });

    // 8. Samplers
    let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("nvg_default_sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    });

    let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("nvg_nearest_sampler"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    });

    // 9. Dummy 1x1 white texture + bind group
    let dummy_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("nvg_dummy_texture"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    ctx.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &dummy_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[255u8, 255, 255, 255],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );

    let dummy_view = dummy_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let dummy_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("nvg_dummy_texture_bind_group"),
        layout: &texture_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&dummy_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&default_sampler),
            },
        ],
    });

    // Store everything
    ctx.shader_module = Some(shader);
    ctx.view_bind_group_layout = Some(view_bind_group_layout);
    ctx.texture_bind_group_layout = Some(texture_bind_group_layout);
    ctx.pipeline_layout = Some(pipeline_layout);
    ctx.pipeline_convex_fill = Some(pipeline_convex_fill);
    ctx.pipeline_triangles = Some(pipeline_triangles);
    ctx.pipeline_stencil_fill_draw_stencil = Some(pipeline_stencil_fill_draw_stencil);
    ctx.pipeline_stencil_fill_draw_aa = Some(pipeline_stencil_fill_draw_aa);
    ctx.pipeline_stencil_fill_cover = Some(pipeline_stencil_fill_cover);
    ctx.pipeline_stroke = Some(pipeline_stroke);
    ctx.pipeline_stencil_stroke_draw = Some(pipeline_stencil_stroke_draw);
    ctx.pipeline_stencil_stroke_aa = Some(pipeline_stencil_stroke_aa);
    ctx.pipeline_stencil_stroke_clear = Some(pipeline_stencil_stroke_clear);
    ctx.view_uniform_buffer = Some(view_uniform_buffer);
    ctx.view_bind_group = Some(view_bind_group);
    ctx.dummy_texture_bind_group = Some(dummy_texture_bind_group);
    ctx.default_sampler = Some(default_sampler);
    ctx.nearest_sampler = Some(nearest_sampler);

    eprintln!("nanovg-wgpu: renderCreate — pipelines and resources initialized");
    1 // success
}

unsafe extern "C" fn render_create_texture(
    uptr: *mut c_void,
    tex_type: c_int,
    w: c_int,
    h: c_int,
    image_flags: c_int,
    data: *const u8,
) -> c_int {
    let ctx = unsafe { ctx_from_uptr(uptr) };
    let id = ctx.next_texture_id;
    ctx.next_texture_id += 1;

    let (format, bpp): (wgpu::TextureFormat, u32) = if tex_type == ffi::NVG_TEXTURE_RGBA {
        (wgpu::TextureFormat::Rgba8Unorm, 4)
    } else {
        (wgpu::TextureFormat::R8Unorm, 1)
    };

    let generate_mipmaps = (image_flags & ffi::NVG_IMAGE_GENERATE_MIPMAPS) != 0;
    let mip_level_count = if generate_mipmaps {
        ((w.max(h) as f32).log2().floor() as u32) + 1
    } else {
        1
    };

    // RENDER_ATTACHMENT is always needed: textures may be used as FBO targets
    // by FramebufferWidget for offscreen caching of SVG panels, knobs, etc.
    let usage = wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::COPY_DST
        | wgpu::TextureUsages::RENDER_ATTACHMENT;
    let _ = generate_mipmaps; // kept for future mipmap generation

    let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&format!("nvg_texture_{id}")),
        size: wgpu::Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        mip_level_count,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    });

    if !data.is_null() {
        let data_size = (w as usize) * (h as usize) * (bpp as usize);
        let data_slice = unsafe { std::slice::from_raw_parts(data, data_size) };
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
                bytes_per_row: Some(w as u32 * bpp),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: w as u32,
                height: h as u32,
                depth_or_array_layers: 1,
            },
        );
    }

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Invalidate any stale bind group cache entry
    ctx.texture_bind_groups.remove(&id);

    ctx.textures.insert(
        id,
        TextureEntry {
            texture,
            view,
            width: w,
            height: h,
            tex_type,
            flags: image_flags,
        },
    );
    eprintln!("nanovg-wgpu: renderCreateTexture id={id} {w}x{h} type={tex_type} mips={mip_level_count}");
    id
}

unsafe extern "C" fn render_delete_texture(uptr: *mut c_void, image: c_int) -> c_int {
    let ctx = unsafe { ctx_from_uptr(uptr) };
    ctx.texture_bind_groups.remove(&image);
    if ctx.textures.remove(&image).is_some() {
        eprintln!("nanovg-wgpu: renderDeleteTexture id={image}");
        1
    } else {
        0
    }
}

unsafe extern "C" fn render_update_texture(
    uptr: *mut c_void,
    image: c_int,
    x: c_int,
    y: c_int,
    w: c_int,
    h: c_int,
    data: *const u8,
) -> c_int {
    let ctx = unsafe { ctx_from_uptr(uptr) };

    let (tex_width, tex_type) = if let Some(tex) = ctx.textures.get(&image) {
        (tex.width, tex.tex_type)
    } else {
        return 0;
    };

    let bpp: usize = if tex_type == ffi::NVG_TEXTURE_RGBA { 4 } else { 1 };

    // Extract the sub-region into a tightly packed buffer
    let row_bytes = w as usize * bpp;
    let mut packed = Vec::with_capacity(h as usize * row_bytes);
    for r in 0..h as usize {
        let src_offset = (y as usize + r) * tex_width as usize * bpp + x as usize * bpp;
        let src = unsafe { std::slice::from_raw_parts(data.add(src_offset), row_bytes) };
        packed.extend_from_slice(src);
    }

    let texture = &ctx.textures.get(&image).unwrap().texture;
    ctx.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: x as u32,
                y: y as u32,
                z: 0,
            },
            aspect: wgpu::TextureAspect::All,
        },
        &packed,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(row_bytes as u32),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
    );

    // Invalidate cached bind group since texture content changed
    ctx.texture_bind_groups.remove(&image);

    eprintln!("nanovg-wgpu: renderUpdateTexture id={image} region=({x},{y},{w},{h})");
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

unsafe extern "C" fn render_flush(uptr: *mut c_void) {
    let ctx = unsafe { ctx_from_uptr(uptr) };

    if ctx.draw_calls.is_empty() {
        ctx.vertices.clear();
        ctx.uniforms.clear();
        return;
    }

    // Get raw pointers to avoid borrow issues with ensure_texture_bind_group
    let render_target_view = match ctx.render_target_view.as_ref() {
        Some(v) => v as *const wgpu::TextureView,
        None => {
            eprintln!("nanovg-wgpu: renderFlush — no render target set, skipping");
            ctx.draw_calls.clear();
            ctx.vertices.clear();
            ctx.uniforms.clear();
            return;
        }
    };
    let render_target_view = unsafe { &*render_target_view };

    // 1. Upload view uniforms
    let view_data: [f32; 4] = [ctx.view_width, ctx.view_height, 0.0, 0.0];
    ctx.queue.write_buffer(
        ctx.view_uniform_buffer.as_ref().unwrap(),
        0,
        unsafe { std::slice::from_raw_parts(view_data.as_ptr() as *const u8, std::mem::size_of_val(&view_data)) },
    );

    // 2. Compute aligned frag uniform size
    let align = ctx
        .device
        .limits()
        .min_uniform_buffer_offset_alignment as usize;
    let frag_size = std::mem::size_of::<FragUniforms>();
    let aligned_frag_size = (frag_size + align - 1) & !(align - 1);

    // 3. Build padded uniform buffer
    let num_uniforms = ctx.uniforms.len();
    let total_uniform_bytes = num_uniforms * aligned_frag_size;
    let mut uniform_data = vec![0u8; total_uniform_bytes.max(aligned_frag_size)];
    for (i, u) in ctx.uniforms.iter().enumerate() {
        let offset = i * aligned_frag_size;
        let src = unsafe {
            std::slice::from_raw_parts(u as *const FragUniforms as *const u8, frag_size)
        };
        uniform_data[offset..offset + frag_size].copy_from_slice(src);
    }

    let frag_uniform_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("nvg_frag_uniform_buffer"),
        contents: &uniform_data,
        usage: wgpu::BufferUsages::UNIFORM,
    });

    // 4. Upload vertices
    if ctx.vertices.is_empty() {
        // Push a dummy vertex so we have a valid buffer
        ctx.vertices.push(ffi::NVGvertex::default());
    }
    let vert_data = unsafe {
        std::slice::from_raw_parts(
            ctx.vertices.as_ptr() as *const u8,
            ctx.vertices.len() * std::mem::size_of::<ffi::NVGvertex>(),
        )
    };
    let vert_buf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("nvg_vertex_buffer"),
        contents: vert_data,
        usage: wgpu::BufferUsages::VERTEX,
    });

    // 5. Create bind group with the real frag uniform buffer
    let view_bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("nvg_view_bind_group_flush"),
        layout: ctx.view_bind_group_layout.as_ref().unwrap(),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: ctx.view_uniform_buffer.as_ref().unwrap().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &frag_uniform_buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(frag_size as u64),
                }),
            },
        ],
    });

    // 6. Encode draw calls
    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("nvg_flush_encoder"),
        });

    // Take draw calls out so we can iterate while borrowing ctx for texture bind groups
    let draw_calls = std::mem::take(&mut ctx.draw_calls);

    let edge_anti_alias = ctx.flags & ffi::NVG_ANTIALIAS != 0;
    let stencil_strokes = ctx.flags & ffi::NVG_STENCIL_STROKES != 0;

    for call in &draw_calls {
        let uniform_offset = (call.uniform_offset as usize * aligned_frag_size) as u32;

        // Get texture bind group
        let tex_bg = ctx.ensure_texture_bind_group(call.image);
        // We need to work around the borrow checker: store a raw pointer
        let tex_bg_ptr = tex_bg as *const wgpu::BindGroup;

        match call.call_type {
            CallType::Triangles => {
                let color_load = if ctx.first_pass_of_frame {
                    ctx.first_pass_of_frame = false;
                    wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                } else {
                    wgpu::LoadOp::Load
                };
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("nvg_triangles_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: render_target_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: color_load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                rpass.set_pipeline(ctx.pipeline_triangles.as_ref().unwrap());
                rpass.set_vertex_buffer(0, vert_buf.slice(..));
                rpass.set_bind_group(0, Some(&view_bind_group), &[uniform_offset]);
                rpass.set_bind_group(1, Some(unsafe { &*tex_bg_ptr }), &[]);
                rpass.draw(
                    call.triangle_offset..call.triangle_offset + call.triangle_count,
                    0..1,
                );
            }

            CallType::ConvexFill => {
                let color_load = if ctx.first_pass_of_frame {
                    ctx.first_pass_of_frame = false;
                    wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                } else {
                    wgpu::LoadOp::Load
                };
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("nvg_convex_fill_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: render_target_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: color_load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                rpass.set_pipeline(ctx.pipeline_convex_fill.as_ref().unwrap());
                rpass.set_vertex_buffer(0, vert_buf.slice(..));
                rpass.set_bind_group(0, Some(&view_bind_group), &[uniform_offset]);
                rpass.set_bind_group(1, Some(unsafe { &*tex_bg_ptr }), &[]);

                for path in &call.paths {
                    if path.fill_count > 0 {
                        rpass.draw(
                            path.fill_offset..path.fill_offset + path.fill_count,
                            0..1,
                        );
                    }
                    if path.stroke_count > 0 {
                        rpass.draw(
                            path.stroke_offset..path.stroke_offset + path.stroke_count,
                            0..1,
                        );
                    }
                }
            }

            CallType::Fill => {
                let stencil_view = match ctx.render_target_stencil_view.as_ref() {
                    Some(v) => v,
                    None => continue,
                };

                let color_load = if ctx.first_pass_of_frame {
                    ctx.first_pass_of_frame = false;
                    wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                } else {
                    wgpu::LoadOp::Load
                };

                let second_uniform_offset =
                    ((call.uniform_offset as usize + 1) * aligned_frag_size) as u32;

                // Pass 1: Draw to stencil (no color write)
                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("nvg_stencil_fill_stencil_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: render_target_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: color_load,
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: stencil_view,
                            depth_ops: None,
                            stencil_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(0),
                                store: wgpu::StoreOp::Store,
                            }),
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    multiview_mask: None,
                    });
                    rpass.set_pipeline(ctx.pipeline_stencil_fill_draw_stencil.as_ref().unwrap());
                    rpass.set_vertex_buffer(0, vert_buf.slice(..));
                    rpass.set_bind_group(0, Some(&view_bind_group), &[uniform_offset]);
                    rpass.set_bind_group(1, Some(ctx.dummy_texture_bind_group.as_ref().unwrap()), &[]);

                    for path in &call.paths {
                        if path.fill_count > 0 {
                            rpass.draw(
                                path.fill_offset..path.fill_offset + path.fill_count,
                                0..1,
                            );
                        }
                    }
                }

                // Pass 2: AA fringe
                if edge_anti_alias {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("nvg_stencil_fill_aa_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: render_target_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: stencil_view,
                            depth_ops: None,
                            stencil_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            }),
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    multiview_mask: None,
                    });
                    rpass.set_pipeline(ctx.pipeline_stencil_fill_draw_aa.as_ref().unwrap());
                    rpass.set_vertex_buffer(0, vert_buf.slice(..));
                    rpass.set_stencil_reference(0);
                    rpass.set_bind_group(0, Some(&view_bind_group), &[second_uniform_offset]);
                    rpass.set_bind_group(1, Some(unsafe { &*tex_bg_ptr }), &[]);

                    for path in &call.paths {
                        if path.stroke_count > 0 {
                            rpass.draw(
                                path.stroke_offset..path.stroke_offset + path.stroke_count,
                                0..1,
                            );
                        }
                    }
                }

                // Pass 3: Cover (fill where stencil != 0, reset stencil)
                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("nvg_stencil_fill_cover_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: render_target_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: stencil_view,
                            depth_ops: None,
                            stencil_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            }),
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    multiview_mask: None,
                    });
                    rpass.set_pipeline(ctx.pipeline_stencil_fill_cover.as_ref().unwrap());
                    rpass.set_vertex_buffer(0, vert_buf.slice(..));
                    rpass.set_stencil_reference(0);
                    rpass.set_bind_group(0, Some(&view_bind_group), &[second_uniform_offset]);
                    rpass.set_bind_group(1, Some(unsafe { &*tex_bg_ptr }), &[]);
                    rpass.draw(
                        call.triangle_offset..call.triangle_offset + call.triangle_count,
                        0..1,
                    );
                }
            }

            CallType::Stroke => {
                if stencil_strokes {
                    let stencil_view = match ctx.render_target_stencil_view.as_ref() {
                        Some(v) => v,
                        None => continue,
                    };

                    let color_load = if ctx.first_pass_of_frame {
                        ctx.first_pass_of_frame = false;
                        wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                    } else {
                        wgpu::LoadOp::Load
                    };

                    let second_uniform_offset =
                        ((call.uniform_offset as usize + 1) * aligned_frag_size) as u32;

                    // Pass 1: Fill stroke base (no overlap)
                    {
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("nvg_stencil_stroke_draw_pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: render_target_view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: color_load,
                                    store: wgpu::StoreOp::Store,
                                },
                                depth_slice: None,
                            })],
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: stencil_view,
                                    depth_ops: None,
                                    stencil_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(0),
                                        store: wgpu::StoreOp::Store,
                                    }),
                                },
                            ),
                            timestamp_writes: None,
                            occlusion_query_set: None,
                    multiview_mask: None,
                        });
                        rpass.set_pipeline(ctx.pipeline_stencil_stroke_draw.as_ref().unwrap());
                        rpass.set_vertex_buffer(0, vert_buf.slice(..));
                        rpass.set_stencil_reference(0);
                        rpass.set_bind_group(
                            0,
                            Some(&view_bind_group),
                            &[second_uniform_offset],
                        );
                        rpass.set_bind_group(1, Some(unsafe { &*tex_bg_ptr }), &[]);

                        for path in &call.paths {
                            if path.stroke_count > 0 {
                                rpass.draw(
                                    path.stroke_offset
                                        ..path.stroke_offset + path.stroke_count,
                                    0..1,
                                );
                            }
                        }
                    }

                    // Pass 2: AA
                    {
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("nvg_stencil_stroke_aa_pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: render_target_view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                                depth_slice: None,
                            })],
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: stencil_view,
                                    depth_ops: None,
                                    stencil_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: wgpu::StoreOp::Store,
                                    }),
                                },
                            ),
                            timestamp_writes: None,
                            occlusion_query_set: None,
                    multiview_mask: None,
                        });
                        rpass.set_pipeline(ctx.pipeline_stencil_stroke_aa.as_ref().unwrap());
                        rpass.set_vertex_buffer(0, vert_buf.slice(..));
                        rpass.set_bind_group(0, Some(&view_bind_group), &[uniform_offset]);
                        rpass.set_bind_group(1, Some(unsafe { &*tex_bg_ptr }), &[]);

                        for path in &call.paths {
                            if path.stroke_count > 0 {
                                rpass.draw(
                                    path.stroke_offset
                                        ..path.stroke_offset + path.stroke_count,
                                    0..1,
                                );
                            }
                        }
                    }

                    // Pass 3: Clear stencil
                    {
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("nvg_stencil_stroke_clear_pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: render_target_view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                                depth_slice: None,
                            })],
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: stencil_view,
                                    depth_ops: None,
                                    stencil_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: wgpu::StoreOp::Store,
                                    }),
                                },
                            ),
                            timestamp_writes: None,
                            occlusion_query_set: None,
                    multiview_mask: None,
                        });
                        rpass.set_pipeline(ctx.pipeline_stencil_stroke_clear.as_ref().unwrap());
                        rpass.set_vertex_buffer(0, vert_buf.slice(..));
                        rpass.set_bind_group(0, Some(&view_bind_group), &[uniform_offset]);
                        rpass.set_bind_group(1, Some(ctx.dummy_texture_bind_group.as_ref().unwrap()), &[]);

                        for path in &call.paths {
                            if path.stroke_count > 0 {
                                rpass.draw(
                                    path.stroke_offset
                                        ..path.stroke_offset + path.stroke_count,
                                    0..1,
                                );
                            }
                        }
                    }
                } else {
                    // Simple stroke (no stencil)
                    let color_load = if ctx.first_pass_of_frame {
                        ctx.first_pass_of_frame = false;
                        wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                    } else {
                        wgpu::LoadOp::Load
                    };
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("nvg_stroke_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: render_target_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: color_load,
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    multiview_mask: None,
                    });
                    rpass.set_pipeline(ctx.pipeline_stroke.as_ref().unwrap());
                    rpass.set_vertex_buffer(0, vert_buf.slice(..));
                    rpass.set_bind_group(0, Some(&view_bind_group), &[uniform_offset]);
                    rpass.set_bind_group(1, Some(unsafe { &*tex_bg_ptr }), &[]);

                    for path in &call.paths {
                        if path.stroke_count > 0 {
                            rpass.draw(
                                path.stroke_offset..path.stroke_offset + path.stroke_count,
                                0..1,
                            );
                        }
                    }
                }
            }
        }
    }

    // 7. Submit
    ctx.queue.submit(std::iter::once(encoder.finish()));

    // 8. Clear batched state
    ctx.draw_calls.clear();
    ctx.vertices.clear();
    ctx.uniforms.clear();
}

unsafe extern "C" fn render_fill(
    uptr: *mut c_void,
    paint: *mut ffi::NVGpaint,
    composite_operation: ffi::NVGcompositeOperationState,
    scissor: *mut ffi::NVGscissor,
    fringe: f32,
    bounds: *const f32,
    paths: *const ffi::NVGpath,
    npaths: c_int,
) {
    let ctx = unsafe { ctx_from_uptr(uptr) };
    let paint = unsafe { &*paint };
    let scissor = unsafe { &*scissor };
    let paths_slice = unsafe { std::slice::from_raw_parts(paths, npaths as usize) };

    let is_convex = npaths == 1 && paths_slice[0].convex != 0;
    let call_type = if is_convex {
        CallType::ConvexFill
    } else {
        CallType::Fill
    };

    let blend = blend_composite_operation(&composite_operation);

    let mut path_data = Vec::with_capacity(npaths as usize);

    // Copy vertices, converting fan/strip to triangle list
    for path in paths_slice {
        let mut pd = PathData::default();

        if path.nfill > 0 {
            let fill_verts =
                unsafe { std::slice::from_raw_parts(path.fill, path.nfill as usize) };
            let triangles = fan_to_triangles(fill_verts);
            if !triangles.is_empty() {
                pd.fill_offset = ctx.vertices.len() as u32;
                pd.fill_count = triangles.len() as u32;
                ctx.vertices.extend_from_slice(&triangles);
            }
        }

        if path.nstroke > 0 {
            let stroke_verts =
                unsafe { std::slice::from_raw_parts(path.stroke, path.nstroke as usize) };
            let triangles = strip_to_triangles(stroke_verts);
            if !triangles.is_empty() {
                pd.stroke_offset = ctx.vertices.len() as u32;
                pd.stroke_count = triangles.len() as u32;
                ctx.vertices.extend_from_slice(&triangles);
            }
        }

        path_data.push(pd);
    }

    let mut triangle_offset = 0u32;
    let mut triangle_count = 0u32;

    if !is_convex {
        // Add bounding quad as 2 triangles for the cover pass
        let b = unsafe { std::slice::from_raw_parts(bounds, 4) };
        triangle_offset = ctx.vertices.len() as u32;
        // bounds = [minx, miny, maxx, maxy]
        // GL code: quad[0]=br, quad[1]=tr, quad[2]=bl, quad[3]=tl
        // GL draws as TRIANGLE_STRIP: br,tr,bl,tl -> triangles: br,tr,bl + bl,tr,tl
        let br = ffi::NVGvertex { x: b[2], y: b[3], u: 0.5, v: 1.0 };
        let tr = ffi::NVGvertex { x: b[2], y: b[1], u: 0.5, v: 1.0 };
        let bl = ffi::NVGvertex { x: b[0], y: b[3], u: 0.5, v: 1.0 };
        let tl = ffi::NVGvertex { x: b[0], y: b[1], u: 0.5, v: 1.0 };
        ctx.vertices.extend_from_slice(&[br, tr, bl, bl, tr, tl]);
        triangle_count = 6;
    }

    // Setup uniforms
    let uniform_offset = ctx.uniforms.len() as u32;
    if !is_convex {
        // Two uniforms: first = simple shader for stencil, second = fill shader
        let mut simple = FragUniforms::default();
        simple.stroke_params[1] = -1.0; // strokeThr = -1
        simple.stroke_params[3] = 2.0; // type = SIMPLE
        ctx.uniforms.push(simple);

        let fill_frag = convert_paint(ctx, paint, scissor, fringe, fringe, -1.0);
        ctx.uniforms.push(fill_frag);
    } else {
        let frag = convert_paint(ctx, paint, scissor, fringe, fringe, -1.0);
        ctx.uniforms.push(frag);
    }

    ctx.draw_calls.push(DrawCall {
        call_type,
        blend,
        image: paint.image,
        paths: path_data,
        triangle_offset,
        triangle_count,
        uniform_offset,
        fringe,
        stroke_width: 0.0,
    });
}

unsafe extern "C" fn render_stroke(
    uptr: *mut c_void,
    paint: *mut ffi::NVGpaint,
    composite_operation: ffi::NVGcompositeOperationState,
    scissor: *mut ffi::NVGscissor,
    fringe: f32,
    stroke_width: f32,
    paths: *const ffi::NVGpath,
    npaths: c_int,
) {
    let ctx = unsafe { ctx_from_uptr(uptr) };
    let paint = unsafe { &*paint };
    let scissor = unsafe { &*scissor };
    let paths_slice = unsafe { std::slice::from_raw_parts(paths, npaths as usize) };

    let blend = blend_composite_operation(&composite_operation);
    let stencil_strokes = ctx.flags & ffi::NVG_STENCIL_STROKES != 0;

    let mut path_data = Vec::with_capacity(npaths as usize);

    for path in paths_slice {
        let mut pd = PathData::default();
        if path.nstroke > 0 {
            let stroke_verts =
                unsafe { std::slice::from_raw_parts(path.stroke, path.nstroke as usize) };
            let triangles = strip_to_triangles(stroke_verts);
            if !triangles.is_empty() {
                pd.stroke_offset = ctx.vertices.len() as u32;
                pd.stroke_count = triangles.len() as u32;
                ctx.vertices.extend_from_slice(&triangles);
            }
        }
        path_data.push(pd);
    }

    let uniform_offset = ctx.uniforms.len() as u32;

    if stencil_strokes {
        // Two uniforms: first with strokeThr=-1 (AA pass), second with strokeThr=1-0.5/255
        let frag0 = convert_paint(ctx, paint, scissor, stroke_width, fringe, -1.0);
        ctx.uniforms.push(frag0);
        let frag1 =
            convert_paint(ctx, paint, scissor, stroke_width, fringe, 1.0 - 0.5 / 255.0);
        ctx.uniforms.push(frag1);
    } else {
        let frag = convert_paint(ctx, paint, scissor, stroke_width, fringe, -1.0);
        ctx.uniforms.push(frag);
    }

    ctx.draw_calls.push(DrawCall {
        call_type: CallType::Stroke,
        blend,
        image: paint.image,
        paths: path_data,
        triangle_offset: 0,
        triangle_count: 0,
        uniform_offset,
        fringe,
        stroke_width,
    });
}

unsafe extern "C" fn render_triangles(
    uptr: *mut c_void,
    paint: *mut ffi::NVGpaint,
    composite_operation: ffi::NVGcompositeOperationState,
    scissor: *mut ffi::NVGscissor,
    verts: *const ffi::NVGvertex,
    nverts: c_int,
    fringe: f32,
) {
    let ctx = unsafe { ctx_from_uptr(uptr) };
    let paint = unsafe { &*paint };
    let scissor = unsafe { &*scissor };
    let verts_slice = unsafe { std::slice::from_raw_parts(verts, nverts as usize) };

    let blend = blend_composite_operation(&composite_operation);

    let triangle_offset = ctx.vertices.len() as u32;
    ctx.vertices.extend_from_slice(verts_slice);
    let triangle_count = nverts as u32;

    let uniform_offset = ctx.uniforms.len() as u32;
    let mut frag = convert_paint(ctx, paint, scissor, 1.0, fringe, -1.0);
    frag.stroke_params[3] = 3.0; // type = IMG (shader for textured triangles)
    ctx.uniforms.push(frag);

    ctx.draw_calls.push(DrawCall {
        call_type: CallType::Triangles,
        blend,
        image: paint.image,
        paths: Vec::new(),
        triangle_offset,
        triangle_count,
        uniform_offset,
        fringe,
        stroke_width: 0.0,
    });
}

unsafe extern "C" fn render_delete(uptr: *mut c_void) {
    eprintln!("nanovg-wgpu: renderDelete — freeing WgpuNvgContext");
    if !uptr.is_null() {
        let _ = unsafe { Box::from_raw(uptr as *mut WgpuNvgContext) };
    }
}

/// No-op render_create for shared contexts — the primary already initialized pipelines.
unsafe extern "C" fn render_create_shared(_uptr: *mut c_void, _other_uptr: *mut c_void) -> c_int {
    eprintln!("nanovg-wgpu: renderCreate (shared) — reusing primary resources");
    1
}

/// No-op render_delete for shared contexts — the primary owns the WgpuNvgContext.
unsafe extern "C" fn render_delete_noop(_uptr: *mut c_void) {
    // Don't free — the primary context owns the WgpuNvgContext
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

/// Create a second NanoVG context that shares the wgpu backend (textures,
/// pipelines, device) with `primary`. This is needed because Rack uses
/// two NanoVG contexts: `vg` for drawing and `fbVg` for rendering into
/// offscreen framebuffers. They must be separate NanoVG contexts (separate
/// transform/style stacks) but share GPU resources.
///
/// The returned context shares the same `WgpuNvgContext` as the primary.
/// `render_delete` on the shared context is a no-op (the primary owns it).
pub fn create_shared_context(
    primary: *mut ffi::NVGcontext,
    flags: c_int,
) -> *mut ffi::NVGcontext {
    if primary.is_null() {
        return std::ptr::null_mut();
    }
    let params = unsafe { ffi::nvgInternalParams(primary) };
    if params.is_null() {
        return std::ptr::null_mut();
    }
    let user_ptr = unsafe { (*params).user_ptr };

    // Create new NVGparams pointing to the SAME WgpuNvgContext.
    // render_delete is replaced with a no-op since the primary owns the backend.
    let mut shared_params = ffi::NVGparams {
        user_ptr,
        edge_anti_alias: if flags & ffi::NVG_ANTIALIAS != 0 { 1 } else { 0 },
        render_create: Some(render_create_shared),
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
        render_delete: Some(render_delete_noop),
    };

    // Pass the primary as `other` so NanoVG shares fonts
    let nvg_ctx = unsafe { ffi::nvgCreateInternal(&mut shared_params, primary) };
    if nvg_ctx.is_null() {
        eprintln!("nanovg-wgpu: nvgCreateInternal (shared) returned null!");
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
