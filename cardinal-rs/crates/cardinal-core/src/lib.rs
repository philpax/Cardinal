//! cardinal-core: Rust bindings to the VCV Rack engine via Cardinal.
//!
//! Compiles the real VCV Rack C++ engine code and exposes it through
//! a safe Rust API for module creation, cable patching, and audio processing.

// Ensure plugin crates are linked (they provide the compiled plugin objects)
extern crate cardinal_plugins_registry;

mod ffi;

use std::ffi::CString;
use std::sync::Once;

static INIT: Once = Once::new();

/// Initialise the Rack engine.  Safe to call multiple times (only first call takes effect).
/// `resource_dir` should point to the Cardinal repository root.
pub fn init(sample_rate: f32, resource_dir: &str) {
    INIT.call_once(|| {
        let c_dir = CString::new(resource_dir).expect("invalid resource_dir");
        eprintln!("cardinal-rs: [init] calling cardinal_init...");
        let ret = unsafe { ffi::cardinal_init(sample_rate, c_dir.as_ptr()) };
        assert_eq!(ret, 0, "cardinal_init failed");

        // Register all plugin vendors (calls each vendor's C registration function,
        // which creates the Plugin, loads the manifest, and calls init__Vendor)
        eprintln!("cardinal-rs: [init] registering plugins...");
        cardinal_plugins_registry::register_all_plugins();
        eprintln!("cardinal-rs: [init] all plugins registered");
    });
}

/// Returns the Cardinal repository root, inferred from this crate's build location.
pub fn default_resource_dir() -> String {
    // CARGO_MANIFEST_DIR at build time points into cardinal-rs/crates/cardinal-core
    // which is 3 levels below the Cardinal root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let p = std::path::PathBuf::from(manifest_dir);
    p.parent().unwrap().parent().unwrap().parent().unwrap()
        .to_str().unwrap().to_string()
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

/// Render a module widget to RGBA pixels.
/// Returns `Some((width, height, rgba_data))` on success.
pub fn module_render(id: ModuleId, max_width: i32, max_height: i32) -> Option<(i32, i32, Vec<u8>)> {
    let buf_size = (max_width * max_height * 4) as usize;
    let mut pixels = vec![0u8; buf_size];
    let mut w = 0i32;
    let mut h = 0i32;
    let ok = unsafe {
        ffi::cardinal_module_render(id.0, pixels.as_mut_ptr(), max_width, max_height, &mut w, &mut h)
    };
    if ok != 0 && w > 0 && h > 0 {
        pixels.truncate((w * h * 4) as usize);
        Some((w, h, pixels))
    } else {
        None
    }
}

// ── Render context (for dedicated render thread) ────────────────────

/// Make the offscreen EGL/NanoVG context current on the calling thread.
/// Call once from a render thread before calling `module_render`.
/// Returns true on success.
pub fn render_claim_context() -> bool {
    unsafe { ffi::cardinal_render_claim_context() != 0 }
}

/// Release the offscreen EGL/NanoVG context from the calling thread.
pub fn render_release_context() {
    unsafe { ffi::cardinal_render_release_context() }
}

// ── Audio I/O ────────────────────────────────────────────────────────

/// Create the audio I/O terminal module.
/// This is a stereo module with 2 inputs (patch→speakers) and 2 outputs (mic→patch).
/// Only one can exist at a time.
pub fn audio_create() -> Option<ModuleId> {
    let h = unsafe { ffi::cardinal_audio_create() };
    if h < 0 { None } else { Some(ModuleId(h)) }
}

/// Process `frames` audio samples through the engine.
/// Writes interleaved stereo output (from the patch) into `output`.
/// If `input` is provided, it feeds interleaved stereo audio into the patch.
/// Samples are in [-1, 1] range.
pub fn audio_process(frames: usize, input: Option<&[f32]>, output: &mut [f32]) {
    assert!(output.len() >= frames * 2);
    if let Some(inp) = input {
        assert!(inp.len() >= frames * 2);
    }
    let inp_ptr = input.map_or(std::ptr::null(), |s| s.as_ptr());
    unsafe {
        ffi::cardinal_audio_process(frames as i32, inp_ptr, output.as_mut_ptr());
    }
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
