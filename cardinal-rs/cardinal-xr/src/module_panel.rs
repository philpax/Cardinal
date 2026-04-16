use cardinal_core::cardinal_thread::RenderResult;
use cardinal_core::{ModuleId, ParamInfo, PortInfo};
use glam::Vec3;
use stardust_xr_fusion::drawable::{Lines, Model, ModelPartAspect};
use stardust_xr_fusion::fields::{Field, Shape};
use stardust_xr_fusion::input::{InputDataType, InputHandler};
use stardust_xr_fusion::spatial::{Spatial, SpatialAspect, SpatialRefAspect, Transform};
use stardust_xr_fusion::values::ResourceID;
use stardust_xr_fusion::values::color::rgba_linear;
use stardust_xr_molecules::button::{Button, ButtonSettings, ButtonVisualSettings};
use stardust_xr_molecules::input_action::{InputQueue, InputQueueable, SingleAction};
use stardust_xr_molecules::lines::{LineExt, circle};
use stardust_xr_molecules::UIElement;

use crate::dmatex;

use crate::constants::{
    DELETE_BUTTON_OFFSET_M, DELETE_BUTTON_SIZE_M, GRAB_MOMENTUM_DRAG, GRAB_MOMENTUM_THRESHOLD,
    PANEL_DEPTH_M, PIXELS_PER_METER, RESIZE_HANDLE_RADIUS_M,
};

/// Path to the panel glTF asset, resolved at compile time relative to the crate root.
fn panel_resource() -> ResourceID {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/panel.glb");
    let path = path.canonicalize()
        .unwrap_or_else(|e| panic!("cardinal-xr: panel.glb not found at {}: {e}", path.display()));
    ResourceID::new_direct(&path)
        .unwrap_or_else(|e| panic!("cardinal-xr: failed to create ResourceID for {}: {e}", path.display()))
}

/// A small grabbable sphere at a corner of the panel, used for resizing.
struct ResizeHandle {
    _field: Field,
    _lines: Lines,
    input: InputQueue,
    grab_action: SingleAction,
    /// Which corner: index 0=top-left, 1=top-right, 2=bottom-left, 3=bottom-right
    corner_index: usize,
}

/// Build sphere-outline lines for a resize handle (3 orthogonal circles).
fn resize_handle_lines() -> Vec<stardust_xr_fusion::drawable::Line> {
    use glam::Mat4;
    use std::f32::consts::FRAC_PI_2;

    let color = rgba_linear!(0.7, 0.7, 0.7, 0.3);
    let thickness = 0.001;
    let r = RESIZE_HANDLE_RADIUS_M;

    let y_circle = circle(12, 0.0, r).color(color).thickness(thickness);
    let x_circle = y_circle
        .clone()
        .transform(Mat4::from_rotation_x(FRAC_PI_2));
    let z_circle = y_circle
        .clone()
        .transform(Mat4::from_rotation_z(FRAC_PI_2));

    vec![y_circle, x_circle, z_circle]
}

impl ResizeHandle {
    fn create(
        parent: &impl SpatialRefAspect,
        offset: Vec3,
        corner_index: usize,
    ) -> Result<Self, stardust_xr_fusion::node::NodeError> {
        let field = Field::create(
            parent,
            Transform::from_translation(mint::Vector3::from(offset)),
            Shape::Sphere(RESIZE_HANDLE_RADIUS_M),
        )?;
        let input = InputHandler::create(parent, Transform::none(), &field)?.queue()?;

        let handle_lines = resize_handle_lines();
        let lines = Lines::create(
            parent,
            Transform::from_translation(mint::Vector3::from(offset)),
            &handle_lines,
        )?;

        Ok(ResizeHandle {
            _field: field,
            _lines: lines,
            input,
            grab_action: SingleAction::default(),
            corner_index,
        })
    }

