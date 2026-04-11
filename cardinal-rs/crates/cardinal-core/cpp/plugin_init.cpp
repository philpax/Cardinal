/*
 * Asset path helpers needed by plugins.cpp's StaticPluginLoader.
 * These match Cardinal's custom/asset.cpp.
 */

#include <asset.hpp>
#include <system.hpp>

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
