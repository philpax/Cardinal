use cardinal_core::{self as cc, CableId, ModuleId};
use std::sync::mpsc;
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId, WindowAttributes};

// ── Messages between UI and Cardinal thread ─────────────────────────

struct EventResult {
    consumed: bool,
    port_drag: Option<cc::PortDragInfo>,
}

enum Command {
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

struct ModuleInfo {
    id: ModuleId,
    size: (f32, f32),
    inputs: Vec<cc::PortInfo>,
    outputs: Vec<cc::PortInfo>,
}

struct RenderResult {
    module_id: ModuleId,
    #[allow(dead_code)]
    width: u32,
    #[allow(dead_code)]
    height: u32,
    texture: wgpu::Texture,
}

// ── Cardinal thread ─────────────────────────────────────────────────

fn spawn_cardinal_thread(
    sample_rate: f32,
) -> (mpsc::Sender<Command>, mpsc::Receiver<RenderResult>) {
    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
    let (render_tx, render_rx) = mpsc::channel::<RenderResult>();

    std::thread::Builder::new()
        .name("cardinal".into())
        .spawn(move || {
            // All Cardinal state lives on this thread
            let resource_dir = cc::default_resource_dir();
            cc::init(sample_rate, &resource_dir);

            #[allow(unused_assignments)]
            let mut nanovg_ctx: *mut cardinal_core::ffi::NVGcontext = std::ptr::null_mut();
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
                    } => {
                        if nanovg_ctx.is_null() { continue; }
                        let device = gpu_device.as_ref().unwrap();
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
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                            view_formats: &[],
                        });
                        let view = texture.create_view(&Default::default());

                        // Set render target and render
                        cardinal_core::nanovg_wgpu::set_render_target(nanovg_ctx, view, w, h);
                        unsafe { cardinal_core::ffi::nvgBeginFrame(nanovg_ctx, w as f32, h as f32, 1.0) };
                        let ok = cc::module_render(module_id, nanovg_ctx, w as i32, h as i32);
                        unsafe { cardinal_core::ffi::nvgEndFrame(nanovg_ctx) };

                        // Only send the texture if the module actually rendered
                        // (modules without widgets return false)
                        if ok {
                            let _ = render_tx.send(RenderResult {
                                module_id,
                                width: w,
                                height: h,
                                texture,
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
                        let flags = cardinal_core::ffi::NVG_ANTIALIAS | cardinal_core::ffi::NVG_STENCIL_STROKES;
                        nanovg_ctx = cardinal_core::nanovg_wgpu::create_context(
                            device, queue, flags,
                        );
                        // Create a shared context for fbVg (used by FramebufferWidget
                        // for offscreen rendering). It shares the same wgpu backend
                        // (textures, pipelines) but has its own NanoVG state.
                        let fb_ctx = cardinal_core::nanovg_wgpu::create_shared_context(
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

// ── Audio backend (cpal) ─────────────────────────────────────────────

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn cpal_sample_rate() -> Option<f32> {
    let host = cpal::default_host();
    let device = host.default_output_device()?;
    let config = device.default_output_config().ok()?;
    Some(config.sample_rate().0 as f32)
}

fn start_audio_stream() -> Option<cpal::Stream> {
    let host = cpal::default_host();
    let device = host.default_output_device()?;
    let config = device.default_output_config().ok()?;

    eprintln!(
        "Audio device: {}, config: {:?}",
        device.name().unwrap_or_default(),
        config
    );

    let channels = config.channels() as usize;
    let sample_rate = config.sample_rate().0;

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_output_stream(
                &cpal::StreamConfig {
                    channels: channels as u16,
                    sample_rate: cpal::SampleRate(sample_rate),
                    buffer_size: cpal::BufferSize::Default,
                },
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    audio_callback(data, channels);
                },
                |err| eprintln!("Audio stream error: {err}"),
                None,
            )
            .ok()?,
        _ => {
            eprintln!("Unsupported sample format: {:?}", config.sample_format());
            return None;
        }
    };

    stream.play().ok()?;
    eprintln!("Audio stream started");
    Some(stream)
}

fn audio_callback(output: &mut [f32], channels: usize) {
    let frames = output.len() / channels;
    const MAX: usize = 8192;
    let frames = frames.min(MAX);
    let mut stereo_buf = [0.0f32; MAX * 2];

    // audio_process calls engine->stepBlock which has internal locking
    cc::audio_process(frames, None, &mut stereo_buf[..frames * 2]);

    for i in 0..frames {
        let l = stereo_buf[i * 2];
        let r = stereo_buf[i * 2 + 1];
        let base = i * channels;
        if channels >= 1 {
            output[base] = l;
        }
        if channels >= 2 {
            output[base + 1] = r;
        }
        for ch in 2..channels {
            output[base + ch] = 0.0;
        }
    }
    let written = frames * channels;
    for s in &mut output[written..] {
        *s = 0.0;
    }
}

// ── App state ────────────────────────────────────────────────────────

struct PlacedModule {
    id: ModuleId,
    name: String,
    pos: egui::Pos2,
    size: egui::Vec2,
    inputs: Vec<cc::PortInfo>,
    outputs: Vec<cc::PortInfo>,
    texture_id: Option<egui::TextureId>,
    render_texture: Option<wgpu::Texture>,
}

struct Cable {
    _id: CableId,
    out_module: ModuleId,
    out_port: i32,
    in_module: ModuleId,
    in_port: i32,
}

enum DragState {
    Cable {
        from_module: ModuleId,
        from_port: i32,
        is_output: bool,
        mouse_pos: egui::Pos2,
    },
    Module {
        module_idx: usize,
    },
}

struct App {
    modules: Vec<PlacedModule>,
    cables: Vec<Cable>,
    catalog: Vec<cc::CatalogEntry>,
    drag: Option<DragState>,
    active_module_drag: Option<ModuleId>,
    browser_filter: String,
    cmd_tx: mpsc::Sender<Command>,
    render_rx: mpsc::Receiver<RenderResult>,
}

fn egui_mods_to_rack(modifiers: &egui::Modifiers) -> i32 {
    let mut mods = 0i32;
    if modifiers.shift {
        mods |= 1;
    }
    if modifiers.ctrl {
        mods |= 2;
    }
    if modifiers.alt {
        mods |= 4;
    }
    if modifiers.mac_cmd || modifiers.command {
        mods |= 8;
    }
    mods
}

impl App {
    fn new(cmd_tx: mpsc::Sender<Command>, render_rx: mpsc::Receiver<RenderResult>) -> Self {
        // Request catalog from Cardinal thread
        let (cat_tx, cat_rx) = mpsc::channel();
        cmd_tx.send(Command::GetCatalog(cat_tx)).unwrap();
        let catalog = cat_rx.recv().unwrap_or_default();

        Self {
            modules: Vec::new(),
            cables: Vec::new(),
            catalog,
            drag: None,
            active_module_drag: None,
            browser_filter: String::new(),
            cmd_tx,
            render_rx,
        }
    }

    fn spawn_module(&mut self, plugin: &str, model: &str, pos: egui::Pos2) {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self.cmd_tx.send(Command::CreateModule {
            plugin: plugin.to_string(),
            model: model.to_string(),
            reply: reply_tx,
        });
        if let Ok(Some(info)) = reply_rx.recv() {
            let name = self
                .catalog
                .iter()
                .find(|e| e.plugin_slug == plugin && e.model_slug == model)
                .map_or(model.to_string(), |e| e.model_name.clone());
            self.modules.push(PlacedModule {
                id: info.id,
                name,
                pos,
                size: egui::vec2(info.size.0, info.size.1),
                inputs: info.inputs,
                outputs: info.outputs,
                texture_id: None,
                render_texture: None,
            });
        }
    }

    fn port_world_pos(m: &PlacedModule, port: &cc::PortInfo) -> egui::Pos2 {
        m.pos + egui::vec2(port.x, port.y)
    }

    fn find_port_at(&self, pos: egui::Pos2) -> Option<(ModuleId, i32, bool)> {
        let r = 12.0;
        for m in &self.modules {
            for p in &m.inputs {
                if Self::port_world_pos(m, p).distance(pos) < r {
                    return Some((m.id, p.id, false));
                }
            }
            for p in &m.outputs {
                if Self::port_world_pos(m, p).distance(pos) < r {
                    return Some((m.id, p.id, true));
                }
            }
        }
        None
    }

    fn send_module_event(
        &self,
        module_id: ModuleId,
        event_type: i32,
        x: f32,
        y: f32,
        button: i32,
        action: i32,
        mods: i32,
        scroll_x: f32,
        scroll_y: f32,
    ) -> Option<EventResult> {
        if event_type == cc::EVENT_BUTTON && action == 1 {
            let (reply_tx, reply_rx) = mpsc::channel();
            let _ = self.cmd_tx.send(Command::ModuleEvent {
                module_id,
                event_type,
                x,
                y,
                button,
                action,
                mods,
                scroll_x,
                scroll_y,
                reply: Some(reply_tx),
            });
            reply_rx.recv().ok()
        } else {
            let _ = self.cmd_tx.send(Command::ModuleEvent {
                module_id,
                event_type,
                x,
                y,
                button,
                action,
                mods,
                scroll_x,
                scroll_y,
                reply: None,
            });
            None
        }
    }

    fn request_renders(&self) {
        for m in &self.modules {
            let _ = self.cmd_tx.send(Command::RenderModule {
                module_id: m.id,
                width: m.size.x as i32,
                height: m.size.y as i32,
            });
        }
    }

    fn poll_render_results(&mut self, renderer: &mut egui_wgpu::Renderer, device: &wgpu::Device) {
        while let Ok(result) = self.render_rx.try_recv() {
            if let Some(m) = self.modules.iter_mut().find(|m| m.id == result.module_id) {
                let view = result.texture.create_view(&wgpu::TextureViewDescriptor::default());

                if let Some(tex_id) = m.texture_id {
                    renderer.update_egui_texture_from_wgpu_texture(
                        device, &view, wgpu::FilterMode::Linear, tex_id,
                    );
                } else {
                    let tex_id = renderer.register_native_texture(
                        device, &view, wgpu::FilterMode::Linear,
                    );
                    m.texture_id = Some(tex_id);
                }
                m.render_texture = Some(result.texture);
            }
        }
    }

    fn ui(&mut self, ctx: &egui::Context) {
        // ── Side panel: Module Browser ───────────────────────────────
        #[allow(deprecated)]
        egui::Panel::left("browser")
            .min_size(200.0)
            .show(ctx, |ui| {
                ui.heading("Module Browser");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Filter:");
                    ui.text_edit_singleline(&mut self.browser_filter);
                });
                ui.separator();

                ui.label(
                    egui::RichText::new("Click to add | Drag ports to cable | Interact with widgets")
                        .small()
                        .weak(),
                );
                ui.add_space(4.0);

                let catalog = self.catalog.clone();
                let filter = self.browser_filter.to_lowercase();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut current_plugin = String::new();
                    for entry in &catalog {
                        if !filter.is_empty()
                            && !entry.model_name.to_lowercase().contains(&filter)
                            && !entry.plugin_slug.to_lowercase().contains(&filter)
                        {
                            continue;
                        }

                        if entry.plugin_slug != current_plugin {
                            current_plugin = entry.plugin_slug.clone();
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(&current_plugin)
                                    .strong()
                                    .color(egui::Color32::from_rgb(150, 180, 220)),
                            );
                        }

                        if ui
                            .add(egui::Button::new(&entry.model_name).min_size(egui::vec2(180.0, 0.0)))
                            .clicked()
                        {
                            let x = 220.0 + self.modules.len() as f32 * 20.0;
                            let y = 50.0 + (self.modules.len() % 3) as f32 * 120.0;
                            self.spawn_module(
                                &entry.plugin_slug,
                                &entry.model_slug,
                                egui::pos2(x, y),
                            );
                        }
                    }
                });

                ui.separator();
                ui.label(format!(
                    "Modules: {} | Cables: {} | SR: {} Hz",
                    self.modules.len(),
                    self.cables.len(),
                    cc::sample_rate(),
                ));
            });

