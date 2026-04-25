use crate::{self as cc, CableId, ModuleId};
use std::sync::mpsc;
use std::sync::Arc;

// ── Messages between UI and Cardinal thread ─────────────────────────

pub struct EventResult {
    pub consumed: bool,
    pub port_drag: Option<cc::PortDragInfo>,
}

pub enum Command {
    CreateModule {
        plugin: String,
        model: String,
        reply: mpsc::Sender<Option<ModuleInfo>>,
    },
    DestroyModule(ModuleId),
    CreateCable {
        out_mod: ModuleId,
        out_port: i32,
        in_mod: ModuleId,
        in_port: i32,
        reply: mpsc::Sender<Option<CableId>>,
    },
    ModuleEvent {
        module_id: ModuleId,
        event_type: i32,
        x: f32,
        y: f32,
        button: i32,
        action: i32,
        mods: i32,
        scroll_x: f32,
        scroll_y: f32,
        reply: Option<mpsc::Sender<EventResult>>,
    },
    RenderModule {
        module_id: ModuleId,
        width: i32,
        height: i32,
        /// If provided, render to this pre-allocated texture instead of creating a new one.
        texture: Option<wgpu::Texture>,
        /// If true, CPU-readback the rendered pixels into `RenderResult::pixels`.
        /// Only the PNG-fallback path needs this; the dmatex path leaves it false.
        want_pixels: bool,
    },
    GetCatalog(mpsc::Sender<Vec<cc::CatalogEntry>>),
    SetIncompleteCable {
        module_id: ModuleId,
        port_id: i32,
        is_output: bool,
    },
    ClearIncompleteCable,
    InitGpu {
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
    },
}

pub struct ModuleInfo {
    pub id: ModuleId,
    pub size: (f32, f32),
    pub inputs: Vec<cc::PortInfo>,
    pub outputs: Vec<cc::PortInfo>,
    pub params: Vec<cc::ParamInfo>,
}

pub struct RenderResult {
    pub module_id: ModuleId,
    pub width: u32,
    pub height: u32,
    pub texture: wgpu::Texture,
    /// If set, contains CPU-readback RGBA8 pixels (for file-based texture fallback).
    pub pixels: Option<Vec<u8>>,
}

