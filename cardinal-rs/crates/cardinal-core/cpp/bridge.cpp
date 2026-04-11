/*
 * Cardinal Rust Bridge
 *
 * Thin C API over the VCV Rack engine, compiled from the real Rack source.
 * Provides module creation, cable patching, engine stepping, and
 * metadata queries — everything needed for an external UI to drive
 * the engine.
 */

#include "bridge.h"

#include <engine/Engine.hpp>
#include <engine/Module.hpp>
#include <engine/Cable.hpp>
#include <context.hpp>
#include <settings.hpp>
#include <plugin.hpp>
#include <random.hpp>
#include <logger.hpp>

#include <cstring>
#include <unordered_map>

// Forward declaration from test_modules.cpp
namespace test_modules { void registerTestModules(); }

// ── Internal state ─────────────────────────────────���─────────────────

static rack::Context* g_context = nullptr;
static rack::engine::Engine* g_engine = nullptr;

// Map module handles to Module pointers (handle = module->id)
static std::unordered_map<int64_t, rack::engine::Module*> g_modules;
// Map cable handles to Cable pointers (handle = cable->id)
static std::unordered_map<int64_t, rack::engine::Cable*> g_cables;
// Map module handles to their Model (for metadata queries)
static std::unordered_map<int64_t, rack::plugin::Model*> g_module_models;

// ── Lifecycle ────────────────────────────────────────────────────────

int cardinal_init(float sample_rate) {
    // Initialise Rack logger (writes to stderr)
    rack::logger::init();

    // Initialise random seed
    rack::random::init();

    // Configure settings for headless
    rack::settings::headless = true;

    // Create context
    g_context = new rack::Context();
    rack::contextSet(g_context);

    // Create engine
    g_engine = new rack::engine::Engine();
    g_context->engine = g_engine;

    // Set sample rate
    g_engine->setSampleRate(sample_rate);

    // Register built-in test modules
    test_modules::registerTestModules();

    return 0;
}

void cardinal_shutdown(void) {
    g_modules.clear();
    g_cables.clear();
    g_module_models.clear();

    if (g_engine) {
        g_engine->clear();
    }

    if (g_context) {
        // Engine is deleted by Context destructor indirectly, but we
        // manage it ourselves, so detach first.
        g_context->engine = nullptr;
        delete g_engine;
        g_engine = nullptr;

        delete g_context;
        g_context = nullptr;
    }

    rack::logger::destroy();
}

// ── Module catalogue ─────────────────────────────────────────────────

int cardinal_catalog_count(void) {
    int count = 0;
    for (auto* plugin : rack::plugin::plugins) {
        for (auto* model : plugin->models) {
            (void)model;
            count++;
        }
    }
    return count;
}

int cardinal_catalog_list(ModuleCatalogEntry* out, int max_entries) {
    int i = 0;
    for (auto* plugin : rack::plugin::plugins) {
        for (auto* model : plugin->models) {
            if (i >= max_entries) return i;
            out[i].plugin_slug = plugin->slug.c_str();
            out[i].model_slug = model->slug.c_str();
            out[i].model_name = model->name.c_str();
            i++;
        }
    }
    return i;
}

// ── Module management ────────────────────────────────��───────────────

ModuleHandle cardinal_module_create(const char* plugin_slug, const char* model_slug) {
    if (!g_engine) return -1;

    // Find model
    rack::plugin::Model* model = nullptr;
    for (auto* plugin : rack::plugin::plugins) {
        if (plugin->slug != plugin_slug) continue;
        for (auto* m : plugin->models) {
            if (m->slug == model_slug) {
                model = m;
                break;
            }
        }
        if (model) break;
    }
    if (!model) return -1;

    // Create module instance
    rack::engine::Module* module = model->createModule();
    if (!module) return -1;

    module->model = model;

    // Add to engine (assigns module->id)
    g_engine->addModule(module);

    int64_t handle = module->id;
    g_modules[handle] = module;
    g_module_models[handle] = model;

    return handle;
}

void cardinal_module_destroy(ModuleHandle h) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return;

    rack::engine::Module* module = it->second;
    g_engine->removeModule(module);
    delete module;

    g_modules.erase(it);
    g_module_models.erase(h);
}

void cardinal_module_get_size(ModuleHandle h, float* width, float* height) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) {
        *width = 0; *height = 0;
        return;
    }

    rack::engine::Module* mod = it->second;
    // Estimate width based on max(inputs, outputs, params) in HP units
    // Standard Rack module: 1 HP = 15 px, height = 380 px
    int numPorts = std::max(mod->getNumInputs(), mod->getNumOutputs());
    int hp = std::max(3, std::min(25, numPorts * 3 + 2));
    // Adjust for params
    int paramRows = (int(mod->getNumParams()) + 1) / 2;
    hp = std::max(hp, std::min(25, paramRows * 3 + 2));

    *width = hp * 15.f;
    *height = 380.f;
}

