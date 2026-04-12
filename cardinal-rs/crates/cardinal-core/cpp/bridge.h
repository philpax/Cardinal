#pragma once
#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// ── Forward declarations ────────────────────────────────────────────
typedef struct NVGcontext NVGcontext;

// ── Opaque handle types ──────────────────────────────────────────────
typedef int64_t ModuleHandle;
typedef int64_t CableHandle;

// ── Port / param metadata ────────────────────────────────────────────
typedef struct {
    int port_id;
    const char* name;
    float x;                // position within module panel (px)
    float y;
} PortInfo;

typedef struct {
    int param_id;
    const char* name;
    float min_value;
    float max_value;
    float default_value;
    float value;
    float x;
    float y;
} ParamInfo;

typedef struct {
    const char* plugin_slug;
    const char* model_slug;
    const char* model_name;
} ModuleCatalogEntry;

// ── Lifecycle ────────────────────────────────────────────────────────
/// Initialise the Rack engine and rendering context. Call once.
/// `resource_dir` should point to the Cardinal repo root (for finding
/// plugin resources, SVGs, fonts).
int cardinal_init(float sample_rate, const char* resource_dir);

/// Shut everything down.
void cardinal_shutdown(void);

// ── Module catalogue ─────────────────────────────────────────────────
int cardinal_catalog_count(void);
int cardinal_catalog_list(ModuleCatalogEntry* out, int max_entries);

// ── Module management ────────────────────────────────────────────────
ModuleHandle cardinal_module_create(const char* plugin_slug, const char* model_slug);
void cardinal_module_destroy(ModuleHandle h);

/// Module dimensions in pixels (Rack grid, 1 HP = 15 px).
void cardinal_module_get_size(ModuleHandle h, float* width, float* height);

int cardinal_module_get_inputs(ModuleHandle h, PortInfo* out, int max);
int cardinal_module_get_outputs(ModuleHandle h, PortInfo* out, int max);
int cardinal_module_get_params(ModuleHandle h, ParamInfo* out, int max);

float cardinal_module_get_param(ModuleHandle h, int param_id);
void  cardinal_module_set_param(ModuleHandle h, int param_id, float value);

float cardinal_module_get_input_voltage(ModuleHandle h, int port_id);
float cardinal_module_get_output_voltage(ModuleHandle h, int port_id);

// ── NanoVG context ──────────────────────────────────────────────────
/// Set the NanoVG contexts used by plugin widgets for font/image loading.
void cardinal_set_vg(NVGcontext* vg, NVGcontext* fb_vg);

// ── Rendering ────────────────────────────────────────────────────────
/// Render a module widget using the provided NanoVG context.
/// Returns 1 on success, 0 on failure.
int cardinal_module_render(ModuleHandle h, NVGcontext* vg, int width, int height);

// ── Cable management ─────────────────────────────────────────────────
CableHandle cardinal_cable_create(
    ModuleHandle out_module, int out_port,
    ModuleHandle in_module,  int in_port);
void cardinal_cable_destroy(CableHandle h);

// ── Audio I/O ────────────────────────────────────────────────────────
/// Create a stereo audio I/O terminal module.
/// This module has 2 input ports (for audio going to speakers) and
/// 2 output ports (for audio coming from a mic/file).
/// Returns module handle or -1 on error.
ModuleHandle cardinal_audio_create(void);

/// Process `frames` audio samples and write interleaved stereo output
/// (from Rack) into `output_buf`. The buffer must hold frames*2 floats.
/// Samples are in [-1, 1] range.
/// If `input_buf` is non-NULL, it provides interleaved stereo input
/// (into Rack) for the same block.
void cardinal_audio_process(int frames, const float* input_buf, float* output_buf);

// ── Engine stepping ──────────────────────────────────────────────────
void cardinal_process(int frames);
float cardinal_get_sample_rate(void);

#ifdef __cplusplus
}
#endif
