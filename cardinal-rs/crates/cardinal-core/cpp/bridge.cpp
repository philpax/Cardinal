/*
 * Cardinal Rust Bridge — real plugin loading + NanoVG rendering
 *
 * Creates an offscreen EGL/GL context, initialises the Rack engine and
 * NanoVG, loads real VCV Rack plugins (starting with Fundamental), and
 * can render any ModuleWidget to an RGBA pixel buffer.
 */

#include "bridge.h"

#include <engine/Engine.hpp>
#include <engine/Module.hpp>
#include <engine/Cable.hpp>
#include <engine/TerminalModule.hpp>
#include <app/ModuleWidget.hpp>
#include <app/CableWidget.hpp>
#include <app/RackWidget.hpp>
#include <app/SvgPanel.hpp>
#include <widget/FramebufferWidget.hpp>
#include <context.hpp>
#include <settings.hpp>
#include <plugin.hpp>
#include <random.hpp>
#include <logger.hpp>
#include <asset.hpp>
#include <system.hpp>
#include <window/Window.hpp>

#include <cstring>
#include <unordered_map>

#include <EGL/egl.h>
#include <GL/gl.h>

// We need glewInit() to populate GL extension function pointers for NanoVG.
// Cardinal provides a stub GL/glew.h that shadows the real one, so we
// declare the handful of GLEW symbols we need directly.
extern "C" {
    extern unsigned char glewExperimental;
    typedef unsigned int GLenum;
    GLenum glewInit(void);
    const unsigned char* glewGetErrorString(GLenum error);
}
#define GLEW_OK 0

// Forward declarations
// Plugin registration is handled by Rust (cardinal_plugins_registry::register_all_plugins)
extern std::vector<rack::plugin::Model*> hostTerminalModels;

// ── AudioIO terminal module ──────────────────────────────────────────
// Bridges between the Rack engine and external audio (via the Rust side).
// 2 inputs (Rack→speakers), 2 outputs (mic→Rack).

static constexpr int AUDIO_IO_MAX_FRAMES = 8192;

struct AudioIOModule : rack::engine::TerminalModule {
    // Double-buffered: Rust writes input here, reads output from here.
    // Protected by the engine mutex (process is single-threaded).
    float inputBuf[AUDIO_IO_MAX_FRAMES * 2] = {};   // interleaved stereo, mic→Rack
    float outputBuf[AUDIO_IO_MAX_FRAMES * 2] = {};   // interleaved stereo, Rack→speakers
    int frameIndex = 0;

    AudioIOModule() {
        config(0, 2, 2, 0);
        configInput(0, "Left from speakers");
        configInput(1, "Right from speakers");
        configOutput(0, "Left from input");
        configOutput(1, "Right from input");
    }

    // Runs BEFORE all other modules — provide audio input to the patch
    void processTerminalInput(const rack::engine::Module::ProcessArgs&) override {
        int k = frameIndex;
        if (k < AUDIO_IO_MAX_FRAMES) {
            outputs[0].setVoltage(inputBuf[k * 2 + 0] * 10.f);
            outputs[1].setVoltage(inputBuf[k * 2 + 1] * 10.f);
        }
    }

    // Runs AFTER all other modules — capture audio output from the patch
    void processTerminalOutput(const rack::engine::Module::ProcessArgs&) override {
        int k = frameIndex;
        if (k < AUDIO_IO_MAX_FRAMES) {
            float l = inputs[0].getVoltageSum() * 0.1f;
            float r = inputs[1].getVoltageSum() * 0.1f;
            // Clamp to [-1, 1]
            outputBuf[k * 2 + 0] = std::max(-1.f, std::min(1.f, l));
            outputBuf[k * 2 + 1] = std::max(-1.f, std::min(1.f, r));
        }
        frameIndex++;
    }
};