        // ── Central panel: Rack ──────────────────────────────────────
        #[allow(deprecated)]
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.max_rect();

            let mut responses: Vec<(usize, egui::Response)> = Vec::new();
            for (idx, m) in self.modules.iter().enumerate() {
                let module_rect = egui::Rect::from_min_size(m.pos, m.size);
                if rect.intersects(module_rect) {
                    let resp = ui.allocate_rect(module_rect, egui::Sense::click_and_drag());
                    responses.push((idx, resp));
                }
            }

            let mut drag_completed: Option<(ModuleId, i32, bool, egui::Pos2)> = None;

            for (idx, response) in &responses {
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let m = &self.modules[*idx];
                        let local_x = pos.x - m.pos.x;
                        let local_y = pos.y - m.pos.y;
                        let mods = egui_mods_to_rack(&ctx.input(|i| i.modifiers));

                        if let Some(result) = self.send_module_event(
                            m.id,
                            cc::EVENT_BUTTON,
                            local_x,
                            local_y,
                            0,
                            1,
                            mods,
                            0.0,
                            0.0,
                        ) {
                            if result.consumed {
                                if let Some(port_info) = result.port_drag {
                                    let _ = self.cmd_tx.send(Command::SetIncompleteCable {
                                        module_id: m.id,
                                        port_id: port_info.port_id,
                                        is_output: port_info.is_output,
                                    });
                                    self.drag = Some(DragState::Cable {
                                        from_module: m.id,
                                        from_port: port_info.port_id,
                                        is_output: port_info.is_output,
                                        mouse_pos: pos,
                                    });
                                } else {
                                    self.active_module_drag = Some(m.id);
                                }
                            } else {
                                self.drag =
                                    Some(DragState::Module { module_idx: *idx });
                            }
                        } else {
                            self.drag = Some(DragState::Module { module_idx: *idx });
                        }
                    }
                }

                if response.dragged() {
                    match &mut self.drag {
                        Some(DragState::Cable { mouse_pos, .. }) => {
                            if let Some(pos) = response.interact_pointer_pos() {
                                *mouse_pos = pos;
                            }
                        }
                        Some(DragState::Module { module_idx }) => {
                            self.modules[*module_idx].pos += response.drag_delta();
                        }
                        None => {
                            if let Some(active_id) = self.active_module_drag {
                                if let Some(pos) = response.interact_pointer_pos() {
                                    if let Some(m) =
                                        self.modules.iter().find(|m| m.id == active_id)
                                    {
                                        let local_x = pos.x - m.pos.x;
                                        let local_y = pos.y - m.pos.y;
                                        self.send_module_event(
                                            active_id,
                                            cc::EVENT_HOVER,
                                            local_x,
                                            local_y,
                                            0,
                                            0,
                                            0,
                                            0.0,
                                            0.0,
                                        );
                                    }
                                }
                            } else {
                                self.modules[*idx].pos += response.drag_delta();
                            }
                        }
                    }
                }

                if response.drag_stopped() {
                    if let Some(DragState::Cable {
                        from_module,
                        from_port,
                        is_output,
                        ..
                    }) = self.drag.take()
                    {
                        let _ = self.cmd_tx.send(Command::ClearIncompleteCable);
                        if let Some(pos) = response.interact_pointer_pos() {
                            drag_completed =
                                Some((from_module, from_port, is_output, pos));
                        }
                    } else if let Some(active_id) = self.active_module_drag.take() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            if let Some(m) =
                                self.modules.iter().find(|m| m.id == active_id)
                            {
                                let local_x = pos.x - m.pos.x;
                                let local_y = pos.y - m.pos.y;
                                self.send_module_event(
                                    active_id,
                                    cc::EVENT_BUTTON,
                                    local_x,
                                    local_y,
                                    0,
                                    0,
                                    0,
                                    0.0,
                                    0.0,
                                );
                            }
                        }
                    } else {
                        self.drag = None;
                    }
                }

                if response.double_clicked() {
                    let mid = self.modules[*idx].id;
                    self.cables.retain(|c| c.out_module != mid && c.in_module != mid);
                    let _ = self.cmd_tx.send(Command::DestroyModule(mid));
                    self.modules.remove(*idx);
                    break;
                }
            }

            if let Some((from_mod, from_port, is_output, end_pos)) = drag_completed {
                if let Some((to_mod, to_port, to_is_output)) = self.find_port_at(end_pos) {
                    let (om, op, im, ip) = if is_output && !to_is_output {
                        (from_mod, from_port, to_mod, to_port)
                    } else if !is_output && to_is_output {
                        (to_mod, to_port, from_mod, from_port)
                    } else {
                        (from_mod, -1, to_mod, -1)
                    };
                    if op >= 0 && ip >= 0 {
                        let (reply_tx, reply_rx) = mpsc::channel();
                        let _ = self.cmd_tx.send(Command::CreateCable {
                            out_mod: om,
                            out_port: op,
                            in_mod: im,
                            in_port: ip,
                            reply: reply_tx,
                        });
                        if let Ok(Some(cid)) = reply_rx.recv() {
                            self.cables.push(Cable {
                                _id: cid,
                                out_module: om,
                                out_port: op,
                                in_module: im,
                                in_port: ip,
                            });
                        }
                    }
                }
            }

            // Forward hover events when not dragging
            if self.drag.is_none() && self.active_module_drag.is_none() {
                if let Some(hover_pos) = ctx.pointer_hover_pos() {
                    for m in &self.modules {
                        let module_rect = egui::Rect::from_min_size(m.pos, m.size);
                        if module_rect.contains(hover_pos) {
                            let local_x = hover_pos.x - m.pos.x;
                            let local_y = hover_pos.y - m.pos.y;
                            self.send_module_event(
                                m.id,
                                cc::EVENT_HOVER,
                                local_x,
                                local_y,
                                0,
                                0,
                                0,
                                0.0,
                                0.0,
                            );
                            break;
                        }
                    }
                }
            }

            // Forward scroll events
            let scroll_delta = ctx.input(|i| i.smooth_scroll_delta);
            if scroll_delta != egui::Vec2::ZERO {
                if let Some(hover_pos) = ctx.pointer_hover_pos() {
                    for m in &self.modules {
                        let module_rect = egui::Rect::from_min_size(m.pos, m.size);
                        if module_rect.contains(hover_pos) {
                            let local_x = hover_pos.x - m.pos.x;
                            let local_y = hover_pos.y - m.pos.y;
                            self.send_module_event(
                                m.id,
                                cc::EVENT_SCROLL,
                                local_x,
                                local_y,
                                0,
                                0,
                                0,
                                scroll_delta.x,
                                scroll_delta.y,
                            );
                            break;
                        }
                    }
                }
            }

            // ── Paint ────────────────────────────────────────────────
            let painter = ui.painter();
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 30, 35));

            // Cables
            for cable in &self.cables {
                let om = self.modules.iter().find(|m| m.id == cable.out_module);
                let im = self.modules.iter().find(|m| m.id == cable.in_module);
                if let (Some(om), Some(im)) = (om, im) {
                    let op = om.outputs.iter().find(|p| p.id == cable.out_port);
                    let ip = im.inputs.iter().find(|p| p.id == cable.in_port);
                    if let (Some(op), Some(ip)) = (op, ip) {
                        let p1 = Self::port_world_pos(om, op);
                        let p2 = Self::port_world_pos(im, ip);
                        let mid_y = p1.y.max(p2.y) + 30.0 + (p1.x - p2.x).abs() * 0.15;
                        let c1 = egui::pos2(p1.x, mid_y);
                        let c2 = egui::pos2(p2.x, mid_y);
                        let pts: Vec<egui::Pos2> = (0..=20)
                            .map(|j| {
                                let t = j as f32 / 20.0;
                                let u = 1.0 - t;
                                egui::pos2(
                                    u * u * u * p1.x
                                        + 3.0 * u * u * t * c1.x
                                        + 3.0 * u * t * t * c2.x
                                        + t * t * t * p2.x,
                                    u * u * u * p1.y
                                        + 3.0 * u * u * t * c1.y
                                        + 3.0 * u * t * t * c2.y
                                        + t * t * t * p2.y,
                                )
                            })
                            .collect();
                        painter.add(egui::Shape::line(
                            pts,
                            egui::Stroke::new(3.0, egui::Color32::from_rgb(180, 200, 120)),
                        ));
                    }
                }
            }

            // Drag cable preview
            if let Some(DragState::Cable {
                from_module,
                from_port,
                is_output,
                mouse_pos,
            }) = &self.drag
            {
                if let Some(m) = self.modules.iter().find(|m| m.id == *from_module) {
                    let ports = if *is_output { &m.outputs } else { &m.inputs };
                    if let Some(p) = ports.iter().find(|p| p.id == *from_port) {
                        let start = Self::port_world_pos(m, p);
                        painter.line_segment(
                            [start, *mouse_pos],
                            egui::Stroke::new(2.0, egui::Color32::YELLOW),
                        );
                    }
                }
            }

            // Modules
            for m in &self.modules {
                let mr = egui::Rect::from_min_size(m.pos, m.size);
                if let Some(tex_id) = m.texture_id {
                    painter.image(
                        tex_id,
                        mr,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                } else {
                    painter.rect_filled(mr, 0.0, egui::Color32::from_rgb(40, 42, 48));
                    painter.text(
                        m.pos + egui::vec2(m.size.x / 2.0, 15.0),
                        egui::Align2::CENTER_CENTER,
                        &m.name,
                        egui::FontId::proportional(11.0),
                        egui::Color32::GRAY,
                    );
                    // Draw port indicators with short labels
                    for p in &m.inputs {
                        let pos = Self::port_world_pos(m, p);
                        painter.circle_filled(pos, 6.0, egui::Color32::from_rgb(60, 120, 200));
                        painter.text(pos + egui::vec2(10.0, 0.0), egui::Align2::LEFT_CENTER,
                            format!("in {}", p.id), egui::FontId::proportional(9.0), egui::Color32::LIGHT_GRAY);
                    }
                    for p in &m.outputs {
                        let pos = Self::port_world_pos(m, p);
                        painter.circle_filled(pos, 6.0, egui::Color32::from_rgb(200, 120, 60));
                        painter.text(pos + egui::vec2(-10.0, 0.0), egui::Align2::RIGHT_CENTER,
                            format!("out {}", p.id), egui::FontId::proportional(9.0), egui::Color32::LIGHT_GRAY);
                    }
                }
            }
        });

        ctx.request_repaint();
    }
}

