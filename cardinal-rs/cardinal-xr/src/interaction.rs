// cardinal-xr/src/interaction.rs
//! Per-widget interaction box data structures and input forwarding helpers.

use std::sync::mpsc;

use cardinal_core::cardinal_thread::{Command, EventResult};
use cardinal_core::{ModuleId, ParamInfo, PortInfo};
use glam::Vec3;

use crate::math;

// ── Widget kind ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetKind {
    InputPort { port_id: i32 },
    OutputPort { port_id: i32 },
    Param { param_id: i32 },
}

impl WidgetKind {
    /// Returns true if this widget is a port (input or output).
    pub fn is_port(self) -> bool {
        matches!(self, WidgetKind::InputPort { .. } | WidgetKind::OutputPort { .. })
    }

    /// Returns the port id for ports, or `None` for params.
    pub fn port_id(self) -> Option<i32> {
        match self {
            WidgetKind::InputPort { port_id } | WidgetKind::OutputPort { port_id } => {
                Some(port_id)
            }
            WidgetKind::Param { .. } => None,
        }
    }

    /// Returns true if this is an output port.
    pub fn is_output(self) -> bool {
        matches!(self, WidgetKind::OutputPort { .. })
    }
}

// ── Interaction box ──────────────────────────────────────────────────────────

pub struct InteractionBox {
    pub kind: WidgetKind,
    pub pixel_x: f32,
    pub pixel_y: f32,
    /// 3D offset from the module panel center.
    pub panel_offset: Vec3,
    pub hovered: bool,
    // TODO: Stardust Field + InputHandler + Lines
}

// ── Builder ──────────────────────────────────────────────────────────────────

/// Create `InteractionBox` entries for every input port, output port, and
/// parameter belonging to a module.
pub fn build_interaction_boxes(
    inputs: &[PortInfo],
    outputs: &[PortInfo],
    params: &[ParamInfo],
    module_width_px: f32,
    module_height_px: f32,
) -> Vec<InteractionBox> {
    let mut boxes = Vec::with_capacity(inputs.len() + outputs.len() + params.len());

    for port in inputs {
        boxes.push(InteractionBox {
            kind: WidgetKind::InputPort { port_id: port.id },
            pixel_x: port.x,
            pixel_y: port.y,
            panel_offset: math::pixel_to_panel_offset(
                port.x,
                port.y,
                module_width_px,
                module_height_px,
            ),
            hovered: false,
        });
    }

    for port in outputs {
        boxes.push(InteractionBox {
            kind: WidgetKind::OutputPort { port_id: port.id },
            pixel_x: port.x,
            pixel_y: port.y,
            panel_offset: math::pixel_to_panel_offset(
                port.x,
                port.y,
                module_width_px,
                module_height_px,
            ),
            hovered: false,
        });
    }

    for param in params {
        boxes.push(InteractionBox {
            kind: WidgetKind::Param { param_id: param.id },
            pixel_x: param.x,
            pixel_y: param.y,
            panel_offset: math::pixel_to_panel_offset(
                param.x,
                param.y,
                module_width_px,
                module_height_px,
            ),
            hovered: false,
        });
    }

    boxes
}

// ── Event forwarding ─────────────────────────────────────────────────────────

/// Send a button event to a module widget and wait for the result.
pub fn send_button_event(
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

/// Send a hover event to a module widget (fire-and-forget).
pub fn send_hover_event(cmd_tx: &mpsc::Sender<Command>, module_id: ModuleId, x: f32, y: f32) {
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

/// Send a scroll event to a module widget (fire-and-forget).
pub fn send_scroll_event(
    cmd_tx: &mpsc::Sender<Command>,
    module_id: ModuleId,
    x: f32,
    y: f32,
    scroll_x: f32,
    scroll_y: f32,
) {
    let _ = cmd_tx.send(Command::ModuleEvent {
        module_id,
        event_type: cardinal_core::EVENT_SCROLL,
        x,
        y,
        button: 0,
        action: 0,
        mods: 0,
        scroll_x,
        scroll_y,
        reply: None,
    });
}
