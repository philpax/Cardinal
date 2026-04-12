use cardinal_core::{self as cc, CableId, ModuleId};
use eframe::egui;

fn main() -> eframe::Result {
    let sample_rate = cpal_sample_rate().unwrap_or(48000.0);
    eprintln!("Audio sample rate: {sample_rate} Hz");

    let resource_dir = cc::default_resource_dir();
    cc::init(sample_rate, &resource_dir);

    let audio_id = cc::audio_create();
    if audio_id.is_none() {
        eprintln!("Warning: failed to create audio I/O module");
    }

    let _audio_stream = start_audio_stream();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 800.0])
            .with_title("Cardinal"),
        ..Default::default()
    };

    eframe::run_native(
        "cardinal-egui",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
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
        cpal::SampleFormat::F32 => {
            device
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
                .ok()?
        }
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
    params: Vec<cc::ParamInfo>,
    texture: Option<egui::TextureHandle>,
    render_frame: u64,
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
    Knob {
        module_idx: usize,
        param_id: i32,
        start_value: f32,
        start_y: f32,
        min: f32,
        max: f32,
    },
    Module {
        module_idx: usize,
    },
}

use std::sync::mpsc;

/// Render request sent to the render thread.
struct RenderRequest {
    module_id: ModuleId,
    width: i32,
    height: i32,
}

/// Render result sent back from the render thread.
struct RenderResult {
    module_id: ModuleId,
    width: i32,
    height: i32,
    pixels: Vec<u8>,
}

fn spawn_render_thread() -> (mpsc::Sender<RenderRequest>, mpsc::Receiver<RenderResult>) {
    let (req_tx, req_rx) = mpsc::channel::<RenderRequest>();
    let (res_tx, res_rx) = mpsc::channel::<RenderResult>();

    std::thread::Builder::new()
        .name("cardinal-render".into())
        .spawn(move || {
            if !cc::render_claim_context() {
                eprintln!("cardinal-render: failed to claim GL context");
                return;
            }
            eprintln!("cardinal-render: GL context claimed, ready");

            while let Ok(req) = req_rx.recv() {
                if let Some((w, h, pixels)) = cc::module_render(req.module_id, req.width, req.height) {
                    let _ = res_tx.send(RenderResult {
                        module_id: req.module_id,
                        width: w,
                        height: h,
                        pixels,
                    });
                }
            }

            cc::render_release_context();
            eprintln!("cardinal-render: thread exiting");
        })
        .expect("failed to spawn render thread");

    (req_tx, res_rx)
}

struct App {
    modules: Vec<PlacedModule>,
    cables: Vec<Cable>,
    catalog: Vec<cc::CatalogEntry>,
    drag: Option<DragState>,
    frame_count: u64,
    browser_filter: String,
    render_tx: mpsc::Sender<RenderRequest>,
    render_rx: mpsc::Receiver<RenderResult>,
}

impl App {
    fn new() -> Self {
        let (render_tx, render_rx) = spawn_render_thread();
        Self {
            modules: Vec::new(),
            cables: Vec::new(),
            catalog: cc::catalog(),
            drag: None,
            frame_count: 0,
            browser_filter: String::new(),
            render_tx,
            render_rx,
        }
    }

