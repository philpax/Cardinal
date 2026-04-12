// Auto-generated — registration function for GoodSheperd
// Renames init() only in the included file, not globally
#define init init__GoodSheperd
#include "GoodSheperd/src/plugin.cpp"
#undef init

#include <rack.hpp>
#include <plugin.hpp>

// Asset helpers (same as plugin_init.cpp)
namespace rack { namespace asset {
extern std::string pluginManifest(const std::string& dirname);
extern std::string pluginPath(const std::string& dirname);
} }

struct StaticPluginLoader {
    rack::plugin::Plugin* const plugin;
    FILE* file;
    json_t* rootJ;
    StaticPluginLoader(rack::plugin::Plugin* const p, const char* const name)
        : plugin(p), file(nullptr), rootJ(nullptr)
    {
        p->path = rack::asset::pluginPath(name);
        const std::string mf = rack::asset::pluginManifest(name);
        if ((file = std::fopen(mf.c_str(), "r")) == nullptr) return;
        json_error_t error;
        if ((rootJ = json_loadf(file, 0, &error)) == nullptr) return;
        json_t* const vJ = json_string((rack::APP_VERSION_MAJOR + ".0").c_str());
        json_object_set(rootJ, "version", vJ);
        json_decref(vJ);
        p->fromJson(rootJ);
    }
    ~StaticPluginLoader() {
        if (rootJ) {
            json_t* const mJ = json_object_get(rootJ, "modules");
            plugin->modulesFromJson(mJ);
            json_decref(rootJ);
            rack::plugin::plugins.push_back(plugin);
        }
        if (file) std::fclose(file);
    }
    bool ok() const noexcept { return rootJ != nullptr; }
};

extern "C" void cardinal_register_GoodSheperd() {
    using namespace rack;
    using namespace rack::plugin;
    Plugin* const p = new Plugin;
    pluginInstance__GoodSheperd = p;
    const StaticPluginLoader spl(p, "GoodSheperd");
    if (spl.ok()) init__GoodSheperd(p);
}