pub fn spawn_cardinal_thread(
    sample_rate: f32,
) -> (mpsc::Sender<Command>, mpsc::Receiver<RenderResult>) {
    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
    let (render_tx, render_rx) = mpsc::channel::<RenderResult>();

    std::thread::Builder::new()
        .name("cardinal".into())
        .spawn(move || {
            let resource_dir = cc::default_resource_dir();
            cc::init(sample_rate, &resource_dir);

            #[allow(unused_assignments)]
            let mut nanovg_ctx: *mut crate::ffi::NVGcontext = std::ptr::null_mut();
            let mut gpu_device: Option<Arc<wgpu::Device>> = None;
            let mut _gpu_queue: Option<Arc<wgpu::Queue>> = None;

            eprintln!("cardinal thread: ready");

            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    Command::CreateModule {
                        plugin,
                        model,
                        reply,
                    } => {
                        let info = cc::module_create(&plugin, &model).map(|id| {
                            let (w, h) = cc::module_size(id);
                            ModuleInfo {
                                id,
                                size: (w.max(90.0), h.max(200.0)),
                                inputs: cc::module_inputs(id),
                                outputs: cc::module_outputs(id),
                                params: cc::module_params(id),
                            }
                        });
                        let _ = reply.send(info);
                    }
                    Command::DestroyModule(id) => {
                        cc::module_destroy(id);
                    }
                    Command::CreateCable {
                        out_mod,
                        out_port,
                        in_mod,
                        in_port,
                        reply,
                    } => {
                        let id = cc::cable_create(out_mod, out_port, in_mod, in_port);
                        let _ = reply.send(id);
                    }
                    Command::ModuleEvent {
                        module_id,
                        event_type,
                        x,
                        y,
                        button,
                        action,
                        mods,
                        scroll_x,
                        scroll_y,
                        reply,
                    } => {
                        let consumed = cc::module_event(
                            module_id, event_type, x, y, button, action, mods, scroll_x,
                            scroll_y,
                        );
                        if let Some(reply) = reply {
                            let port_drag = if event_type == cc::EVENT_BUTTON
                                && action == 1
                                && consumed
                            {
                                cc::module_check_port_drag(module_id)
                            } else {
                                None
                            };
                            let _ = reply.send(EventResult {
                                consumed,
                                port_drag,
                            });
                        }
                    }
                    Command::RenderModule {
                        module_id,
                        width,
                        height,
                        texture: pre_allocated_texture,
                        want_pixels,
                    } => {
                        if nanovg_ctx.is_null() { continue; }
                        let w = width as u32;
                        let h = height as u32;

                        let texture = pre_allocated_texture.unwrap_or_else(|| {
                            let device = gpu_device.as_ref().unwrap();
                            device.create_texture(&wgpu::TextureDescriptor {
                                label: Some("nvg_render_target"),
                                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                                mip_level_count: 1,
                                sample_count: 1,
                                dimension: wgpu::TextureDimension::D2,
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
                                view_formats: &[],
                            })
                        });
                        let view = texture.create_view(&Default::default());

                        crate::nanovg_wgpu::set_render_target(nanovg_ctx, view, w, h);
                        unsafe { crate::ffi::nvgBeginFrame(nanovg_ctx, w as f32, h as f32, 1.0) };
                        let ok = cc::module_render(module_id, nanovg_ctx, w as i32, h as i32);
                        unsafe { crate::ffi::nvgEndFrame(nanovg_ctx) };

                        if ok {
                            // CPU readback only when the caller explicitly asks
                            // (PNG fallback path). The dmatex path uses GPU copy.
                            let pixels = if want_pixels {
                                let device = gpu_device.as_ref().unwrap();
                                let queue = _gpu_queue.as_ref().unwrap();
                                Some(cpu_readback_rgba8(device, queue, &texture, w, h))
                            } else {
                                None
                            };
                            let _ = render_tx.send(RenderResult {
                                module_id,
                                width: w,
                                height: h,
                                texture,
                                pixels,
                            });
                        }
                    }
                    Command::SetIncompleteCable { module_id, port_id, is_output } => {
                        cc::set_incomplete_cable(module_id, port_id, is_output);
                    }
                    Command::ClearIncompleteCable => {
                        cc::clear_incomplete_cable();
                    }
                    Command::GetCatalog(reply) => {
                        let _ = reply.send(cc::catalog());
                    }
                    Command::InitGpu { device, queue } => {
                        gpu_device = Some(device.clone());
                        _gpu_queue = Some(queue.clone());
                        let flags = crate::ffi::NVG_ANTIALIAS | crate::ffi::NVG_STENCIL_STROKES;
                        nanovg_ctx = crate::nanovg_wgpu::create_context(
                            device, queue, flags,
                        );
                        let fb_ctx = crate::nanovg_wgpu::create_shared_context(
                            nanovg_ctx, flags,
                        );
                        cc::set_vg(nanovg_ctx, fb_ctx);
                        eprintln!("cardinal thread: wgpu NanoVG contexts created (vg + fbVg)");
                    }
                }
            }
        })
        .expect("failed to spawn cardinal thread");

    (cmd_tx, render_rx)
}

/// Read back pixels from an Rgba8Unorm texture on the GPU.
fn cpu_readback_rgba8(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let bpp = 4u32;
    let unpadded = width * bpp;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded = (unpadded + align - 1) / align * align;

    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * height) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut enc = device.create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    queue.submit(std::iter::once(enc.finish()));

    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();

    let mapped = slice.get_mapped_range();
    let mut out = Vec::with_capacity((unpadded * height) as usize);
    for row in 0..height as usize {
        let s = row * padded as usize;
        out.extend_from_slice(&mapped[s..s + unpadded as usize]);
    }
    out
}