    fn spawn_module(&mut self, plugin: &str, model: &str, pos: egui::Pos2) {
        if let Some(id) = cc::module_create(plugin, model) {
            let (w, h) = cc::module_size(id);
            let inputs = cc::module_inputs(id);
            let outputs = cc::module_outputs(id);
            let params = cc::module_params(id);
            let name = self
                .catalog
                .iter()
                .find(|e| e.plugin_slug == plugin && e.model_slug == model)
                .map_or(model.to_string(), |e| e.model_name.clone());

            self.modules.push(PlacedModule {
                id,
                name,
                pos,
                size: egui::vec2(w.max(90.0), h.max(200.0)),
                inputs,
                outputs,
                params,
                texture: None,
                render_frame: 0,
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

    fn find_knob_at(&self, pos: egui::Pos2) -> Option<(usize, &cc::ParamInfo)> {
        for (idx, m) in self.modules.iter().enumerate() {
            for param in &m.params {
                let pp = m.pos + egui::vec2(param.x, param.y);
                if pp.distance(pos) < 16.0 {
                    return Some((idx, param));
                }
            }
        }
        None
    }

    fn request_render(&self, idx: usize) {
        let m = &self.modules[idx];
        let w = m.size.x as i32;
        let h = m.size.y as i32;
        if w > 0 && h > 0 {
            let _ = self.render_tx.send(RenderRequest {
                module_id: m.id,
                width: w,
                height: h,
            });
        }
    }

    fn poll_render_results(&mut self, ctx: &egui::Context) {
        while let Ok(result) = self.render_rx.try_recv() {
            if let Some(m) = self.modules.iter_mut().find(|m| m.id == result.module_id) {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [result.width as _, result.height as _],
                    &result.pixels,
                );
                let tex = ctx.load_texture(
                    format!("mod_{}", result.module_id.0),
                    image,
                    egui::TextureOptions::LINEAR,
                );
                m.texture = Some(tex);
                m.render_frame = self.frame_count;
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_count += 1;

        // Poll for completed renders from the render thread
        self.poll_render_results(ctx);

        // Request re-renders every frame (async — doesn't block the UI)
        for i in 0..self.modules.len() {
            self.request_render(i);
        }

        // ── Side panel: Module Browser ───────────────────────────────
        egui::SidePanel::left("browser")
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Module Browser");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Filter:");
                    ui.text_edit_singleline(&mut self.browser_filter);
                });
                ui.separator();

                ui.label(
                    egui::RichText::new("Click a module to add it to the rack")
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
                            self.spawn_module(&entry.plugin_slug, &entry.model_slug, egui::pos2(x, y));
                        }
                    }
                });

                ui.separator();
                ui.heading("Status");
                ui.label(format!("Modules: {}", self.modules.len()));
                ui.label(format!("Cables: {}", self.cables.len()));
                ui.label(format!("SR: {} Hz", cc::sample_rate()));

