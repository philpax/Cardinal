// cardinal-xr/src/constants.rs
//! Single source of truth for all tunable values.

use glam::Vec4;

// ── Panel ──────────────────────────────────────────────────────────
pub const PIXELS_PER_METER: f32 = 3000.0;
pub const PANEL_DEPTH_M: f32 = 0.008;

// ── Interaction ────────────────────────────────────────────────────
pub const INTERACTION_BOX_MIN_SIZE_M: f32 = 0.015;
pub const INTERACTION_BOX_PROTRUSION_M: f32 = 0.005;
pub const INTERACTION_LAYER_OFFSET_M: f32 = 0.001;
pub const PORT_HOVER_COLOR: Vec4 = Vec4::new(0.3, 0.5, 1.0, 1.0);
pub const PARAM_HOVER_COLOR: Vec4 = Vec4::new(1.0, 0.6, 0.2, 1.0);
pub const HOVER_HIGHLIGHT_OPACITY_IDLE: f32 = 0.1;
pub const HOVER_HIGHLIGHT_OPACITY_ACTIVE: f32 = 0.6;

// ── Resize ─────────────────────────────────────────────────────────
pub const RESIZE_HANDLE_RADIUS_M: f32 = 0.01;

// ── Cables ─────────────────────────────────────────────────────────
pub const CABLE_THICKNESS_M: f32 = 0.003;
pub const CABLE_SEGMENT_COUNT: usize = 20;
pub const CABLE_SAG_FACTOR: f32 = 0.05;
pub const CABLE_COLORS: &[Vec4] = &[
    Vec4::new(1.0, 0.2, 0.2, 1.0), // red
    Vec4::new(0.2, 0.5, 1.0, 1.0), // blue
    Vec4::new(0.2, 0.9, 0.3, 1.0), // green
    Vec4::new(1.0, 0.9, 0.2, 1.0), // yellow
    Vec4::new(0.7, 0.3, 1.0, 1.0), // purple
    Vec4::new(1.0, 0.5, 0.0, 1.0), // orange
];

// ── Hand Menu ──────────────────────────────────────────────────────
pub const MENU_PALM_UP_THRESHOLD: f32 = 0.7;
pub const MENU_PALM_DOWN_THRESHOLD: f32 = 0.5;
pub const MENU_PALM_OFFSET_M: f32 = 0.05;
pub const MENU_POSITION_SMOOTHING: f32 = 0.3;
pub const MENU_HOVER_EXPAND_DELAY_SECS: f32 = 0.3;
pub const MENU_MAX_VISIBLE_ITEMS: usize = 10;
pub const MENU_ITEM_HEIGHT_M: f32 = 0.025;
pub const MENU_ITEM_WIDTH_M: f32 = 0.08;
pub const MENU_COLUMN_GAP_M: f32 = 0.01;

// ── Module Spawning ────────────────────────────────────────────────
pub const MODULE_SPAWN_DISTANCE_M: f32 = 0.5;

// ── Delete Button ──────────────────────────────────────────────────
pub const DELETE_BUTTON_SIZE_M: f32 = 0.015;
pub const DELETE_BUTTON_OFFSET_M: f32 = 0.01;