// ── Wgpu + winit application handler ────────────────────────────────

struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    egui_renderer: egui_wgpu::Renderer,
    egui_state: egui_winit::State,
    egui_ctx: egui::Context,
}

struct WgpuApp {
    gpu: Option<GpuState>,
    app: Option<App>,
    cmd_tx: mpsc::Sender<Command>,
    render_rx: Option<mpsc::Receiver<RenderResult>>,
    _audio_stream: Option<cpal::Stream>,
}

impl WgpuApp {
    fn new(
        cmd_tx: mpsc::Sender<Command>,
        render_rx: mpsc::Receiver<RenderResult>,
        audio_stream: Option<cpal::Stream>,
    ) -> Self {
        Self {
            gpu: None,
            app: None,
            cmd_tx,
            render_rx: Some(render_rx),
            _audio_stream: audio_stream,
        }
    }
}

impl ApplicationHandler for WgpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }

        let window_attrs = WindowAttributes::default()
            .with_title("Cardinal")
            .with_inner_size(winit::dpi::LogicalSize::new(1400.0, 800.0));

        let window = Arc::new(event_loop.create_window(window_attrs).expect("failed to create window"));

        // Create wgpu instance, surface, adapter, device
        let instance = wgpu::Instance::default();

        let surface = instance.create_surface(window.clone()).expect("failed to create surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("failed to find a suitable adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("cardinal_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
        ))
        .expect("failed to create device");

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // Configure surface
        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Send GPU device to Cardinal thread
        let _ = self.cmd_tx.send(Command::InitGpu {
            device: device.clone(),
            queue: queue.clone(),
        });

        // Create egui context and renderer
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            surface_format,
            egui_wgpu::RendererOptions::default(),
        );

        // Create App
        let render_rx = self.render_rx.take().expect("render_rx already taken");
        let app = App::new(self.cmd_tx.clone(), render_rx);

        self.gpu = Some(GpuState {
            window,
            surface,
            surface_config,
            device,
            queue,
            egui_renderer,
            egui_state,
            egui_ctx,
        });
        self.app = Some(app);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let (Some(gpu), Some(app)) = (self.gpu.as_mut(), self.app.as_mut()) else {
            return;
        };

        // Let egui process the event first
        let response = gpu.egui_state.on_window_event(&gpu.window, &event);
        if response.consumed {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if new_size.width > 0 && new_size.height > 0 {
                    gpu.surface_config.width = new_size.width;
                    gpu.surface_config.height = new_size.height;
                    gpu.surface.configure(&gpu.device, &gpu.surface_config);
                    gpu.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                // 0. Poll render results and request new renders
                app.poll_render_results(&mut gpu.egui_renderer, &gpu.device);
                app.request_renders();

                // 1. Begin egui frame
                let raw_input = gpu.egui_state.take_egui_input(&gpu.window);
                #[allow(deprecated)]
                let full_output = gpu.egui_ctx.run(raw_input, |ctx| {
                    app.ui(ctx);
                });

                // 2. Handle egui platform output
                gpu.egui_state.handle_platform_output(&gpu.window, full_output.platform_output);

                // 3. Tessellate
                let paint_jobs = gpu.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
                let screen_descriptor = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [gpu.surface_config.width, gpu.surface_config.height],
                    pixels_per_point: gpu.egui_ctx.pixels_per_point(),
                };

                // 4. Render
                let surface_texture = match gpu.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(tex)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
                    wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                        gpu.surface.configure(&gpu.device, &gpu.surface_config);
                        gpu.window.request_redraw();
                        return;
                    }
                    wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                        return;
                    }
                    wgpu::CurrentSurfaceTexture::Validation => {
                        eprintln!("wgpu surface validation error");
                        return;
                    }
                };

                let surface_view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("egui_encoder"),
                });

                // Upload textures
                for (id, delta) in &full_output.textures_delta.set {
                    gpu.egui_renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
                }

                gpu.egui_renderer.update_buffers(
                    &gpu.device,
                    &gpu.queue,
                    &mut encoder,
                    &paint_jobs,
                    &screen_descriptor,
                );

                {
                    let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("egui_render"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &surface_view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.1,
                                    g: 0.1,
                                    b: 0.12,
                                    a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        ..Default::default()
                    });
                    let mut render_pass = render_pass.forget_lifetime();
                    gpu.egui_renderer.render(&mut render_pass, &paint_jobs, &screen_descriptor);
                }

                gpu.queue.submit(std::iter::once(encoder.finish()));
                surface_texture.present();

                // Free textures
                for id in &full_output.textures_delta.free {
                    gpu.egui_renderer.free_texture(id);
                }

                // Request next frame
                gpu.window.request_redraw();
            }
            _ => {}
        }
    }
}

// ── Main ─────────────────────────────────────────────────────────────

fn main() {
    let sample_rate = cpal_sample_rate().unwrap_or(48000.0);
    eprintln!("Audio sample rate: {sample_rate} Hz");

    // Spawn the Cardinal thread — all engine/plugin/GL state lives there
    let (cmd_tx, render_rx) = spawn_cardinal_thread(sample_rate);

    // Start cpal audio stream (calls audio_process on its own thread;
    // engine->stepBlock has internal locking)
    let audio_stream = start_audio_stream();

    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut wgpu_app = WgpuApp::new(cmd_tx, render_rx, audio_stream);
    event_loop.run_app(&mut wgpu_app).expect("event loop error");
}
