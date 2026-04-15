use cardinal_core::cardinal_thread::RenderResult;
use cardinal_core::{ModuleId, PortInfo, ParamInfo};

pub struct ModulePanel {
    pub id: ModuleId,
    pub size_px: (f32, f32),
    pub inputs: Vec<PortInfo>,
    pub outputs: Vec<PortInfo>,
    pub params: Vec<ParamInfo>,
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
}

impl ModulePanel {
    pub fn new(
        id: ModuleId,
        size_px: (f32, f32),
        inputs: Vec<PortInfo>,
        outputs: Vec<PortInfo>,
        position: glam::Vec3,
        rotation: glam::Quat,
    ) -> Self {
        let params = cardinal_core::module_params(id);
        Self {
            id,
            size_px,
            inputs,
            outputs,
            params,
            position,
            rotation,
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
