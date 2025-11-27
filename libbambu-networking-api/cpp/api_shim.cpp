#include "api.hpp"
#include <string>
#include <cstring>
#include <vector>

// Helper to keep strings alive for return
static std::vector<std::string> g_string_cache;
static const char* cache_string(std::string s) {
    g_string_cache.push_back(s);
    return g_string_cache.back().c_str();
}

extern "C" {

// Version wrapper
const char* bambu_network_get_version_c() {
    // The C++ API returns std::string by value
    // We need to cache it to return a valid const char*
    static std::string version = bambu_network_get_version();
    return version.c_str();
}

// Global agent pointer for shim usage
static void* g_agent = nullptr;

void bambu_network_init_c() {
    if (!g_agent) {
        g_agent = bambu_network_create_agent();
        bambu_network_start(g_agent);
    }
}

int32_t bambu_network_connect_c(const char* dev_id, const char* ip, const char* password, bool ssl) {
    if (!g_agent) return -1;
    return bambu_network_connect_printer(g_agent, dev_id, ip, "bblp", password, ssl);
}

int32_t bambu_network_disconnect_c(const char* dev_id) {
    if (!g_agent) return -1;
    return bambu_network_disconnect_printer(g_agent);
}

int32_t bambu_network_send_c(const char* dev_id, const char* data) {
    if (!g_agent) return -1;
    return bambu_network_send_message_to_printer(g_agent, dev_id, data, 0);
}

} // extern "C"