// Model for AudioIO (needed so the engine recognises it as a terminal module)
struct AudioIOModel : rack::plugin::Model {
    rack::engine::Module* createModule() override {
        auto* m = new AudioIOModule();
        m->model = this;
        return m;
    }
    rack::app::ModuleWidget* createModuleWidget(rack::engine::Module*) override {
        return nullptr;  // No widget — this is a bridge module
    }
};

static AudioIOModel g_audioIOModel;
static AudioIOModule* g_audioIO = nullptr;

// ── Internal state ───────────────────────────────────────────────────

static rack::Context* g_context = nullptr;
static rack::engine::Engine* g_engine = nullptr;

// EGL state
static EGLDisplay g_eglDisplay = EGL_NO_DISPLAY;
static EGLContext g_eglContext = EGL_NO_CONTEXT;
static EGLSurface g_eglSurface = EGL_NO_SURFACE;

// NanoVG context
static NVGcontext* g_vg = nullptr;
static NVGcontext* g_fbVg = nullptr;

// Module tracking
struct ModuleEntry {
    rack::engine::Module* module = nullptr;
    rack::app::ModuleWidget* widget = nullptr;
    rack::plugin::Model* model = nullptr;
};
static std::unordered_map<int64_t, ModuleEntry> g_modules;
static std::unordered_map<int64_t, rack::engine::Cable*> g_cables;

// ── EGL offscreen context ────────────────────────────────────────────

static void shutdownEGL();

static bool initEGL() {
    g_eglDisplay = eglGetDisplay(EGL_DEFAULT_DISPLAY);
    if (g_eglDisplay == EGL_NO_DISPLAY) {
        fprintf(stderr, "cardinal: eglGetDisplay failed\n");
        return false;
    }

    EGLint major, minor;
    if (!eglInitialize(g_eglDisplay, &major, &minor)) {
        fprintf(stderr, "cardinal: eglInitialize failed\n");
        return false;
    }

    // Request OpenGL (not OpenGL ES)
    eglBindAPI(EGL_OPENGL_API);

    EGLint configAttribs[] = {
        EGL_SURFACE_TYPE, EGL_PBUFFER_BIT,
        EGL_RENDERABLE_TYPE, EGL_OPENGL_BIT,
        EGL_RED_SIZE, 8,
        EGL_GREEN_SIZE, 8,
        EGL_BLUE_SIZE, 8,
        EGL_ALPHA_SIZE, 8,
        EGL_STENCIL_SIZE, 8,
        EGL_NONE
    };

    EGLConfig config;
    EGLint numConfigs;
    if (!eglChooseConfig(g_eglDisplay, configAttribs, &config, 1, &numConfigs) || numConfigs == 0) {
        fprintf(stderr, "cardinal: eglChooseConfig failed\n");
        return false;
    }

    // Create a small pbuffer surface (NanoVG FBO rendering doesn't need a real surface)
    EGLint pbufferAttribs[] = {
        EGL_WIDTH, 16,
        EGL_HEIGHT, 16,
        EGL_NONE
    };
    g_eglSurface = eglCreatePbufferSurface(g_eglDisplay, config, pbufferAttribs);

    EGLint contextAttribs[] = {
        EGL_CONTEXT_MAJOR_VERSION, 2,
        EGL_CONTEXT_MINOR_VERSION, 0,
        EGL_NONE
    };
    g_eglContext = eglCreateContext(g_eglDisplay, config, EGL_NO_CONTEXT, contextAttribs);
    if (g_eglContext == EGL_NO_CONTEXT) {
        fprintf(stderr, "cardinal: eglCreateContext failed\n");
        return false;
    }

    if (!eglMakeCurrent(g_eglDisplay, g_eglSurface, g_eglSurface, g_eglContext)) {
        fprintf(stderr, "cardinal: eglMakeCurrent failed\n");
        return false;
    }

    // Initialize GLEW so GL extension function pointers are loaded
    // (NanoVG GL2 backend calls GL functions through GLEW)
    glewExperimental = GL_TRUE;
    GLenum glewErr = glewInit();
    if (glewErr != GLEW_OK) {
        fprintf(stderr, "cardinal: glewInit failed: %s\n", glewGetErrorString(glewErr));
        shutdownEGL();
        return false;
    }

    // Verify the GL context is actually functional
    const char* glVersion = (const char*)glGetString(GL_VERSION);
    if (!glVersion) {
        fprintf(stderr, "cardinal: GL context broken (glGetString returned null)\n");
        shutdownEGL();
        return false;
    }
    fprintf(stderr, "cardinal: EGL/GL initialized — %s\n", glVersion);

    return true;
}

