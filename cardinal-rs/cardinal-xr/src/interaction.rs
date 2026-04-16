// cardinal-xr/src/interaction.rs
//! Per-widget interaction boxes with Stardust Fields, InputHandlers,
//! and pinch-based input forwarding to cardinal-core.

use std::sync::mpsc;

use cardinal_core::cardinal_thread::{Command, EventResult};
use cardinal_core::{ModuleId, ParamInfo, PortInfo};
use glam::Vec3;
use stardust_xr_fusion::drawable::Lines;
use stardust_xr_fusion::fields::{Field, Shape};
use stardust_xr_fusion::input::{InputDataType, InputHandler};
use stardust_xr_fusion::spatial::{SpatialRefAspect, Transform};
use stardust_xr_fusion::values::color::rgba_linear;
use stardust_xr_molecules::input_action::{InputQueue, InputQueueable, SingleAction};
use stardust_xr_molecules::lines::{LineExt, circle};
use stardust_xr_molecules::UIElement;

use crate::constants::*;
use crate::math;

// ── Widget kind ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetKind {
    InputPort { port_id: i32 },
    OutputPort { port_id: i32 },
    Param { param_id: i32 },
}

impl WidgetKind {
    pub fn is_port(self) -> bool {
        matches!(self, WidgetKind::InputPort { .. } | WidgetKind::OutputPort { .. })
    }

    pub fn port_id(self) -> Option<i32> {
        match self {
            WidgetKind::InputPort { port_id } | WidgetKind::OutputPort { port_id } => {
                Some(port_id)
            }
            WidgetKind::Param { .. } => None,
        }
    }

    pub fn is_output(self) -> bool {
        matches!(self, WidgetKind::OutputPort { .. })
    }
}

// ── Interaction box ──────────────────────────────────────────────────────────

pub struct InteractionBox {
    pub kind: WidgetKind,
    pub pixel_x: f32,
    pub pixel_y: f32,
    pub panel_offset: Vec3,
    pub hovered: bool,
    /// Whether a pinch is currently active on this box.
    pub pinching: bool,

    // Stardust scene graph nodes
    _field: Field,
    _visual: Lines,
    input: InputQueue,
    action: SingleAction,
}

