use cardinal_core::{self as cc, CableId, ModuleId};
use eframe::egui;

fn main() -> eframe::Result {
    // Query the host audio sample rate before initialising the engine
    let sample_rate = cpal_sample_rate().unwrap_or(48000.0);
    eprintln!("Audio sample rate: {sample_rate} Hz");

    let resource_dir = cc::default_resource_dir();
    cc::init(sample_rate, &resource_dir);

    // Create the audio I/O terminal module
    let audio_id = cc::audio_create();
    if audio_id.is_none() {
        eprintln!("Warning: failed to create audio I/O module");
    }

    // Start the real-time audio thread
    let _audio_stream = start_audio_stream();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Cardinal XR Prototype"),
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
            device.build_output_stream(
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
            ).ok()?
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

    // Stack buffer to avoid allocation in the audio thread.
    // Max 8192 frames (from AUDIO_IO_MAX_FRAMES in bridge.cpp).
    const MAX: usize = 8192;
    let frames = frames.min(MAX);
    let mut stereo_buf = [0.0f32; MAX * 2];

    // Process through the Rack engine
    cc::audio_process(frames, None, &mut stereo_buf[..frames * 2]);

    // Write to output buffer (may be more than 2 channels)
    for i in 0..frames {
        let l = stereo_buf[i * 2];
        let r = stereo_buf[i * 2 + 1];
        let base = i * channels;
        if channels >= 1 { output[base] = l; }
        if channels >= 2 { output[base + 1] = r; }
        for ch in 2..channels { output[base + ch] = 0.0; }
    }
    // Zero any remaining frames if output is longer
    let written = frames * channels;
    for s in &mut output[written..] { *s = 0.0; }
}

struct PlacedModule {
    id: ModuleId,
    name: String,
    pos: egui::Pos2,
    size: egui::Vec2,
    inputs: Vec<cc::PortInfo>,
    outputs: Vec<cc::PortInfo>,
    params: Vec<cc::ParamInfo>,
}

struct Cable {
    _id: CableId,
    out_module: ModuleId,
    out_port: i32,
    in_module: ModuleId,
    in_port: i32,
}

struct DragCable {
    from_module: ModuleId,
    from_port: i32,
    is_output: bool,
    mouse_pos: egui::Pos2,
}

struct App {
    modules: Vec<PlacedModule>,
    cables: Vec<Cable>,
    catalog: Vec<cc::CatalogEntry>,
    drag_cable: Option<DragCable>,
}

impl App {
    fn new() -> Self {
        Self {
            modules: Vec::new(),
            cables: Vec::new(),
            catalog: cc::catalog(),
            drag_cable: None,
        }
    }

    fn spawn_module(&mut self, plugin: &str, model: &str, pos: egui::Pos2) {
        if let Some(id) = cc::module_create(plugin, model) {
            let (w, h) = cc::module_size(id);
            let inputs = cc::module_inputs(id);
            let outputs = cc::module_outputs(id);
            let params = cc::module_params(id);
            let name = self.catalog.iter()
                .find(|e| e.plugin_slug == plugin && e.model_slug == model)
                .map_or(model.to_string(), |e| e.model_name.clone());

            self.modules.push(PlacedModule {
                id, name, pos,
                size: egui::vec2(w.max(90.0), h.max(200.0)),
                inputs, outputs, params,
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
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Audio processing is driven by the cpal audio thread
        // via cardinal_audio_process(), not here.

        // ── Side panel ───────────────────────────────────────────────
        egui::SidePanel::left("browser").min_width(180.0).show(ctx, |ui| {
            ui.heading("Module Browser");
            ui.separator();

            let catalog = self.catalog.clone();
            for entry in &catalog {
                if ui.button(&entry.model_name).clicked() {
                    let x = 200.0 + self.modules.len() as f32 * 20.0;
                    let y = 100.0 + self.modules.len() as f32 * 20.0;
                    self.spawn_module(&entry.plugin_slug, &entry.model_slug,
                                     egui::pos2(x, y));
                }
            }

            ui.separator();
            ui.heading("Status");
            ui.label(format!("Modules: {}", self.modules.len()));
            ui.label(format!("Cables: {}", self.cables.len()));
            ui.label(format!("SR: {} Hz", cc::sample_rate()));
        });

        // ── Central panel ────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            // 1) Allocate rects and collect interactions first
            let rect = ui.max_rect();
            let mut responses: Vec<(usize, egui::Response)> = Vec::new();

            for (idx, m) in self.modules.iter().enumerate() {
                let module_rect = egui::Rect::from_min_size(m.pos, m.size);
                if rect.intersects(module_rect) {
                    let resp = ui.allocate_rect(module_rect, egui::Sense::click_and_drag());
                    responses.push((idx, resp));
                }
            }

            // 2) Handle interactions
            let mut drag_completed: Option<(ModuleId, i32, bool, egui::Pos2)> = None;

            for (idx, response) in &responses {
                let module_id = self.modules[*idx].id;

                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        if let Some((mid, pid, is_out)) = self.find_port_at(pos) {
                            if mid == module_id {
                                self.drag_cable = Some(DragCable {
                                    from_module: mid,
                                    from_port: pid,
                                    is_output: is_out,
                                    mouse_pos: pos,
                                });
                            }
                        }
                    }
                }

                if response.dragged() {
                    if let Some(dc) = &mut self.drag_cable {
                        if let Some(pos) = response.interact_pointer_pos() {
                            dc.mouse_pos = pos;
                        }
                    } else {
                        self.modules[*idx].pos += response.drag_delta();
                    }
                }

                if response.drag_stopped() {
                    if let Some(dc) = self.drag_cable.take() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            drag_completed = Some((dc.from_module, dc.from_port, dc.is_output, pos));
                        }
                    }
                }
            }

