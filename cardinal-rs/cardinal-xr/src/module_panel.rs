use cardinal_core::cardinal_thread::RenderResult;
use cardinal_core::{ModuleId, ParamInfo, PortInfo};
use glam::Vec3;
use stardust_xr_fusion::drawable::Model;
use stardust_xr_fusion::fields::{Field, Shape};
use stardust_xr_fusion::input::{InputDataType, InputHandler};
use stardust_xr_fusion::spatial::{Spatial, SpatialAspect, SpatialRefAspect, Transform};
use stardust_xr_fusion::values::ResourceID;
use stardust_xr_molecules::button::{Button, ButtonSettings, ButtonVisualSettings};
use stardust_xr_molecules::input_action::{InputQueue, InputQueueable, SingleAction};
use stardust_xr_molecules::UIElement;
use stardust_xr_fusion::values::color::rgba_linear;

use crate::constants::{
    DELETE_BUTTON_OFFSET_M, DELETE_BUTTON_SIZE_M, PANEL_DEPTH_M, PIXELS_PER_METER,
    RESIZE_HANDLE_RADIUS_M,
};

/// Path to the panel glTF asset, resolved at compile time relative to the crate root.
fn panel_resource() -> ResourceID {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/panel.glb");
    ResourceID::new_direct(&path)
        .unwrap_or_else(|e| panic!("cardinal-xr: panel.glb not found at {}: {e}", path.display()))
}

/// A small grabbable sphere at a corner of the panel, used for resizing.
struct ResizeHandle {
    _field: Field,
    input: InputQueue,
    grab_action: SingleAction,
    /// Which corner: index 0=top-left, 1=top-right, 2=bottom-left, 3=bottom-right
    corner_index: usize,
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

        Ok(ResizeHandle {
            _field: field,
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

    /// Grabbable body for moving the panel in 3D space.
    body_grab: BodyGrab,
    /// Resize handles at each corner.
    resize_handles: [ResizeHandle; 4],
    /// Delete button at the top-right corner.
    delete_button: Button,
    /// Set to true when the delete button is pressed; workspace should check and remove.
    pub pending_delete: bool,
}

impl ModulePanel {
    pub fn new(
        parent: &impl SpatialRefAspect,
        id: ModuleId,
        size_px: (f32, f32),
        inputs: Vec<PortInfo>,
        outputs: Vec<PortInfo>,
        position: glam::Vec3,
        rotation: glam::Quat,
    ) -> Self {
        let params = cardinal_core::module_params(id);

        let width_m = size_px.0 / PIXELS_PER_METER;
        let height_m = size_px.1 / PIXELS_PER_METER;

        // Create a root spatial at the requested position/rotation, parented to the workspace root.
        let spatial = Spatial::create(
            parent,
            Transform::from_translation_rotation(
                mint::Vector3::from(position),
                mint::Quaternion::from(rotation),
            ),
        )
        .expect("cardinal-xr: failed to create panel spatial");

        // Load the panel model as a child of the root spatial, scaled to the module's
        // world-space dimensions. The model is already PANEL_DEPTH_M deep, so depth stays 1.0.
        let model = Model::create(
            &spatial,
            Transform::from_scale(mint::Vector3 {
                x: width_m,
                y: height_m,
                z: 1.0,
            }),
            &panel_resource(),
        )
        .expect("cardinal-xr: failed to create panel model");

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
            body_grab,
            resize_handles,
            delete_button,
            pending_delete: false,
        }
    }

    pub fn on_render_result(&mut self, _result: RenderResult) {
        // TODO: update dmatex texture
    }

    pub fn width_m(&self) -> f32 {
        self.size_px.0 / PIXELS_PER_METER
    }

    pub fn height_m(&self) -> f32 {
        self.size_px.1 / PIXELS_PER_METER
    }

    /// Called each frame to process grab, resize, and delete interactions.
    pub fn frame_update(&mut self) {
        // --- Body grab (moving) ---
        self.body_grab.handle_events();

        if self.body_grab.actor_started() {
            if let Some(gp) = self.body_grab.grab_point() {
                self.body_grab.grab_offset = self.position - gp;
            }
        }

        if self.body_grab.actor_acting() {
            if let Some(gp) = self.body_grab.grab_point() {
                let new_pos = gp + self.body_grab.grab_offset;
                self.position = new_pos;
                let _ = self.spatial.set_local_transform(
                    Transform::from_translation_rotation(
                        mint::Vector3::from(self.position),
                        mint::Quaternion::from(self.rotation),
                    ),
                );
            }
        }

        // --- Resize handles ---
        for handle in &mut self.resize_handles {
            handle.handle_events();
        }

        // Check if any resize handle is being dragged
        if let Some(active_idx) = self.resize_handles.iter().position(|h| h.is_grabbing()) {
            if let Some(grab_pos) = self.resize_handles[active_idx].grab_point() {
                // Simple resize: compute new size based on distance from center.
                // The grab position is in the parent (spatial) coordinate space.
                // We use the absolute x/y to determine new half-extents.
                let new_hw = grab_pos.x.abs().max(0.01);
                let new_hh = grab_pos.y.abs().max(0.01);

                let new_width_m = new_hw * 2.0;
                let new_height_m = new_hh * 2.0;

                // Update size_px (approximate, for bookkeeping)
                self.size_px = (new_width_m * PIXELS_PER_METER, new_height_m * PIXELS_PER_METER);

                // Rescale model
                let _ = self.model.set_local_transform(Transform::from_scale(mint::Vector3 {
                    x: new_width_m,
                    y: new_height_m,
                    z: 1.0,
                }));
            }
        }

        // --- Delete button ---
        self.delete_button.handle_events();
        if self.delete_button.pressed() {
            self.pending_delete = true;
        }
    }
}
