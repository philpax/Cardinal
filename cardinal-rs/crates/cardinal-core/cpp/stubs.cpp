/*
 * Stubs for Rack subsystems not needed in the bridge.
 * This provides link-time symbols for code that references windowing,
 * audio I/O, MIDI, etc.
 */

#include <audio.hpp>
#include <app/Scene.hpp>
// Include order matters: system.hpp and context.hpp must be visible
// to Window.hpp and our function bodies.
#include <system.hpp>
#include <context.hpp>
#include <window/Window.hpp>
#include <nanosvg.h>

// ── Window stubs ────────────────────────────────────────────────────
// The real Window is complex (owns GL context, manages GLFW).
// We provide a minimal implementation since our bridge manages
// EGL + NanoVG directly.

namespace rack {
namespace window {

struct Window::Internal {
    std::map<std::string, std::shared_ptr<Font>> fontCache;
    std::map<std::string, std::shared_ptr<Image>> imageCache;
};

Window::Window() {
    internal = new Internal;
}

Window::~Window() {
    delete internal;
}

math::Vec Window::getSize() { return math::Vec(1920, 1080); }
void Window::setSize(math::Vec) {}
void Window::close() {}
void Window::cursorLock() {}
void Window::cursorUnlock() {}
bool Window::isCursorLocked() { return false; }
int Window::getMods() { return 0; }
void Window::setFullScreen(bool) {}
bool Window::isFullScreen() { return false; }
double Window::getMonitorRefreshRate() { return 60.0; }
double Window::getFrameTime() { return 0.0; }
double Window::getLastFrameDuration() { return 1.0 / 60.0; }
double Window::getFrameDurationRemaining() { return 1.0 / 60.0; }

std::shared_ptr<Font> Window::loadFont(const std::string& filename) {
    if (!vg) return nullptr;  // headless — no NanoVG context

    auto& cache = internal->fontCache;
    auto it = cache.find(filename);
    if (it != cache.end())
        return it->second;

    auto font = std::make_shared<Font>();
    try {
        font->loadFile(filename, vg);
    } catch (...) {
        fprintf(stderr, "cardinal: failed to load font %s\n", filename.c_str());
        return nullptr;
    }
    cache[filename] = font;
    return font;
}

std::shared_ptr<Image> Window::loadImage(const std::string& filename) {
    if (!vg) return nullptr;  // headless — no NanoVG context

    auto& cache = internal->imageCache;
    auto it = cache.find(filename);
    if (it != cache.end())
        return it->second;

    auto image = std::make_shared<Image>();
    try {
        image->loadFile(filename, vg);
    } catch (...) {
        fprintf(stderr, "cardinal: failed to load image %s\n", filename.c_str());
        return nullptr;
    }
    cache[filename] = image;
    return image;
}

static bool s_fbDirty = true;
bool& Window::fbDirtyOnSubpixelChange() { return s_fbDirty; }
static int s_fbCount = 0;
int& Window::fbCount() { return s_fbCount; }

void generateScreenshot() {}

// Font/Image (normally in Window.cpp)
Font::~Font() {}
void Font::loadFile(const std::string& filename, NVGcontext* vg) {
    this->vg = vg;
    std::string name = rack::system::getStem(filename);
    size_t size;
    uint8_t* data = rack::system::readFile(filename, &size);
    if (!data) throw Exception("Failed to read font %s", filename.c_str());
    handle = nvgCreateFontMem(vg, name.c_str(), data, size, 1);
    if (handle < 0) throw Exception("Failed to load font %s", filename.c_str());
}

std::shared_ptr<Font> Font::load(const std::string& filename) {
    return APP->window->loadFont(filename);
}

Image::~Image() {
    if (handle >= 0 && vg)
        nvgDeleteImage(vg, handle);
}

void Image::loadFile(const std::string& filename, NVGcontext* vg) {
    this->vg = vg;
    std::vector<uint8_t> data = rack::system::readFile(filename);
    handle = nvgCreateImageMem(vg, NVG_IMAGE_REPEATX | NVG_IMAGE_REPEATY,
                                data.data(), data.size());
    if (handle <= 0) throw Exception("Failed to load image %s", filename.c_str());
}

std::shared_ptr<Image> Image::load(const std::string& filename) {
    return APP->window->loadImage(filename);
}

// SVG loading provided by Rack/src/window/Svg.cpp (not stubbed)

}  // namespace window
}  // namespace rack