    fn handle_events(&mut self) {
        if !self.input.handle_events() {
            return;
        }
        let max_dist = RESIZE_HANDLE_RADIUS_M + 0.02; // radius + padding
        self.grab_action.update(
            true,
            &self.input,
            |input| match &input.input {
                InputDataType::Pointer(_) => false,
                _ => input.distance < max_dist,
            },
            |input| {
                input.datamap.with_data(|datamap| match &input.input {
                    InputDataType::Hand(_) => datamap.idx("pinch_strength").as_f32() > 0.90,
                    _ => datamap.idx("grab").as_f32() > 0.90,
                })
            },
        );
    }

    /// Returns the current grab point in parent-relative coordinates if actively grabbed.
    fn grab_point(&self) -> Option<Vec3> {
        let actor = self.grab_action.actor()?;
        match &actor.input {
            InputDataType::Pointer(_) => None,
            InputDataType::Hand(h) => {
                Some(Vec3::from(h.thumb.tip.position).lerp(Vec3::from(h.index.tip.position), 0.5))
            }
            InputDataType::Tip(t) => Some(t.origin.into()),
        }
    }

    fn is_grabbing(&self) -> bool {
        self.grab_action.actor_acting()
    }
}

/// Simple body grab state using InputQueue + SingleAction, similar to flatland's grab_ball.
struct BodyGrab {
    _field: Field,
    input: InputQueue,
    grab_action: SingleAction,
    /// The offset between the grab point and the panel position when grab started.
    grab_offset: Vec3,
    /// Previous grab position for velocity tracking.
    prev_grab_pos: Vec3,
    /// Current velocity (meters/sec) for momentum after release.
    velocity: Vec3,
}

impl BodyGrab {
    fn create(
        parent: &impl SpatialRefAspect,
        width_m: f32,
        height_m: f32,
    ) -> Result<Self, stardust_xr_fusion::node::NodeError> {
        // Create a box field covering the panel face, slightly protruding forward
        let field = Field::create(
            parent,
            Transform::identity(),
            Shape::Box(mint::Vector3 {
                x: width_m,
                y: height_m,
                z: PANEL_DEPTH_M + 0.01,
            }),
        )?;
        let input = InputHandler::create(parent, Transform::none(), &field)?.queue()?;

        Ok(BodyGrab {
            _field: field,
            input,
            grab_action: SingleAction::default(),
            grab_offset: Vec3::ZERO,
            prev_grab_pos: Vec3::ZERO,
            velocity: Vec3::ZERO,
        })
    }

    fn handle_events(&mut self) {
        if !self.input.handle_events() {
            return;
        }
        let max_distance = 0.05;
        self.grab_action.update(
            true,
            &self.input,
            |input| match &input.input {
                InputDataType::Hand(h) => {
                    h.thumb.tip.distance < max_distance && h.index.tip.distance < max_distance
                }
                _ => input.distance < max_distance,
            },
            stardust_xr_molecules::input_action::grab_pinch_interact,
        );
    }

    fn grab_point(&self) -> Option<Vec3> {
        let actor = self.grab_action.actor()?;
        match &actor.input {
            InputDataType::Pointer(p) => Some(p.origin.into()),
            InputDataType::Hand(h) => Some(h.palm.position.into()),
            InputDataType::Tip(t) => Some(t.origin.into()),
        }
    }

    fn actor_started(&self) -> bool {
        self.grab_action.actor_started()
    }

    fn actor_acting(&self) -> bool {
        self.grab_action.actor_acting()
    }

    fn actor_stopped(&self) -> bool {
        self.grab_action.actor_stopped()
    }
}

pub struct ModulePanel {
    pub id: ModuleId,
    pub size_px: (f32, f32),
    pub inputs: Vec<PortInfo>,
    pub outputs: Vec<PortInfo>,
    pub params: Vec<ParamInfo>,
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    /// Root spatial node for the module panel, parented to the workspace root.
    pub spatial: Spatial,
    /// The 3D model representing the panel surface.
    pub model: Model,
    /// Debug outline showing panel bounds.
    _outline: Lines,

    /// Grabbable body for moving the panel in 3D space.
    body_grab: BodyGrab,
    /// Resize handles at each corner.
    resize_handles: [ResizeHandle; 4],
    /// Per-widget interaction boxes (ports + params).
    pub interaction_boxes: Vec<crate::interaction::InteractionBox>,
    /// Delete button at the top-right corner.
    delete_button: Button,
    /// Set to true when the delete button is pressed; workspace should check and remove.
    pub pending_delete: bool,
    /// Whether we've applied a texture at least once.
    texture_applied: bool,