                ui.add_space(8.0);
                ui.separator();
                ui.label(egui::RichText::new("Controls").strong());
                ui.label(egui::RichText::new("Drag module body to move").small());
                ui.label(egui::RichText::new("Drag from port to port to cable").small());
                ui.label(egui::RichText::new("Drag knob up/down to adjust").small());
            });

        // ── Central panel: Rack ──────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.max_rect();

            // Allocate rects for each module
            let mut responses: Vec<(usize, egui::Response)> = Vec::new();
            for (idx, m) in self.modules.iter().enumerate() {
                let module_rect = egui::Rect::from_min_size(m.pos, m.size);
                if rect.intersects(module_rect) {
                    let resp = ui.allocate_rect(module_rect, egui::Sense::click_and_drag());
                    responses.push((idx, resp));
                }
            }

            // Handle interactions
            let mut drag_completed: Option<(ModuleId, i32, bool, egui::Pos2)> = None;

            for (idx, response) in &responses {
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        // Check knob first
                        if let Some((knob_idx, param)) = self.find_knob_at(pos) {
                            let val = cc::module_get_param(self.modules[knob_idx].id, param.id);
                            self.drag = Some(DragState::Knob {
                                module_idx: knob_idx,
                                param_id: param.id,
                                start_value: val,
                                start_y: pos.y,
                                min: param.min,
                                max: param.max,
                            });
                        }
                        // Then port
                        else if let Some((mid, pid, is_out)) = self.find_port_at(pos) {
                            self.drag = Some(DragState::Cable {
                                from_module: mid,
                                from_port: pid,
                                is_output: is_out,
                                mouse_pos: pos,
                            });
                        }
                        // Otherwise move module
                        else {
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
                        Some(DragState::Knob {
                            module_idx,
                            param_id,
                            start_value,
                            start_y,
                            min,
                            max,
                        }) => {
                            if let Some(pos) = response.interact_pointer_pos() {
                                let dy = *start_y - pos.y; // up = increase
                                let range = *max - *min;
                                let sensitivity = range / 200.0;
                                let new_val = (*start_value + dy * sensitivity).clamp(*min, *max);
                                cc::module_set_param(
                                    self.modules[*module_idx].id,
                                    *param_id,
                                    new_val,
                                );
                            }
                        }
                        Some(DragState::Module { module_idx }) => {
                            self.modules[*module_idx].pos += response.drag_delta();
                        }
                        None => {
                            self.modules[*idx].pos += response.drag_delta();
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
                        if let Some(pos) = response.interact_pointer_pos() {
                            drag_completed = Some((from_module, from_port, is_output, pos));
                        }
                    } else {
                        self.drag = None;
                    }
                }

                // Double-click to remove module
                if response.double_clicked() {
                    let mid = self.modules[*idx].id;
                    // Remove cables connected to this module
                    self.cables.retain(|c| c.out_module != mid && c.in_module != mid);
                    cc::module_destroy(mid);
                    self.modules.remove(*idx);
                    break; // indices invalidated
                }
            }

            // Complete cable connection
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
                        if let Some(cid) = cc::cable_create(om, op, im, ip) {
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

            // ── Paint ────────────────────────────────────────────────
            let painter = ui.painter();
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 30, 35));

            // Draw cables
            for cable in &self.cables {
                let om = self.modules.iter().find(|m| m.id == cable.out_module);
                let im = self.modules.iter().find(|m| m.id == cable.in_module);
                if let (Some(om), Some(im)) = (om, im) {
                    let op = om.outputs.iter().find(|p| p.id == cable.out_port);
                    let ip = im.inputs.iter().find(|p| p.id == cable.in_port);
                    if let (Some(op), Some(ip)) = (op, ip) {
                        let p1 = Self::port_world_pos(om, op);
                        let p2 = Self::port_world_pos(im, ip);

                        let v = cc::module_get_output_voltage(cable.out_module, cable.out_port);
                        let intensity = (v.abs() / 5.0).clamp(0.0, 1.0);
                        let color = egui::Color32::from_rgb(
                            (100.0 + 155.0 * intensity) as u8,
                            (200.0 * (1.0 - intensity * 0.5)) as u8,
                            (80.0 + 100.0 * intensity) as u8,
                        );

                        let mid_y =
                            p1.y.max(p2.y) + 30.0 + (p1.x - p2.x).abs() * 0.15;
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
                        painter.add(egui::Shape::line(pts, egui::Stroke::new(3.0, color)));
                    }
                }
            }

            // Draw drag cable
            if let Some(DragState::Cable {
                from_module,
                from_port,
                is_output,
                mouse_pos,
            }) = &self.drag
            {
                if let Some(m) = self.modules.iter().find(|m| m.id == *from_module) {
                    let ports = if *is_output {
                        &m.outputs
                    } else {
                        &m.inputs
                    };
                    if let Some(p) = ports.iter().find(|p| p.id == *from_port) {
                        let start = Self::port_world_pos(m, p);
                        painter.line_segment(
                            [start, *mouse_pos],
                            egui::Stroke::new(2.0, egui::Color32::YELLOW),
                        );
                    }
                }
            }

            // Draw modules — use Cardinal's rendered widget texture
            for m in &self.modules {
                let mr = egui::Rect::from_min_size(m.pos, m.size);

                if let Some(tex) = &m.texture {
                    painter.image(
                        tex.id(),
                        mr,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                } else {
                    // Fallback until first render completes
                    painter.rect_filled(mr, 0.0, egui::Color32::from_rgb(40, 42, 48));
                    painter.text(
                        m.pos + egui::vec2(m.size.x / 2.0, m.size.y / 2.0),
                        egui::Align2::CENTER_CENTER,
                        &m.name,
                        egui::FontId::proportional(11.0),
                        egui::Color32::GRAY,
                    );
                }
            }
        });

        ctx.request_repaint();
    }
}
