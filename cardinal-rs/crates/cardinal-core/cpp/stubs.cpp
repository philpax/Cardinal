/*
 * Stubs for Rack subsystems we don't use in headless engine mode.
 */

// Window.hpp brings in GLFW/GL/NanoVG headers — include it first.
#include <window/Window.hpp>
#include <nanosvg.h>
#include <nanovg_gl_utils.h>

// ── Window member function stubs ────────────────────────────────────
namespace rack {
namespace window {
    Window::~Window() {}
    int Window::getMods() { return 0; }
    void Window::cursorLock() {}
    void Window::cursorUnlock() {}
    bool Window::isCursorLocked() { return false; }
    bool Window::isFullScreen() { return false; }
    static int s_fbCount = 0;
    int& Window::fbCount() { return s_fbCount; }
    static bool s_fbDirty = true;
    bool& Window::fbDirtyOnSubpixelChange() { return s_fbDirty; }
    double Window::getFrameDurationRemaining() { return 0.0; }
    std::shared_ptr<Font> Window::loadFont(const std::string&) { return nullptr; }
    std::shared_ptr<Image> Window::loadImage(const std::string&) { return nullptr; }
    void generateScreenshot() {}
}
}

// ── Blendish helper ─────────────────────────────────────────────────
extern "C" float bnd_clamp(float v, float mn, float mx) {
    return (v > mx) ? mx : (v < mn) ? mn : v;
}

// ── GLFW stubs ──────────────────────────────────────────────────────
void glfwSetClipboardString(GLFWwindow*, const char*) {}
const char* glfwGetClipboardString(GLFWwindow*) { return ""; }
int glfwGetKeyScancode(int) { return 0; }
const char* glfwGetKeyName(int, int) { return ""; }

// ── NanoVG GL utility stubs ─────────────────────────────────────────
NVGLUframebuffer* nvgluCreateFramebuffer(NVGcontext*, int, int, int) { return nullptr; }
void nvgluBindFramebuffer(NVGLUframebuffer*) {}
void nvgluDeleteFramebuffer(NVGLUframebuffer*) {}

// ── NanoSVG stubs ───────────────────────────────────────────────────
extern "C" {
    NSVGimage* nsvgParse(char*, const char*, float) { return nullptr; }
    NSVGimage* nsvgParseFromFile(const char*, const char*, float) { return nullptr; }
    void nsvgDelete(NSVGimage*) {}
}

// ── osdialog stubs ──────────────────────────────────────────────────
#include <osdialog.h>

char* osdialog_file(osdialog_file_action action, const char* dir,
                    const char* filename, osdialog_filters* filters) {
    (void)action; (void)dir; (void)filename; (void)filters;
    return nullptr;
}
int osdialog_message(osdialog_message_level level,
                     osdialog_message_buttons buttons, const char* msg) {
    (void)level; (void)buttons; (void)msg;
    return 0;
}
osdialog_filters* osdialog_filters_parse(const char*) { return nullptr; }
void osdialog_filters_free(osdialog_filters*) {}

// ── Standard includes ───────────────────────────────────────────────
#include <common.hpp>
#include <math.hpp>
#include <plugin/Plugin.hpp>
#include <plugin/Model.hpp>
#include <engine/Cable.hpp>
#include <engine/Module.hpp>
#include <context.hpp>
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
namespace rack {
namespace asset {
    std::string systemDir;
    std::string userDir;
    std::string bundlePath;

    std::string system(std::string filename) {
        return systemDir + "/" + filename;
    }
    std::string user(std::string filename) {
        return userDir + "/" + filename;
    }
    std::string plugin(rack::plugin::Plugin*, std::string filename) {
        return filename;
    }
}
}

// ── Plugin registry stubs ───────────────────────────────────────────
namespace rack {
namespace plugin {
    std::string pluginsPath;
    std::vector<Plugin*> plugins;

    Plugin* getPlugin(const std::string&) { return nullptr; }
    Plugin* getPluginFallback(const std::string&) { return nullptr; }
    Model* getModel(const std::string&, const std::string&) { return nullptr; }
    Model* getModelFallback(const std::string&, const std::string&) { return nullptr; }
    Model* modelFromJson(json_t*) { return nullptr; }
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
