use cardinal_core::{CableId, ModuleId};

pub struct Cable {
    pub id: CableId,
    pub out_module: ModuleId,
    pub out_port: i32,
    pub in_module: ModuleId,
    pub in_port: i32,
    pub color_idx: usize,
}

impl Cable {
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