static void shutdownEGL() {
    if (g_eglDisplay != EGL_NO_DISPLAY) {
        eglMakeCurrent(g_eglDisplay, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
        if (g_eglContext != EGL_NO_CONTEXT)
            eglDestroyContext(g_eglDisplay, g_eglContext);
        if (g_eglSurface != EGL_NO_SURFACE)
            eglDestroySurface(g_eglDisplay, g_eglSurface);
        eglTerminate(g_eglDisplay);
    }
    g_eglDisplay = EGL_NO_DISPLAY;
    g_eglContext = EGL_NO_CONTEXT;
    g_eglSurface = EGL_NO_SURFACE;
}

// ── NanoVG init ──────────────────────────────────────────────────────

// NanoVG GL2 functions (implementation in nanovg_gl_impl.c)
#define NANOVG_GL2
#include <nanovg_gl.h>

static bool initNanoVG() {
    g_vg = nvgCreateGL2(NVG_ANTIALIAS | NVG_STENCIL_STROKES);
    if (!g_vg) {
        fprintf(stderr, "cardinal: nvgCreateGL2 failed\n");
        return false;
    }
    g_fbVg = nvgCreateGL2(NVG_ANTIALIAS | NVG_STENCIL_STROKES);
    if (!g_fbVg) {
        fprintf(stderr, "cardinal: nvgCreateGL2 (fb) failed\n");
        return false;
    }
    return true;
}

// ── Lifecycle ────────────────────────────────────────────────────────

int cardinal_init(float sample_rate, const char* resource_dir) {
    fprintf(stderr, "cardinal: [init] logger...\n");
    rack::logger::init();
    fprintf(stderr, "cardinal: [init] random...\n");
    rack::random::init();

    // Set up asset paths
    std::string root(resource_dir);
    rack::asset::systemDir = root + "/src/Rack/res";
    rack::asset::userDir = root + "/user_data";
    rack::asset::bundlePath = "";  // Empty = local source build mode
    fprintf(stderr, "cardinal: [init] systemDir=%s\n", rack::asset::systemDir.c_str());

    // Create user_data dir if missing
    rack::system::createDirectories(rack::asset::userDir);

    rack::settings::headless = false;  // We want rendering
    rack::settings::devMode = true;

    // Init EGL
    fprintf(stderr, "cardinal: [init] EGL...\n");
    if (!initEGL()) {
        fprintf(stderr, "cardinal: EGL init failed, falling back to headless\n");
        rack::settings::headless = true;
    }

    // Init NanoVG (only if we have GL)
    if (!rack::settings::headless) {
        fprintf(stderr, "cardinal: [init] NanoVG...\n");
        if (!initNanoVG()) {
            fprintf(stderr, "cardinal: NanoVG init failed, falling back to headless\n");
            rack::settings::headless = true;
        }
    }

    fprintf(stderr, "cardinal: [init] headless=%d\n", (int)rack::settings::headless);

    // Create context
    fprintf(stderr, "cardinal: [init] creating Context...\n");
    g_context = new rack::Context();
    rack::contextSet(g_context);

    // Create engine
    fprintf(stderr, "cardinal: [init] creating Engine...\n");
    g_engine = new rack::engine::Engine();
    g_context->engine = g_engine;
    g_engine->setSampleRate(sample_rate);

    // Create a Window so APP->window is never null.
    // Plugin code (Font::load, Image::load, Svg::load) accesses APP->window.
    fprintf(stderr, "cardinal: [init] creating Window...\n");
    auto* window = new rack::window::Window();
    g_context->window = window;

    if (!rack::settings::headless) {
        // Wire our NanoVG contexts into the Window
        window->vg = g_vg;
        window->fbVg = g_fbVg;
    }
    // If headless, window->vg stays null; loadFont/loadImage will return nullptr.

    // Plugin registration is deferred to Rust side
    // (cardinal_plugins_registry::register_all_plugins() called from cardinal_core::init())

    fprintf(stderr, "cardinal: [init] done — %d plugins, %d models (headless=%d)\n",
            (int)rack::plugin::plugins.size(),
            cardinal_catalog_count(),
            (int)rack::settings::headless);

    return 0;
}

void cardinal_shutdown(void) {
    // Clean up modules
    for (auto& [id, entry] : g_modules) {
        if (entry.widget) {
            delete entry.widget;
        }
        if (entry.module) {
            g_engine->removeModule(entry.module);
            delete entry.module;
        }
    }
    g_modules.clear();
    g_cables.clear();

    // Detach NanoVG from Window before destroying them
    if (g_context && g_context->window) {
        g_context->window->vg = nullptr;
        g_context->window->fbVg = nullptr;
        delete g_context->window;
        g_context->window = nullptr;
    }

    if (g_fbVg) { nvgDeleteGL2(g_fbVg); g_fbVg = nullptr; }
    if (g_vg) { nvgDeleteGL2(g_vg); g_vg = nullptr; }

    if (g_engine) {
        g_context->engine = nullptr;
        delete g_engine;
        g_engine = nullptr;
    }

    delete g_context;
    g_context = nullptr;

    shutdownEGL();
    rack::logger::destroy();
}

// ── Module catalogue ─────────────────────────────────────────────────

int cardinal_catalog_count(void) {
    int count = 0;
    for (auto* plugin : rack::plugin::plugins)
        count += plugin->models.size();
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

// ── Module management ────────────────────────────────────────────────

static rack::plugin::Model* findModel(const char* plugin_slug, const char* model_slug) {
    for (auto* plugin : rack::plugin::plugins) {
        if (plugin->slug != plugin_slug) continue;
        for (auto* m : plugin->models) {
            if (m->slug == model_slug)
                return m;
        }
        break;
    }
    return nullptr;
}

ModuleHandle cardinal_module_create(const char* plugin_slug, const char* model_slug) {
    if (!g_engine) return -1;

    rack::plugin::Model* model = findModel(plugin_slug, model_slug);
    if (!model) return -1;

    // Create the engine-side module
    rack::engine::Module* module = model->createModule();
    if (!module) return -1;

    // Add to engine
    g_engine->addModule(module);
    int64_t handle = module->id;

    // Create the widget (for rendering)
    rack::app::ModuleWidget* widget = nullptr;
    if (!rack::settings::headless) {
        try {
            widget = model->createModuleWidget(module);
        } catch (...) {
            fprintf(stderr, "cardinal: failed to create widget for %s/%s\n",
                    plugin_slug, model_slug);
        }
    }

    g_modules[handle] = { module, widget, model };
    return handle;
}

void cardinal_module_destroy(ModuleHandle h) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return;

    if (it->second.widget)
        delete it->second.widget;

    g_engine->removeModule(it->second.module);
    delete it->second.module;

    g_modules.erase(it);
}

void cardinal_module_get_size(ModuleHandle h, float* width, float* height) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) { *width = 0; *height = 0; return; }

    if (it->second.widget) {
        *width = it->second.widget->box.size.x;
        *height = it->second.widget->box.size.y;
    } else {
        // Fallback for headless
        auto* mod = it->second.module;
        int numPorts = std::max(mod->getNumInputs(), mod->getNumOutputs());
        int hp = std::max(3, std::min(25, numPorts * 3 + 2));
        *width = hp * 15.f;
        *height = 380.f;
    }
}

