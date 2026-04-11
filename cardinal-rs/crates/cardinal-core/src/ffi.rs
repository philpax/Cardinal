//! Raw FFI bindings to the C bridge API.

use std::ffi::c_char;

#[repr(C)]
#[derive(Clone)]
pub struct PortInfo {
    pub port_id: i32,
    pub name: *const c_char,
    pub x: f32,
    pub y: f32,
}

impl Default for PortInfo {
    fn default() -> Self {
        Self { port_id: 0, name: std::ptr::null(), x: 0.0, y: 0.0 }
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct ParamInfo {
    pub param_id: i32,
    pub name: *const c_char,
    pub min_value: f32,
    pub max_value: f32,
    pub default_value: f32,
    pub value: f32,
    pub x: f32,
    pub y: f32,
}

impl Default for ParamInfo {
    fn default() -> Self {
        Self {
            param_id: 0, name: std::ptr::null(),
            min_value: 0.0, max_value: 1.0, default_value: 0.0, value: 0.0,
            x: 0.0, y: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct ModuleCatalogEntry {
    pub plugin_slug: *const c_char,
    pub model_slug: *const c_char,
    pub model_name: *const c_char,
}

impl Default for ModuleCatalogEntry {
    fn default() -> Self {
        Self {
            plugin_slug: std::ptr::null(),
            model_slug: std::ptr::null(),
            model_name: std::ptr::null(),
        }
    }
}

unsafe extern "C" {
    pub fn cardinal_init(sample_rate: f32, resource_dir: *const c_char) -> i32;
    pub fn cardinal_shutdown();

    pub fn cardinal_catalog_count() -> i32;
    pub fn cardinal_catalog_list(out: *mut ModuleCatalogEntry, max_entries: i32) -> i32;

    pub fn cardinal_module_create(plugin_slug: *const c_char, model_slug: *const c_char) -> i64;
    pub fn cardinal_module_destroy(h: i64);
    pub fn cardinal_module_get_size(h: i64, width: *mut f32, height: *mut f32);
    pub fn cardinal_module_get_inputs(h: i64, out: *mut PortInfo, max: i32) -> i32;
    pub fn cardinal_module_get_outputs(h: i64, out: *mut PortInfo, max: i32) -> i32;
    pub fn cardinal_module_get_params(h: i64, out: *mut ParamInfo, max: i32) -> i32;
    pub fn cardinal_module_get_param(h: i64, param_id: i32) -> f32;
    pub fn cardinal_module_set_param(h: i64, param_id: i32, value: f32);
    pub fn cardinal_module_get_input_voltage(h: i64, port_id: i32) -> f32;
    pub fn cardinal_module_get_output_voltage(h: i64, port_id: i32) -> f32;

    pub fn cardinal_cable_create(out_module: i64, out_port: i32, in_module: i64, in_port: i32) -> i64;
    pub fn cardinal_cable_destroy(h: i64);

    pub fn cardinal_module_render(
        h: i64,
        pixels: *mut u8, max_width: i32, max_height: i32,
        out_width: *mut i32, out_height: *mut i32,
    ) -> i32;

    pub fn cardinal_process(frames: i32);
    pub fn cardinal_get_sample_rate() -> f32;
}
