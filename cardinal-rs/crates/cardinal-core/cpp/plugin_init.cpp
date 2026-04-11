/*
 * Plugin initialization for the Cardinal Rust bridge.
 * Minimal version of plugins/plugins.cpp.
 */

#include <rack.hpp>
#include <plugin.hpp>

using namespace rack;
using namespace rack::plugin;

// ── Asset path helpers ──────────────────────────────────────────────

namespace rack {
namespace asset {

std::string pluginManifest(const std::string& dirname) {
    return rack::system::join(systemDir, "..", "..", "plugins", dirname, "plugin.json");
}

std::string pluginPath(const std::string& dirname) {
    return rack::system::join(systemDir, "..", "..", "plugins", dirname);
}

}
}

// ── StaticPluginLoader ──────────────────────────────────────────────

struct StaticPluginLoader {
    Plugin* const plugin;
    FILE* file;
    json_t* rootJ;

    StaticPluginLoader(Plugin* const p, const char* const name)
        : plugin(p), file(nullptr), rootJ(nullptr)
    {
        p->path = asset::pluginPath(name);
        const std::string manifestFilename = asset::pluginManifest(name);
        if ((file = std::fopen(manifestFilename.c_str(), "r")) == nullptr) {
            fprintf(stderr, "cardinal: manifest %s not found\n", manifestFilename.c_str());
            return;
        }
        json_error_t error;
        if ((rootJ = json_loadf(file, 0, &error)) == nullptr) {
            fprintf(stderr, "cardinal: JSON error at %s %d:%d %s\n",
                    manifestFilename.c_str(), error.line, error.column, error.text);
            return;
        }
        json_t* const versionJ = json_string((APP_VERSION_MAJOR + ".0").c_str());
        json_object_set(rootJ, "version", versionJ);
        json_decref(versionJ);
        p->fromJson(rootJ);
    }

    ~StaticPluginLoader() {
        if (rootJ != nullptr) {
            json_t* const modulesJ = json_object_get(rootJ, "modules");
            plugin->modulesFromJson(modulesJ);
            json_decref(rootJ);
            plugins.push_back(plugin);
        }
        if (file != nullptr)
            std::fclose(file);
    }

    bool ok() const noexcept { return rootJ != nullptr; }
};

// Safe addModel that skips nulls
static void safeAddModel(Plugin* p, Model* m) {
    if (m) p->addModel(m);
}

// ── Fundamental ──────────────────────────────────────────────────────

#include "Fundamental/src/plugin.hpp"

// pluginInstance is defined in Fundamental/src/plugin.cpp
// but we don't call its init() because it tries to add modelUnity
// which doesn't exist in this submodule version.

static void initStatic__Fundamental() {
    Plugin* const p = new Plugin;
    pluginInstance = p;

    const StaticPluginLoader spl(p, "Fundamental");
    if (spl.ok()) {
        safeAddModel(p, modelVCO);
        safeAddModel(p, modelVCO2);
        safeAddModel(p, modelVCF);
        safeAddModel(p, modelVCA_1);
        safeAddModel(p, modelVCA);
        safeAddModel(p, modelLFO);
        safeAddModel(p, modelLFO2);
        safeAddModel(p, modelDelay);
        safeAddModel(p, modelADSR);
        safeAddModel(p, modelMixer);
        safeAddModel(p, modelVCMixer);
        safeAddModel(p, model_8vert);
        safeAddModel(p, modelMutes);
        safeAddModel(p, modelPulses);
        safeAddModel(p, modelScope);
        safeAddModel(p, modelSEQ3);
        safeAddModel(p, modelSequentialSwitch1);
        safeAddModel(p, modelSequentialSwitch2);
        safeAddModel(p, modelOctave);
        safeAddModel(p, modelQuantizer);
        safeAddModel(p, modelSplit);
        safeAddModel(p, modelMerge);
        safeAddModel(p, modelSum);
        // modelViz not available in this Fundamental revision
        safeAddModel(p, modelMidSide);
        safeAddModel(p, modelNoise);
        safeAddModel(p, modelRandom);
    }
}

// ── Master init ─────────────────────────────────────────────────────

namespace rack {
namespace plugin {

void initStaticPlugins() {
    initStatic__Fundamental();
}

}
}