            // Complete cable
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
                                _id: cid, out_module: om, out_port: op,
                                in_module: im, in_port: ip,
                            });
                        }
                    }
                }
            }

            // 3) Paint everything
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
                        let i = (v.abs() / 5.0).clamp(0.0, 1.0);
                        let color = egui::Color32::from_rgb(
                            (100.0 + 155.0 * i) as u8,
                            (200.0 * (1.0 - i * 0.5)) as u8,
                            (80.0 + 100.0 * i) as u8,
                        );

                        let mid_y = p1.y.max(p2.y) + 30.0 + (p1.x - p2.x).abs() * 0.15;
                        let c1 = egui::pos2(p1.x, mid_y);
                        let c2 = egui::pos2(p2.x, mid_y);
                        let pts: Vec<egui::Pos2> = (0..=20).map(|j| {
                            let t = j as f32 / 20.0;
                            let u = 1.0 - t;
                            egui::pos2(
                                u*u*u*p1.x + 3.0*u*u*t*c1.x + 3.0*u*t*t*c2.x + t*t*t*p2.x,
                                u*u*u*p1.y + 3.0*u*u*t*c1.y + 3.0*u*t*t*c2.y + t*t*t*p2.y,
                            )
                        }).collect();
                        painter.add(egui::Shape::line(pts, egui::Stroke::new(3.0, color)));
                    }
                }
            }

            // Draw drag cable
            if let Some(dc) = &self.drag_cable {
                if let Some(m) = self.modules.iter().find(|m| m.id == dc.from_module) {
                    let ports = if dc.is_output { &m.outputs } else { &m.inputs };
                    if let Some(p) = ports.iter().find(|p| p.id == dc.from_port) {
                        let start = Self::port_world_pos(m, p);
                        painter.line_segment(
                            [start, dc.mouse_pos],
                            egui::Stroke::new(2.0, egui::Color32::YELLOW),
                        );
                    }
                }
            }

            // Draw modules
            for m in &self.modules {
                let mr = egui::Rect::from_min_size(m.pos, m.size);
                painter.rect_filled(mr, 4.0, egui::Color32::from_rgb(50, 52, 58));
                painter.rect_stroke(mr, 4.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 82, 88)),
                    egui::StrokeKind::Outside);

                // Name
                painter.text(m.pos + egui::vec2(m.size.x / 2.0, 12.0),
                    egui::Align2::CENTER_CENTER, &m.name,
                    egui::FontId::proportional(13.0), egui::Color32::WHITE);

                // Input ports
                for port in &m.inputs {
                    let pp = Self::port_world_pos(m, port);
                    let v = cc::module_get_input_voltage(m.id, port.id);
                    let b = (v.abs() / 5.0).clamp(0.0, 1.0);
                    painter.circle_filled(pp, 8.0, egui::Color32::from_rgb(
                        60 + (b * 100.0) as u8, 120 + (b * 80.0) as u8, 200));
                    painter.circle_stroke(pp, 8.0,
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 160, 255)));
                    painter.text(pp + egui::vec2(12.0, 0.0), egui::Align2::LEFT_CENTER,
                        &port.name, egui::FontId::proportional(9.0), egui::Color32::LIGHT_GRAY);
                }

                // Output ports
                for port in &m.outputs {
                    let pp = Self::port_world_pos(m, port);
                    let v = cc::module_get_output_voltage(m.id, port.id);
                    let b = (v.abs() / 5.0).clamp(0.0, 1.0);
                    painter.circle_filled(pp, 8.0, egui::Color32::from_rgb(
                        200, 100 + (b * 100.0) as u8, 60 + (b * 60.0) as u8));
                    painter.circle_stroke(pp, 8.0,
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 160, 80)));
                    painter.text(pp + egui::vec2(-12.0, 0.0), egui::Align2::RIGHT_CENTER,
                        &port.name, egui::FontId::proportional(9.0), egui::Color32::LIGHT_GRAY);
                }

                // Params as knobs
                for param in &m.params {
                    let pp = m.pos + egui::vec2(param.x, param.y);
                    let val = cc::module_get_param(m.id, param.id);
                    let norm = if param.max > param.min {
                        (val - param.min) / (param.max - param.min)
                    } else { 0.5 };

                    painter.circle_filled(pp, 14.0, egui::Color32::from_rgb(40, 42, 48));
                    painter.circle_stroke(pp, 14.0,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 102, 108)));

                    let angle = std::f32::consts::PI * 0.75 + norm * std::f32::consts::PI * 1.5;
                    let tip = pp + egui::vec2(angle.cos() * 10.0, angle.sin() * 10.0);
                    painter.line_segment([pp, tip],
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(220, 220, 230)));

                    painter.text(pp + egui::vec2(0.0, 20.0), egui::Align2::CENTER_CENTER,
                        &param.name, egui::FontId::proportional(8.0), egui::Color32::GRAY);
                }
            }
        });

        ctx.request_repaint();
    }
}
