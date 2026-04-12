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
        }
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
        multiview: None,
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
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::Always,
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

// ── Callback stubs ───────────────────────────────────────────────────

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
        bind_group_layouts: &[&view_bind_group_layout, &texture_bind_group_layout],
        push_constant_ranges: &[],
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
        mipmap_filter: wgpu::FilterMode::Linear,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    });

    let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("nvg_nearest_sampler"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
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