    /// DMA-BUF texture state for streaming module renders to Stardust.
    /// None if DMA-BUF export is unavailable (fallback path).
    pub dmatex_state: Option<DmatexState>,
}

/// Tracks the DMA-BUF texture streaming state for one module.
pub struct DmatexState {
    /// The two exportable textures for double-buffering.
    pub textures: [dmatex::ExportableTexture; 2],
    /// Stardust dmatex IDs for the two double-buffered textures.
    pub dmatex_ids: [u64; 2],
    /// Which buffer was last submitted (0 or 1).
    pub current_buffer: usize,
    /// Current acquire timeline point.
    pub acquire_point: u64,
    /// The timeline syncobj for GPU synchronization.
    pub syncobj: timeline_syncobj::timeline_syncobj::TimelineSyncObj,
}

impl ModulePanel {
    pub fn new(
        parent: &impl SpatialRefAspect,
        id: ModuleId,
        size_px: (f32, f32),
        inputs: Vec<PortInfo>,
        outputs: Vec<PortInfo>,
        params: Vec<ParamInfo>,
        position: glam::Vec3,
        rotation: glam::Quat,
        scale: f32,
    ) -> Self {

        let width_m = size_px.0 / PIXELS_PER_METER * scale;
        let height_m = size_px.1 / PIXELS_PER_METER * scale;

        // Create a root spatial at the requested position/rotation, parented to the workspace root.
        let spatial = Spatial::create(
            parent,
            Transform::from_translation_rotation(
                mint::Vector3::from(position),
                mint::Quaternion::from(rotation),
            ),
        )
        .expect("cardinal-xr: failed to create panel spatial");

        // Load the panel model as a child of the root spatial.
        // The flatland panel.glb is a unit cube (1×1×1 after its internal 0.5 scale).
        // Scale X=width, Y=height, Z=depth to get a thin slab.
        let model = Model::create(
            &spatial,
            Transform::from_scale(mint::Vector3 {
                x: width_m,
                y: height_m,
                z: PANEL_DEPTH_M,
            }),
            &panel_resource(),
        )
        .expect("cardinal-xr: failed to create panel model");

        // Debug outline: bright green rectangle showing panel bounds
        let hw = width_m / 2.0;
        let hh = height_m / 2.0;
        let outline_color = rgba_linear!(0.0, 1.0, 0.0, 1.0);
        let outline_thickness = 0.002;
        let outline_points: Vec<stardust_xr_fusion::drawable::LinePoint> = [
            [-hw, -hh, 0.0],
            [ hw, -hh, 0.0],
            [ hw,  hh, 0.0],
            [-hw,  hh, 0.0],
            [-hw, -hh, 0.0],
        ].iter().map(|p| stardust_xr_fusion::drawable::LinePoint {
            point: mint::Vector3 { x: p[0], y: p[1], z: p[2] },
            thickness: outline_thickness,
            color: outline_color,
        }).collect();
        let outline_line = stardust_xr_fusion::drawable::Line { points: outline_points, cyclic: false };
        let _outline = Lines::create(&spatial, Transform::identity(), &[outline_line])
            .expect("cardinal-xr: failed to create panel outline");


        // --- Body grab ---
        let body_grab = BodyGrab::create(&spatial, width_m, height_m)
            .expect("cardinal-xr: failed to create body grab");


        // --- Resize handles at four corners ---
        let hw = width_m / 2.0;
        let hh = height_m / 2.0;
        let corner_offsets = [
            Vec3::new(-hw, hh, 0.0),  // top-left
            Vec3::new(hw, hh, 0.0),   // top-right
            Vec3::new(-hw, -hh, 0.0), // bottom-left
            Vec3::new(hw, -hh, 0.0),  // bottom-right
        ];
        let resize_handles = std::array::from_fn(|i| {
            ResizeHandle::create(&spatial, corner_offsets[i], i)
                .expect("cardinal-xr: failed to create resize handle")
        });


        // --- Interaction boxes (per-widget) ---
        let interaction_boxes = crate::interaction::create_interaction_boxes(
            &spatial, &inputs, &outputs, &params,
            size_px.0, size_px.1, scale,
        );
        eprintln!(
            "cardinal-xr: created {} interaction boxes for {:?}",
            interaction_boxes.len(), id
        );

        // --- Delete button at top-right corner ---
        let delete_button_x = hw + DELETE_BUTTON_OFFSET_M;
        let delete_button_y = hh + DELETE_BUTTON_OFFSET_M;
        let delete_button = Button::create(
            &spatial,
            Transform::from_translation(mint::Vector3 {
                x: delete_button_x,
                y: delete_button_y,
                z: 0.0,
            }),
            [DELETE_BUTTON_SIZE_M, DELETE_BUTTON_SIZE_M],
            ButtonSettings {
                max_hover_distance: 0.025,
                visuals: Some(ButtonVisualSettings {
                    line_thickness: 0.002,
                    accent_color: rgba_linear!(1.0, 0.2, 0.2, 1.0),
                }),
            },
        )
        .expect("cardinal-xr: failed to create delete button");

        Self {
            id,
            size_px,
            inputs,
            outputs,
            params,
            position,
            rotation,
            spatial,
            model,
            _outline,
            body_grab,
            resize_handles,
            interaction_boxes,
            delete_button,
            pending_delete: false,
            texture_applied: false,
            dmatex_state: None,
        }
    }