int cardinal_module_get_inputs(ModuleHandle h, PortInfo* out, int max) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return 0;

    rack::engine::Module* mod = it->second;
    int count = std::min(max, (int)mod->getNumInputs());

    float moduleWidth = 0, moduleHeight = 0;
    cardinal_module_get_size(h, &moduleWidth, &moduleHeight);

    for (int i = 0; i < count; i++) {
        auto* info = mod->getInputInfo(i);
        out[i].port_id = i;
        out[i].name = info ? info->name.c_str() : "";
        // Layout: inputs on left side, evenly spaced vertically
        out[i].x = 15.f;  // ~1 HP from left
        out[i].y = 80.f + i * 40.f;
    }
    return count;
}

int cardinal_module_get_outputs(ModuleHandle h, PortInfo* out, int max) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return 0;

    rack::engine::Module* mod = it->second;
    int count = std::min(max, (int)mod->getNumOutputs());

    float moduleWidth = 0, moduleHeight = 0;
    cardinal_module_get_size(h, &moduleWidth, &moduleHeight);

    for (int i = 0; i < count; i++) {
        auto* info = mod->getOutputInfo(i);
        out[i].port_id = i;
        out[i].name = info ? info->name.c_str() : "";
        // Layout: outputs on right side
        out[i].x = moduleWidth - 15.f;
        out[i].y = 80.f + i * 40.f;
    }
    return count;
}

int cardinal_module_get_params(ModuleHandle h, ParamInfo* out, int max) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return 0;

    rack::engine::Module* mod = it->second;
    int count = std::min(max, (int)mod->getNumParams());

    float moduleWidth = 0, moduleHeight = 0;
    cardinal_module_get_size(h, &moduleWidth, &moduleHeight);
    float centerX = moduleWidth / 2.f;

    for (int i = 0; i < count; i++) {
        auto* pq = mod->getParamQuantity(i);
        out[i].param_id = i;
        out[i].name = pq ? pq->name.c_str() : "";
        out[i].min_value = pq ? pq->getMinValue() : 0.f;
        out[i].max_value = pq ? pq->getMaxValue() : 1.f;
        out[i].default_value = pq ? pq->getDefaultValue() : 0.f;
        out[i].value = pq ? pq->getValue() : 0.f;
        // Layout: params in center, in two columns if width allows
        int col = i % 2;
        int row = i / 2;
        if (moduleWidth > 60.f) {
            out[i].x = centerX + (col == 0 ? -18.f : 18.f);
        } else {
            out[i].x = centerX;
        }
        out[i].y = 50.f + row * 45.f;
    }
    return count;
}

float cardinal_module_get_param(ModuleHandle h, int param_id) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return 0.f;
    return g_engine->getParamValue(it->second, param_id);
}

void cardinal_module_set_param(ModuleHandle h, int param_id, float value) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return;
    g_engine->setParamValue(it->second, param_id, value);
}

float cardinal_module_get_input_voltage(ModuleHandle h, int port_id) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return 0.f;
    if (port_id < 0 || port_id >= (int)it->second->getNumInputs()) return 0.f;
    return it->second->getInput(port_id).getVoltage();
}

float cardinal_module_get_output_voltage(ModuleHandle h, int port_id) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return 0.f;
    if (port_id < 0 || port_id >= (int)it->second->getNumOutputs()) return 0.f;
    return it->second->getOutput(port_id).getVoltage();
}

// ── Cable management ─────────────────────────────────────────────────

CableHandle cardinal_cable_create(
    ModuleHandle out_module, int out_port,
    ModuleHandle in_module,  int in_port)
{
    if (!g_engine) return -1;

    auto out_it = g_modules.find(out_module);
    auto in_it = g_modules.find(in_module);
    if (out_it == g_modules.end() || in_it == g_modules.end()) return -1;

    rack::engine::Cable* cable = new rack::engine::Cable();
    cable->outputModule = out_it->second;
    cable->outputId = out_port;
    cable->inputModule = in_it->second;
    cable->inputId = in_port;

    try {
        g_engine->addCable(cable);
    } catch (...) {
        delete cable;
        return -1;
    }

    int64_t handle = cable->id;
    g_cables[handle] = cable;
    return handle;
}

void cardinal_cable_destroy(CableHandle h) {
    auto it = g_cables.find(h);
    if (it == g_cables.end()) return;

    rack::engine::Cable* cable = it->second;
    g_engine->removeCable(cable);
    delete cable;

    g_cables.erase(it);
}

// ── Engine stepping ──────────────────────────────────────────────────

void cardinal_process(int frames) {
    if (!g_engine) return;
    g_engine->stepBlock(frames);
}

float cardinal_get_sample_rate(void) {
    if (!g_engine) return 0.f;
    return g_engine->getSampleRate();
}
