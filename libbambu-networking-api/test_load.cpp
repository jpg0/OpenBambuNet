#include <dlfcn.h>
#include <iostream>
#include <string>

// Function pointer types matching the library
typedef bool (*CheckDebugConsistentPtr)(bool);
typedef std::string (*GetVersionPtr)();
typedef void* (*CreateAgentPtr)();

int main() {
    std::cout << "Loading library..." << std::endl;
    
    // Load the library
    void* handle = dlopen("target/release/libbambu_networking.dylib", RTLD_LAZY | RTLD_LOCAL);
    if (!handle) {
        std::cerr << "Failed to load library: " << dlerror() << std::endl;
        return 1;
    }
    std::cout << "Library loaded successfully!" << std::endl;
    
    // Try to load bambu_network_check_debug_consistent
    dlerror(); // Clear any existing error
    CheckDebugConsistentPtr check_debug_fn = (CheckDebugConsistentPtr)dlsym(handle, "bambu_network_check_debug_consistent");
    const char* dlsym_error = dlerror();
    if (dlsym_error) {
        std::cerr << "Failed to load bambu_network_check_debug_consistent: " << dlsym_error << std::endl;
    } else {
        std::cout << "✓ bambu_network_check_debug_consistent loaded" << std::endl;
        bool result = check_debug_fn(false);
        std::cout << "  Result: " << (result ? "true" : "false") << std::endl;
    }
    
    // Try to load bambu_network_get_version
    dlerror(); // Clear any existing error
    GetVersionPtr get_version_fn = (GetVersionPtr)dlsym(handle, "bambu_network_get_version");
    dlsym_error = dlerror();
    if (dlsym_error) {
        std::cerr << "Failed to load bambu_network_get_version: " << dlsym_error << std::endl;
    } else {
        std::cout << "✓ bambu_network_get_version loaded" << std::endl;
        std::string version = get_version_fn();
        std::cout << "  Version: " << version << std::endl;
    }
    
    // Try to load bambu_network_create_agent
    dlerror(); // Clear any existing error
    CreateAgentPtr create_agent_fn = (CreateAgentPtr)dlsym(handle, "bambu_network_create_agent");
    dlsym_error = dlerror();
    if (dlsym_error) {
        std::cerr << "Failed to load bambu_network_create_agent: " << dlsym_error << std::endl;
    } else {
        std::cout << "✓ bambu_network_create_agent loaded" << std::endl;
        void* agent = create_agent_fn();
        std::cout << "  Agent: " << agent << std::endl;
    }
    
    dlclose(handle);
    std::cout << "\nAll function pointers loaded successfully!" << std::endl;
    return 0;
}
