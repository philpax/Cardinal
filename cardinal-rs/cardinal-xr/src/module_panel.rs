use cardinal_core::cardinal_thread::RenderResult;
use cardinal_core::{ModuleId, ParamInfo, PortInfo};
use stardust_xr_fusion::drawable::Model;
use stardust_xr_fusion::spatial::{Spatial, SpatialRefAspect, Transform};
use stardust_xr_fusion::values::ResourceID;

/// Path to the panel glTF asset, resolved at compile time relative to the crate root.
fn panel_resource() -> ResourceID {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/panel.glb");
    ResourceID::new_direct(&path)
        .unwrap_or_else(|e| panic!("cardinal-xr: panel.glb not found at {}: {e}", path.display()))
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

        let width_m = size_px.0 / crate::constants::PIXELS_PER_METER;
        let height_m = size_px.1 / crate::constants::PIXELS_PER_METER;

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
        }
    }

    pub fn on_render_result(&mut self, _result: RenderResult) {
        // TODO: update dmatex texture
    }

    pub fn width_m(&self) -> f32 {
        self.size_px.0 / crate::constants::PIXELS_PER_METER
    }

    pub fn height_m(&self) -> f32 {
        self.size_px.1 / crate::constants::PIXELS_PER_METER
    }
}