// ── GLFW stubs ──────────────────────────────────────────────────────
// ── Scene stubs ─────────────────────────────────────────────────────
namespace rack { namespace app {
    math::Vec Scene::getMousePos() { return mousePos; }
}}

// ── Audio port stubs (we use our own AudioIO terminal module) ───────
namespace rack { namespace audio {
    Port::Port() {}
    Port::~Port() {}
    void Port::setDriverId(int) {}
    int Port::getDriverId() { return 0; }
    float Port::getSampleRate() { return 0; }
    int Port::getBlockSize() { return 0; }
    int Port::getNumInputs() { return 0; }
    int Port::getNumOutputs() { return 0; }
    void Port::fromJson(json_t*) {}
    json_t* Port::toJson() { return json_object(); }
    std::string Port::getDeviceName(int) { return ""; }
    std::vector<int> Port::getDeviceIds() { return {}; }
    int Port::getDeviceNumInputs(int) { return 0; }
    int Port::getDeviceNumOutputs(int) { return 0; }
    int Port::getDeviceId() { return 0; }
    Device* Port::getDevice() { return nullptr; }
}}

// ── App stubs for skipped subsystems ────────────────────────────────
#include <midi.hpp>
#include <app/AudioDisplay.hpp>
#include <app/MidiDisplay.hpp>
namespace rack { namespace app {
    void appendMidiMenu(ui::Menu*, rack::midi::Port*) {}
    void AudioDisplay::setAudioPort(audio::Port*) {}
}}

// osdialog prompt stub
#include <osdialog.h>
char* osdialog_prompt(osdialog_message_level level, const char* message, const char* text) {
    (void)level; (void)message; (void)text;
    return nullptr;
}

// ── GLFW stubs ──────────────────────────────────────────────────────
void glfwSetClipboardString(GLFWwindow*, const char*) {}
const char* glfwGetClipboardString(GLFWwindow*) { return ""; }
int glfwGetKeyScancode(int) { return 0; }
const char* glfwGetKeyName(int, int) { return ""; }
int glfwGetKey(GLFWwindow*, int) { return 0; }
double glfwGetTime() { return 0.0; }
void glfwSetCursor(GLFWwindow*, GLFWcursor*) {}
GLFWcursor* glfwCreateStandardCursor(int) { return nullptr; }

// ── Blendish helper ─────────────────────────────────────────────────
extern "C" float bnd_clamp(float v, float mn, float mx) {
    return (v > mx) ? mx : (v < mn) ? mn : v;
}

// ── Standard includes ───────────────────────────────────────────────
#include <common.hpp>
#include <math.hpp>
#include <plugin/Plugin.hpp>
#include <plugin/Model.hpp>
#include <engine/Cable.hpp>
#include <engine/Module.hpp>
#include <context.hpp>
#include <system.hpp>
#include <midiloopback.hpp>

#include <string>
#include <vector>
#include <cstdarg>
#include <cstdio>

// ── Version globals ─────────────────────────────────────────────────
namespace rack {
    const std::string APP_NAME = "Cardinal";
    const std::string APP_EDITION = "";
    const std::string APP_EDITION_NAME = "";
    const std::string APP_VERSION_MAJOR = "2";
    const std::string APP_VERSION = "2.5.2";
    const std::string APP_OS = "lin";
    const std::string APP_OS_NAME = "Linux";
    const std::string APP_CPU = "x64";
    const std::string APP_CPU_NAME = "x64";
    const std::string API_URL = "";

    Exception::Exception(const char* format, ...) {
        va_list args;
        va_start(args, format);
        char buf[4096];
        vsnprintf(buf, sizeof(buf), format, args);
        va_end(args);
        msg = buf;
    }
}

// ── Asset stubs ─────────────────────────────────────────────────────
// Real implementations are in custom/asset.cpp equivalent in plugin_init.cpp
// These are the base functions from rack's asset.hpp
namespace rack {
namespace asset {
    std::string configDir;
    std::string userDir;
    std::string systemDir;
    std::string bundlePath;

