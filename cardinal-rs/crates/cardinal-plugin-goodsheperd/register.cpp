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

extern "C" void cardinal_register_GoodSheperd() {
    using namespace rack;
    using namespace rack::plugin;
    Plugin* const p = new Plugin;
    pluginInstance__GoodSheperd = p;

    // Load manifest
    p->path = rack::asset::pluginPath("GoodSheperd");
    const std::string mf = rack::asset::pluginManifest("GoodSheperd");
    FILE* file = std::fopen(mf.c_str(), "r");
    if (!file) {
        fprintf(stderr, "cardinal: WARNING: manifest not found for GoodSheperd: %s\n", mf.c_str());
        delete p;
        pluginInstance__GoodSheperd = nullptr;
        return;
    }
    json_error_t error;
    json_t* rootJ = json_loadf(file, 0, &error);
    std::fclose(file);
    if (!rootJ) {
        fprintf(stderr, "cardinal: WARNING: failed to parse manifest for GoodSheperd\n");
        delete p;
        pluginInstance__GoodSheperd = nullptr;
        return;
    }

    // Set version and parse plugin metadata
    json_t* const vJ = json_string((rack::APP_VERSION_MAJOR + ".0").c_str());
    json_object_set(rootJ, "version", vJ);
    json_decref(vJ);
    p->fromJson(rootJ);

    // Call the plugin's init to register models via addModel()
    try {
        init__GoodSheperd(p);
    } catch (const std::exception& e) {
        fprintf(stderr, "cardinal: WARNING: init failed for GoodSheperd: %s\n", e.what());
    }

    // Match models to manifest modules (skip missing ones instead of throwing)
    json_t* const mJ = json_object_get(rootJ, "modules");
    if (mJ && json_array_size(mJ) > 0) {
        std::list<Model*> ordered;
        size_t moduleId;
        json_t* moduleJ;
        json_array_foreach(mJ, moduleId, moduleJ) {
            json_t* slugJ = json_object_get(moduleJ, "slug");
            if (!slugJ) continue;
            std::string slug = json_string_value(slugJ);
            auto it = std::find_if(p->models.begin(), p->models.end(),
                [&](Model* m) { return m->slug == slug; });
            if (it == p->models.end()) continue;  // skip missing models
            Model* model = *it;
            p->models.erase(it);
            ordered.push_back(model);
            try { model->fromJson(moduleJ); } catch (...) {}
        }
        // Keep any extra models that weren't in the manifest
        for (auto* m : p->models) ordered.push_back(m);
        p->models = ordered;
    }

    json_decref(rootJ);
    rack::plugin::plugins.push_back(p);
}
