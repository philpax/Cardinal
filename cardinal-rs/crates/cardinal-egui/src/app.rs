use cardinal_core::{self as cc, CableId, ModuleId};
use std::sync::mpsc;

use crate::cardinal_thread::{Command, EventResult, RenderResult};

pub struct PlacedModule {
    pub id: ModuleId,
    pub name: String,
    pub pos: egui::Pos2,
    pub size: egui::Vec2,
    pub inputs: Vec<cc::PortInfo>,
    pub outputs: Vec<cc::PortInfo>,
    pub texture_id: Option<egui::TextureId>,
    pub render_texture: Option<wgpu::Texture>,
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

pub struct App {
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
    pub fn new(cmd_tx: mpsc::Sender<Command>, render_rx: mpsc::Receiver<RenderResult>) -> Self {
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

    pub fn request_renders(&self) {
        for m in &self.modules {
            let _ = self.cmd_tx.send(Command::RenderModule {
                module_id: m.id,
                width: m.size.x as i32,
                height: m.size.y as i32,
            });
        }
    }

    pub fn poll_render_results(&mut self, renderer: &mut egui_wgpu::Renderer, device: &wgpu::Device) {
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

    pub fn ui(&mut self, ctx: &egui::Context) {
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