    std::string config(std::string filename) {
        return rack::system::join(configDir, filename);
    }
    std::string system(std::string filename) {
        return rack::system::join(systemDir, bundlePath.empty() ? filename : filename);
    }
    std::string user(std::string filename) {
        return rack::system::join(userDir, filename);
    }
    std::string plugin(rack::plugin::Plugin* plugin, std::string filename) {
        if (!plugin) return filename;
        return rack::system::join(plugin->path, filename);
    }
}
}

// ── osdialog stubs ──────────────────────────────────────────────────
#include <osdialog.h>

char* osdialog_file(osdialog_file_action, const char*, const char*, osdialog_filters*) {
    return nullptr;
}
int osdialog_message(osdialog_message_level, osdialog_message_buttons, const char*) {
    return 0;
}
osdialog_filters* osdialog_filters_parse(const char*) { return nullptr; }
void osdialog_filters_free(osdialog_filters*) {}

// ── Plugin registry ─────────────────────────────────────────────────
// ── Null stubs for model pointers missing from current submodule revisions ──
#include <plugin/Model.hpp>
rack::plugin::Model* modelViz = nullptr;
rack::plugin::Model* modelUnity = nullptr;
rack::plugin::Model* modelMidiThing = nullptr;  // Befaco (needs MIDI hw)
rack::plugin::Model* modelANTN = nullptr;       // Bidoo (needs curl)
// ChowDSP models (source files in dep subdirs not compiled)
rack::plugin::Model* modelChowTape = nullptr;
rack::plugin::Model* modelChowPhaserFeedback = nullptr;
rack::plugin::Model* modelChowPhaserMod = nullptr;
rack::plugin::Model* modelChowFDN = nullptr;
rack::plugin::Model* modelChowRNN = nullptr;
rack::plugin::Model* modelChowModal = nullptr;
rack::plugin::Model* modelChowDer = nullptr;
rack::plugin::Model* modelWarp = nullptr;
rack::plugin::Model* modelCredit = nullptr;
rack::plugin::Model* modelChowPulse = nullptr;
rack::plugin::Model* modelChowTapeCompression = nullptr;
rack::plugin::Model* modelChowTapeChew = nullptr;
rack::plugin::Model* modelChowTapeDegrade = nullptr;
rack::plugin::Model* modelChowTapeLoss = nullptr;
rack::plugin::Model* modelChowChorus = nullptr;
rack::plugin::Model* modelWerner = nullptr;     // repelzen (renamed away)

// hostTerminalModels — used by Engine to identify terminal modules.
// Populated by bridge.cpp when creating the AudioIO module.
std::vector<rack::plugin::Model*> hostTerminalModels;

// ── Asset path helpers for plugin registration ──────────────────────
namespace rack { namespace asset {
    std::string pluginManifest(const std::string& dirname) {
        return rack::system::join(systemDir, "..", "..", "plugins", dirname, "plugin.json");
    }
    std::string pluginPath(const std::string& dirname) {
        return rack::system::join(systemDir, "..", "..", "plugins", dirname);
    }
}}

namespace rack {
namespace plugin {
    std::string pluginsPath;
    std::vector<Plugin*> plugins;

    Plugin* getPlugin(const std::string& slug) {
        for (auto* p : plugins)
            if (p->slug == slug) return p;
        return nullptr;
    }
    Plugin* getPluginFallback(const std::string& slug) {
        return getPlugin(slug);
    }
    Model* getModel(const std::string& pluginSlug, const std::string& modelSlug) {
        if (auto* p = getPlugin(pluginSlug))
            return p->getModel(modelSlug);
        return nullptr;
    }
    Model* getModelFallback(const std::string& pluginSlug, const std::string& modelSlug) {
        return getModel(pluginSlug, modelSlug);
    }
    Model* modelFromJson(json_t* moduleJ) {
        std::string pluginSlug = json_string_value(json_object_get(moduleJ, "plugin"));
        std::string modelSlug = json_string_value(json_object_get(moduleJ, "model"));
        return getModel(pluginSlug, modelSlug);
    }
    bool isSlugValid(const std::string&) { return true; }
    std::string normalizeSlug(const std::string& slug) { return slug; }
    void settingsMergeJson(json_t*) {}
}

namespace library {
    void init() {}
    void destroy() {}
}

namespace midiloopback {
    Context::Context() {}
    Context::~Context() {}
    void init() {}
}
}