int cardinal_module_get_inputs(ModuleHandle h, PortInfo* out, int max) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return 0;

    auto* mod = it->second.module;
    auto* widget = it->second.widget;
    int count = std::min(max, (int)mod->getNumInputs());

    for (int i = 0; i < count; i++) {
        auto* info = mod->getInputInfo(i);
        out[i].port_id = i;
        out[i].name = info ? info->name.c_str() : "";

        // Get port position from widget if available
        if (widget) {
            auto* pw = widget->getInput(i);
            if (pw) {
                out[i].x = pw->box.getCenter().x;
                out[i].y = pw->box.getCenter().y;
                continue;
            }
        }
        // Fallback layout
        out[i].x = 15.f;
        out[i].y = 80.f + i * 40.f;
    }
    return count;
}

int cardinal_module_get_outputs(ModuleHandle h, PortInfo* out, int max) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return 0;

    auto* mod = it->second.module;
    auto* widget = it->second.widget;
    int count = std::min(max, (int)mod->getNumOutputs());

    for (int i = 0; i < count; i++) {
        auto* info = mod->getOutputInfo(i);
        out[i].port_id = i;
        out[i].name = info ? info->name.c_str() : "";

        if (widget) {
            auto* pw = widget->getOutput(i);
            if (pw) {
                out[i].x = pw->box.getCenter().x;
                out[i].y = pw->box.getCenter().y;
                continue;
            }
        }
        float moduleWidth = 0, moduleHeight = 0;
        cardinal_module_get_size(h, &moduleWidth, &moduleHeight);
        out[i].x = moduleWidth - 15.f;
        out[i].y = 80.f + i * 40.f;
    }
    return count;
}

