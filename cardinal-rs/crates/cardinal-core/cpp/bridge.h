#pragma once
#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// ── Opaque handle types ──────────────────────────────────────────────
typedef int64_t ModuleHandle;
typedef int64_t CableHandle;

// ── Port / param metadata ────────────────────────────────────────────
typedef struct {
    int port_id;
    const char* name;       // valid until module removed
    float x;                // position within module panel (px)
    float y;
} PortInfo;

typedef struct {
    int param_id;
    const char* name;       // valid until module removed
    float min_value;
    float max_value;
    float default_value;
    float value;
    float x;                // position within module panel (px)
    float y;
} ParamInfo;

typedef struct {
    const char* plugin_slug;
    const char* model_slug;
    const char* model_name;
} ModuleCatalogEntry;

// ── Lifecycle ────────────────────────────────────────────────────────
/// Initialise the Rack engine. Call once.
int cardinal_init(float sample_rate);
/// Shut everything down.
void cardinal_shutdown(void);

// ── Module catalogue ─────────────────────────────────────────────────
/// Number of available module types.
int cardinal_catalog_count(void);
/// Fill `out` with up to `max_entries` catalog entries. Returns count written.
int cardinal_catalog_list(ModuleCatalogEntry* out, int max_entries);

// ── Module management ────────────────────────────────────────────────
/// Spawn a module by slug pair. Returns handle (>=0) or -1 on error.
ModuleHandle cardinal_module_create(const char* plugin_slug, const char* model_slug);
/// Remove and destroy a module.
void cardinal_module_destroy(ModuleHandle h);

/// Module dimensions in pixels (Rack grid, 1 HP = 15 px).
void cardinal_module_get_size(ModuleHandle h, float* width, float* height);

/// Query ports. Returns count; fills `out` up to `max`.
int cardinal_module_get_inputs(ModuleHandle h, PortInfo* out, int max);
int cardinal_module_get_outputs(ModuleHandle h, PortInfo* out, int max);
int cardinal_module_get_params(ModuleHandle h, ParamInfo* out, int max);

/// Read / write a parameter value.
float cardinal_module_get_param(ModuleHandle h, int param_id);
void  cardinal_module_set_param(ModuleHandle h, int param_id, float value);

/// Read port voltages (channel 0).
float cardinal_module_get_input_voltage(ModuleHandle h, int port_id);
float cardinal_module_get_output_voltage(ModuleHandle h, int port_id);

// ── Cable management ─────────────────────────────────────────────────
/// Connect two ports. Returns handle (>=0) or -1 on error.
CableHandle cardinal_cable_create(
    ModuleHandle out_module, int out_port,
    ModuleHandle in_module,  int in_port);
/// Disconnect and destroy a cable.
void cardinal_cable_destroy(CableHandle h);

// ── Engine stepping ──────────────────────────────────────────────────
/// Process `frames` audio samples.
void cardinal_process(int frames);

/// Get current sample rate.
float cardinal_get_sample_rate(void);

#ifdef __cplusplus
}
#endif
