use glam::{Vec3, Vec4};
use cardinal_core::{CableId, ModuleId};
use crate::constants::*;
use crate::math;

pub enum CableDragState {
    Idle,
    Dragging {
        from_module: ModuleId,
        from_port: i32,
        is_output: bool,
        hand_pos: Vec3,
    },
}

pub struct Cable {
    pub id: CableId,
    pub out_module: ModuleId,
    pub out_port: i32,
    pub in_module: ModuleId,
    pub in_port: i32,
    pub color_idx: usize,
}

impl Cable {
    pub fn color(&self) -> Vec4 {
        CABLE_COLORS[self.color_idx % CABLE_COLORS.len()]
    }

    pub fn compute_points(&self, out_pos: Vec3, in_pos: Vec3) -> Vec<Vec3> {
        math::cable_bezier_points(out_pos, in_pos)
    }

    pub fn new(
        id: CableId,
        out_module: ModuleId,
        out_port: i32,
        in_module: ModuleId,
        in_port: i32,
        color_idx: usize,
    ) -> Self {
        Self {
            id,
            out_module,
            out_port,
            in_module,
            in_port,
            color_idx,
        }
    }
}

pub fn preview_cable_points(port_pos: Vec3, hand_pos: Vec3) -> Vec<Vec3> {
    math::cable_bezier_points(port_pos, hand_pos)
}