int cardinal_module_get_params(ModuleHandle h, ParamInfo* out, int max) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return 0;

    auto* mod = it->second.module;
    auto* widget = it->second.widget;
    int count = std::min(max, (int)mod->getNumParams());

    float mw = 0, mh = 0;
    cardinal_module_get_size(h, &mw, &mh);

    for (int i = 0; i < count; i++) {
        auto* pq = mod->getParamQuantity(i);
        out[i].param_id = i;
        out[i].name = pq ? pq->name.c_str() : "";
        out[i].min_value = pq ? pq->getMinValue() : 0.f;
        out[i].max_value = pq ? pq->getMaxValue() : 1.f;
        out[i].default_value = pq ? pq->getDefaultValue() : 0.f;
        out[i].value = pq ? pq->getValue() : 0.f;

        if (widget) {
            auto* pw = widget->getParam(i);
            if (pw) {
                out[i].x = pw->box.getCenter().x;
                out[i].y = pw->box.getCenter().y;
                continue;
            }
        }
        int col = i % 2, row = i / 2;
        out[i].x = mw / 2.f + (col == 0 ? -18.f : 18.f);
        out[i].y = 50.f + row * 45.f;
    }
    return count;
}

float cardinal_module_get_param(ModuleHandle h, int param_id) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return 0.f;
    return g_engine->getParamValue(it->second.module, param_id);
}

void cardinal_module_set_param(ModuleHandle h, int param_id, float value) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return;
    g_engine->setParamValue(it->second.module, param_id, value);
}

float cardinal_module_get_input_voltage(ModuleHandle h, int port_id) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return 0.f;
    if (port_id < 0 || port_id >= (int)it->second.module->getNumInputs()) return 0.f;
    return it->second.module->getInput(port_id).getVoltage();
}

float cardinal_module_get_output_voltage(ModuleHandle h, int port_id) {
    auto it = g_modules.find(h);
    if (it == g_modules.end()) return 0.f;
    if (port_id < 0 || port_id >= (int)it->second.module->getNumOutputs()) return 0.f;
    return it->second.module->getOutput(port_id).getVoltage();
}

// ── Rendering ────────────────────────────────────────────────────────