    /// Called when a new render texture arrives from the cardinal thread.
    /// Writes the rendered pixels to a PNG and applies as a file-based texture.
    /// (DMA-BUF path is available but Stardust's timeline syncobj has issues;
    /// this fallback uses CPU readback + file texture.)
    pub fn on_render_result(&mut self, result: RenderResult) {
        // Only apply texture once for now (file-based path is slow).
        // TODO: switch to dmatex for real-time streaming.
        if self.texture_applied {
            return;
        }

        let w = result.width;
        let h = result.height;

        let Some(pixels) = result.pixels else {
            return;
        };

        let path = format!("/tmp/cardinal-xr-module-{}.png", self.id.0);
        if let Some(img) = image::RgbaImage::from_raw(w, h, pixels) {
            match img.save(&path) {
                Ok(()) => {
                    eprintln!("cardinal-xr: saved module texture to {path} ({w}x{h})");
                    if let Ok(part) = self.model.part("Panel") {
                        use stardust_xr_fusion::drawable::MaterialParameter;
                        use stardust_xr_fusion::values::ResourceID;
                        if let Ok(resource) = ResourceID::new_direct(&path) {
                            let _ = part.set_material_parameter(
                                "diffuse",
                                MaterialParameter::Texture(resource),
                            );
                            self.texture_applied = true;
                            eprintln!("cardinal-xr: applied texture to panel for {:?}", self.id);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("cardinal-xr: failed to save PNG: {e}");
                }
            }
        }
    }

    pub fn width_m(&self) -> f32 {
        self.size_px.0 / PIXELS_PER_METER
    }

    pub fn height_m(&self) -> f32 {
        self.size_px.1 / PIXELS_PER_METER
    }

    /// Called each frame to process grab, resize, and delete interactions.
    pub fn frame_update(&mut self, dt: f32, cmd_tx: &std::sync::mpsc::Sender<cardinal_core::cardinal_thread::Command>) {
        // --- Body grab (moving) ---
        self.body_grab.handle_events();

        if self.body_grab.actor_started() {
            if let Some(gp) = self.body_grab.grab_point() {
                self.body_grab.grab_offset = self.position - gp;
                self.body_grab.prev_grab_pos = gp;
                self.body_grab.velocity = Vec3::ZERO;
            }
        }

        if self.body_grab.actor_acting() {
            if let Some(gp) = self.body_grab.grab_point() {
                let new_pos = gp + self.body_grab.grab_offset;

                // Track velocity for momentum on release
                if dt > 0.0 {
                    self.body_grab.velocity = (gp - self.body_grab.prev_grab_pos) / dt;
                }
                self.body_grab.prev_grab_pos = gp;

                self.position = new_pos;
                let _ = self.spatial.set_local_transform(
                    Transform::from_translation_rotation(
                        mint::Vector3::from(self.position),
                        mint::Quaternion::from(self.rotation),
                    ),
                );
            }
        }

        if self.body_grab.actor_stopped() {
            // Velocity was already tracked during grab; momentum will be applied below.
        }

        // --- Momentum (post-release drift) ---
        if !self.body_grab.actor_acting() && self.body_grab.velocity.length() > GRAB_MOMENTUM_THRESHOLD {
            // Exponential decay
            self.body_grab.velocity *= (-GRAB_MOMENTUM_DRAG * dt).exp();
            self.position += self.body_grab.velocity * dt;
            let _ = self.spatial.set_local_transform(
                Transform::from_translation_rotation(
                    mint::Vector3::from(self.position),
                    mint::Quaternion::from(self.rotation),
                ),
            );
            // Stop when below threshold
            if self.body_grab.velocity.length() < GRAB_MOMENTUM_THRESHOLD {
                self.body_grab.velocity = Vec3::ZERO;
            }
        }

        // --- Resize handles ---
        for handle in &mut self.resize_handles {
            handle.handle_events();
        }

        // Check if any resize handle is being dragged
        if let Some(active_idx) = self.resize_handles.iter().position(|h| h.is_grabbing()) {
            if let Some(grab_pos) = self.resize_handles[active_idx].grab_point() {
                let corner_idx = self.resize_handles[active_idx].corner_index;
                let old_w = self.width_m();
                let old_h = self.height_m();
                let aspect = old_w / old_h;

                // Determine the anchor corner (opposite to the dragged corner).
                // Corner layout: 0=TL, 1=TR, 2=BL, 3=BR
                let anchor_signs: (f32, f32) = match corner_idx {
                    0 => (1.0, -1.0),  // TL dragged -> anchor BR
                    1 => (-1.0, -1.0), // TR dragged -> anchor BL
                    2 => (1.0, 1.0),   // BL dragged -> anchor TR
                    3 => (-1.0, 1.0),  // BR dragged -> anchor TL
                    _ => unreachable!(),
                };
                let anchor_world = self.position
                    + Vec3::new(anchor_signs.0 * old_w / 2.0, anchor_signs.1 * old_h / 2.0, 0.0);

                // The grab position is in parent-space. Compute desired size from
                // anchor to grab point, then lock aspect ratio using dominant axis.
                let delta = grab_pos - anchor_world;
                let desired_w = delta.x.abs();
                let desired_h = delta.y.abs();

                // Pick dominant axis and derive the other from aspect ratio.
                let (new_w, new_h) = if desired_w / aspect > desired_h {
                    // Width is dominant
                    (desired_w.max(0.02), (desired_w / aspect).max(0.02))
                } else {
                    // Height is dominant
                    ((desired_h * aspect).max(0.02), desired_h.max(0.02))
                };

                // Reposition so the anchor corner stays fixed.
                let new_center = anchor_world
                    + Vec3::new(-anchor_signs.0 * new_w / 2.0, -anchor_signs.1 * new_h / 2.0, 0.0);
                self.position = new_center;

                self.size_px = (new_w * PIXELS_PER_METER, new_h * PIXELS_PER_METER);

                // Rescale model
                let _ = self.model.set_local_transform(Transform::from_scale(mint::Vector3 {
                    x: new_w,
                    y: new_h,
                    z: 1.0,
                }));

                // Update panel position
                let _ = self.spatial.set_local_transform(
                    Transform::from_translation_rotation(
                        mint::Vector3::from(self.position),
                        mint::Quaternion::from(self.rotation),
                    ),
                );
            }
        }

        // --- Interaction boxes ---
        crate::interaction::process_interactions(
            &mut self.interaction_boxes, cmd_tx, self.id,
        );

        // --- Delete button ---
        self.delete_button.handle_events();
        if self.delete_button.pressed() {
            self.pending_delete = true;
        }
    }
}
