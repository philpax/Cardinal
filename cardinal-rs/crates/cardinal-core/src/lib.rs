//! cardinal-core: Rust bindings to the VCV Rack engine via Cardinal.
//!
//! Compiles the real VCV Rack C++ engine code and exposes it through
//! a safe Rust API for module creation, cable patching, and audio processing.

mod ffi;

use std::ffi::CString;
use std::sync::Once;

static INIT: Once = Once::new();

/// Initialise the Rack engine.  Safe to call multiple times (only first call takes effect).
pub fn init(sample_rate: f32) {
    INIT.call_once(|| {
        let ret = unsafe { ffi::cardinal_init(sample_rate) };
        assert_eq!(ret, 0, "cardinal_init failed");
    });
}

/// Shut down the engine (best-effort, usually called at process exit).
pub fn shutdown() {
    unsafe { ffi::cardinal_shutdown() }
}

/// A module type available in the catalog.
#[derive(Debug, Clone)]
pub struct CatalogEntry {
    pub plugin_slug: String,
    pub model_slug: String,
    pub model_name: String,
}

/// List all available module types.
pub fn catalog() -> Vec<CatalogEntry> {
    let count = unsafe { ffi::cardinal_catalog_count() } as usize;
    let mut raw = vec![ffi::ModuleCatalogEntry::default(); count];
    let written = unsafe { ffi::cardinal_catalog_list(raw.as_mut_ptr(), count as _) } as usize;
    raw.truncate(written);
    raw.into_iter()
        .map(|e| CatalogEntry {
            plugin_slug: unsafe { c_str_to_string(e.plugin_slug) },
            model_slug: unsafe { c_str_to_string(e.model_slug) },
            model_name: unsafe { c_str_to_string(e.model_name) },
        })
        .collect()
}

unsafe fn c_str_to_string(ptr: *const std::ffi::c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }
}

/// Handle to a live module in the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(pub i64);

/// Handle to a live cable in the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CableId(pub i64);

/// Port metadata.
#[derive(Debug, Clone)]
pub struct PortInfo {
    pub id: i32,
    pub name: String,
    pub x: f32,
    pub y: f32,
}

/// Parameter metadata.
#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub id: i32,
    pub name: String,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub value: f32,
    pub x: f32,
    pub y: f32,
}

/// Create a new module instance. Returns `None` if the slug pair isn't found.
pub fn module_create(plugin_slug: &str, model_slug: &str) -> Option<ModuleId> {
    let ps = CString::new(plugin_slug).ok()?;
    let ms = CString::new(model_slug).ok()?;
    let h = unsafe { ffi::cardinal_module_create(ps.as_ptr(), ms.as_ptr()) };
    if h < 0 { None } else { Some(ModuleId(h)) }
}

/// Remove and destroy a module.
pub fn module_destroy(id: ModuleId) {
    unsafe { ffi::cardinal_module_destroy(id.0) }
}

/// Get a module's pixel dimensions (width, height).
pub fn module_size(id: ModuleId) -> (f32, f32) {
    let (mut w, mut h) = (0f32, 0f32);
    unsafe { ffi::cardinal_module_get_size(id.0, &mut w, &mut h) }
    (w, h)
}

/// Get input port metadata for a module.
pub fn module_inputs(id: ModuleId) -> Vec<PortInfo> {
    read_ports(id, true)
}

/// Get output port metadata for a module.
pub fn module_outputs(id: ModuleId) -> Vec<PortInfo> {
    read_ports(id, false)
}

fn read_ports(id: ModuleId, inputs: bool) -> Vec<PortInfo> {
    let mut raw = vec![ffi::PortInfo::default(); 32];
    let n = if inputs {
        unsafe { ffi::cardinal_module_get_inputs(id.0, raw.as_mut_ptr(), 32) }
    } else {
        unsafe { ffi::cardinal_module_get_outputs(id.0, raw.as_mut_ptr(), 32) }
    } as usize;
    raw.truncate(n);
    raw.into_iter()
        .map(|p| PortInfo {
            id: p.port_id,
            name: unsafe { c_str_to_string(p.name) },
            x: p.x,
            y: p.y,
        })
        .collect()
}

/// Get parameter metadata for a module.
pub fn module_params(id: ModuleId) -> Vec<ParamInfo> {
    let mut raw = vec![ffi::ParamInfo::default(); 32];
    let n =
        unsafe { ffi::cardinal_module_get_params(id.0, raw.as_mut_ptr(), 32) } as usize;
    raw.truncate(n);
    raw.into_iter()
        .map(|p| ParamInfo {
            id: p.param_id,
            name: unsafe { c_str_to_string(p.name) },
            min: p.min_value,
            max: p.max_value,
            default: p.default_value,
            value: p.value,
            x: p.x,
            y: p.y,
        })
        .collect()
}

pub fn module_get_param(id: ModuleId, param_id: i32) -> f32 {
    unsafe { ffi::cardinal_module_get_param(id.0, param_id) }
}

pub fn module_set_param(id: ModuleId, param_id: i32, value: f32) {
    unsafe { ffi::cardinal_module_set_param(id.0, param_id, value) }
}

pub fn module_get_output_voltage(id: ModuleId, port_id: i32) -> f32 {
    unsafe { ffi::cardinal_module_get_output_voltage(id.0, port_id) }
}

pub fn module_get_input_voltage(id: ModuleId, port_id: i32) -> f32 {
    unsafe { ffi::cardinal_module_get_input_voltage(id.0, port_id) }
}

/// Connect an output port to an input port.
pub fn cable_create(
    out_module: ModuleId, out_port: i32,
    in_module: ModuleId, in_port: i32,
) -> Option<CableId> {
    let h = unsafe {
        ffi::cardinal_cable_create(out_module.0, out_port, in_module.0, in_port)
    };
    if h < 0 { None } else { Some(CableId(h)) }
}

/// Remove a cable.
pub fn cable_destroy(id: CableId) {
    unsafe { ffi::cardinal_cable_destroy(id.0) }
}

/// Process N audio frames through the engine.
pub fn process(frames: i32) {
    unsafe { ffi::cardinal_process(frames) }
}

/// Current engine sample rate.
pub fn sample_rate() -> f32 {
    unsafe { ffi::cardinal_get_sample_rate() }
}