int cardinal_module_render(ModuleHandle h,
                           unsigned char* pixels, int max_width, int max_height,
                           int* out_width, int* out_height)
{
    auto it = g_modules.find(h);
    if (it == g_modules.end() || !it->second.widget || !g_vg)
        return 0;

    auto* widget = it->second.widget;
    int w = (int)widget->box.size.x;
    int h2 = (int)widget->box.size.y;

    if (w <= 0 || h2 <= 0 || w > max_width || h2 > max_height)
        return 0;

    *out_width = w;
    *out_height = h2;

    // Create FBO
    NVGLUframebuffer* fbo = nvgluCreateFramebuffer(g_vg, w, h2, 0);
    if (!fbo) return 0;

    nvgluBindFramebuffer(fbo);
    glViewport(0, 0, w, h2);
    glClearColor(0, 0, 0, 0);
    glClear(GL_COLOR_BUFFER_BIT | GL_STENCIL_BUFFER_BIT);

    // Begin NanoVG frame
    nvgBeginFrame(g_vg, w, h2, 1.0f);

    // Draw the module widget
    rack::widget::Widget::DrawArgs args;
    args.vg = g_vg;
    args.clipBox = rack::math::Rect(rack::math::Vec(0, 0),
                                     rack::math::Vec(w, h2));
    args.fb = nullptr;

    widget->draw(args);
    // Also draw layer 1 (lights/halos)
    widget->drawLayer(args, 1);

    nvgEndFrame(g_vg);

    // Read pixels back
    glReadPixels(0, 0, w, h2, GL_RGBA, GL_UNSIGNED_BYTE, pixels);

    nvgluBindFramebuffer(nullptr);
    nvgluDeleteFramebuffer(fbo);

    // OpenGL returns pixels bottom-up; flip vertically
    int stride = w * 4;
    std::vector<unsigned char> row(stride);
    for (int y = 0; y < h2 / 2; y++) {
        unsigned char* top = pixels + y * stride;
        unsigned char* bot = pixels + (h2 - 1 - y) * stride;
        memcpy(row.data(), top, stride);
        memcpy(top, bot, stride);
        memcpy(bot, row.data(), stride);
    }

    return 1;
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

    auto* cable = new rack::engine::Cable();
    cable->outputModule = out_it->second.module;
    cable->outputId = out_port;
    cable->inputModule = in_it->second.module;
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

    g_engine->removeCable(it->second);
    delete it->second;
    g_cables.erase(it);
}

// ── Audio I/O ────────────────────────────────────────────────────────

ModuleHandle cardinal_audio_create(void) {
    if (!g_engine || g_audioIO) return -1;  // only one audio module

    g_audioIOModel.slug = "AudioIO";
    g_audioIOModel.name = "Audio I/O";

    // Register as a terminal model so the engine processes it
    // before/after all other modules
    hostTerminalModels.push_back(&g_audioIOModel);

    auto* module = new AudioIOModule();
    module->model = &g_audioIOModel;
    g_engine->addModule(module);
    g_audioIO = module;

    int64_t handle = module->id;
    g_modules[handle] = { module, nullptr, &g_audioIOModel };
    return handle;
}

void cardinal_audio_process(int frames, const float* input_buf, float* output_buf) {
    if (!g_engine || !g_audioIO) {
        if (output_buf) memset(output_buf, 0, frames * 2 * sizeof(float));
        return;
    }

    if (frames > AUDIO_IO_MAX_FRAMES)
        frames = AUDIO_IO_MAX_FRAMES;

    // Copy input audio into the terminal module's buffer
    if (input_buf) {
        memcpy(g_audioIO->inputBuf, input_buf, frames * 2 * sizeof(float));
    } else {
        memset(g_audioIO->inputBuf, 0, frames * 2 * sizeof(float));
    }

    // Reset frame counter and process
    g_audioIO->frameIndex = 0;
    g_engine->stepBlock(frames);

    // Copy output audio from the terminal module's buffer
    if (output_buf) {
        memcpy(output_buf, g_audioIO->outputBuf, frames * 2 * sizeof(float));
    }
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