impl InteractionBox {
    fn create(
        parent: &impl SpatialRefAspect,
        kind: WidgetKind,
        pixel_x: f32,
        pixel_y: f32,
        panel_offset: Vec3,
        scale: f32,
    ) -> Option<Self> {
        let box_size = INTERACTION_BOX_MIN_SIZE_M * scale;
        // Push interaction boxes well in front of the panel body grab field
        let protrusion = (INTERACTION_BOX_PROTRUSION_M + PANEL_DEPTH_M) * scale;

        let pos = Vec3::new(panel_offset.x, panel_offset.y, protrusion);

        let field = Field::create(
            parent,
            Transform::from_translation(mint::Vector3::from(pos)),
            Shape::Box(mint::Vector3 {
                x: box_size,
                y: box_size,
                z: protrusion * 2.0,
            }),
        ).ok()?;

        let input = InputHandler::create(
            parent,
            Transform::from_translation(mint::Vector3::from(pos)),
            &field,
        ).ok()?.queue().ok()?;

        // Visual indicator — small circle outline at the widget position
        let color = if kind.is_port() {
            rgba_linear!(PORT_HOVER_COLOR.x, PORT_HOVER_COLOR.y, PORT_HOVER_COLOR.z, HOVER_HIGHLIGHT_OPACITY_IDLE)
        } else {
            rgba_linear!(PARAM_HOVER_COLOR.x, PARAM_HOVER_COLOR.y, PARAM_HOVER_COLOR.z, HOVER_HIGHLIGHT_OPACITY_IDLE)
        };

        let visual_line = circle(8, 0.0, box_size * 0.5)
            .color(color)
            .thickness(0.001 * scale);

        // circle() generates in XZ plane; rotate 90° around X to bring onto XY (panel face)
        let visual = Lines::create(
            parent,
            Transform::from_translation_rotation(
                mint::Vector3::from(pos),
                mint::Quaternion::from(glam::Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            ),
            &[visual_line],
        ).ok()?;

        Some(InteractionBox {
            kind,
            pixel_x,
            pixel_y,
            panel_offset,
            hovered: false,
            pinching: false,
            _field: field,
            _visual: visual,
            input,
            action: SingleAction::default(),
        })
    }

    /// Process input events and return whether a pinch started or stopped.
    fn update(&mut self) -> InteractionEvent {
        if !self.input.handle_events() {
            return InteractionEvent::None;
        }

        // Debug: log first input to see what data we get
        static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !LOGGED.load(std::sync::atomic::Ordering::Relaxed) {
            for (data, _) in self.input.input() {
                LOGGED.store(true, std::sync::atomic::Ordering::Relaxed);
                let input_type = match &data.input {
                    InputDataType::Hand(_) => "Hand",
                    InputDataType::Pointer(_) => "Pointer",
                    InputDataType::Tip(_) => "Tip",
                };
                let select = data.datamap.with_data(|dm| dm.idx("select").as_f32());
                let grab = data.datamap.with_data(|dm| dm.idx("grab").as_f32());
                eprintln!(
                    "cardinal-xr: interaction input: type={input_type}, dist={:.4}, select={select:.2}, grab={grab:.2}",
                    data.distance,
                );
            }
        }

        let max_dist = INTERACTION_BOX_MIN_SIZE_M + 0.02;
        self.action.update(
            true,
            &self.input,
            |input| input.distance < max_dist,
            |input| {
                input.datamap.with_data(|datamap| match &input.input {
                    InputDataType::Hand(_) => datamap.idx("pinch_strength").as_f32() > 0.90,
                    InputDataType::Pointer(_) => datamap.idx("select").as_f32() > 0.90,
                    _ => datamap.idx("grab").as_f32() > 0.90,
                })
            },
        );

        if self.action.actor_started() {
            self.pinching = true;
            InteractionEvent::PinchStarted
        } else if self.action.actor_stopped() {
            self.pinching = false;
            InteractionEvent::PinchStopped
        } else if self.action.actor_acting() {
            InteractionEvent::PinchHeld
        } else {
            let was_hovered = self.hovered;
            self.hovered = !self.action.hovering().current().is_empty();
            if self.hovered && !was_hovered {
                InteractionEvent::HoverEntered
            } else if !self.hovered && was_hovered {
                InteractionEvent::HoverExited
            } else {
                InteractionEvent::None
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionEvent {
    None,
    HoverEntered,
    HoverExited,
    PinchStarted,
    PinchHeld,
    PinchStopped,
}

// ── Builder ──────────────────────────────────────────────────────────────────

/// Create Stardust interaction boxes for every port and param on a module.
pub fn create_interaction_boxes(
    parent: &impl SpatialRefAspect,
    inputs: &[PortInfo],
    outputs: &[PortInfo],
    params: &[ParamInfo],
    module_width_px: f32,
    module_height_px: f32,
    scale: f32,
) -> Vec<InteractionBox> {
    let mut boxes = Vec::new();

    for port in inputs {
        let offset = math::pixel_to_panel_offset(port.x, port.y, module_width_px, module_height_px) * scale;
        if let Some(b) = InteractionBox::create(
            parent,
            WidgetKind::InputPort { port_id: port.id },
            port.x, port.y, offset, scale,
        ) {
            boxes.push(b);
        }
    }

    for port in outputs {
        let offset = math::pixel_to_panel_offset(port.x, port.y, module_width_px, module_height_px) * scale;
        if let Some(b) = InteractionBox::create(
            parent,
            WidgetKind::OutputPort { port_id: port.id },
            port.x, port.y, offset, scale,
        ) {
            boxes.push(b);
        }
    }

    for param in params {
        let offset = math::pixel_to_panel_offset(param.x, param.y, module_width_px, module_height_px) * scale;
        if let Some(b) = InteractionBox::create(
            parent,
            WidgetKind::Param { param_id: param.id },
            param.x, param.y, offset, scale,
        ) {
            boxes.push(b);
        }
    }

    boxes
}

/// Process interaction events for all boxes and forward to cardinal-core.
pub fn process_interactions(
    boxes: &mut [InteractionBox],
    cmd_tx: &mpsc::Sender<Command>,
    module_id: ModuleId,
) {
    for ibox in boxes.iter_mut() {
        let event = ibox.update();
        match event {
            InteractionEvent::PinchStarted => {
                // Send button press
                let result = send_button_event(cmd_tx, module_id, ibox.pixel_x, ibox.pixel_y, 1, 0);
                if let Some(result) = result {
                    if result.consumed {
                        if let Some(port_drag) = result.port_drag {
                            eprintln!(
                                "cardinal-xr: port drag started on {:?} port {}",
                                module_id, port_drag.port_id
                            );
                            // TODO: initiate cable drag
                        }
                    }
                }
            }
            InteractionEvent::PinchHeld => {
                // Send hover (drag) events
                send_hover_event(cmd_tx, module_id, ibox.pixel_x, ibox.pixel_y);
            }
            InteractionEvent::PinchStopped => {
                // Send button release
                send_button_event(cmd_tx, module_id, ibox.pixel_x, ibox.pixel_y, 0, 0);
            }
            _ => {}
        }
    }
}

// ── Event forwarding ─────────────────────────────────────────────────────────

fn send_button_event(
    cmd_tx: &mpsc::Sender<Command>,
    module_id: ModuleId,
    x: f32,
    y: f32,
    action: i32,
    mods: i32,
) -> Option<EventResult> {
    let (reply_tx, reply_rx) = mpsc::channel();
    cmd_tx
        .send(Command::ModuleEvent {
            module_id,
            event_type: cardinal_core::EVENT_BUTTON,
            x,
            y,
            button: 0,
            action,
            mods,
            scroll_x: 0.0,
            scroll_y: 0.0,
            reply: Some(reply_tx),
        })
        .ok()?;
    reply_rx.recv().ok()
}

fn send_hover_event(cmd_tx: &mpsc::Sender<Command>, module_id: ModuleId, x: f32, y: f32) {
    let _ = cmd_tx.send(Command::ModuleEvent {
        module_id,
        event_type: cardinal_core::EVENT_HOVER,
        x,
        y,
        button: 0,
        action: 0,
        mods: 0,
        scroll_x: 0.0,
        scroll_y: 0.0,
        reply: None,
    });
}
