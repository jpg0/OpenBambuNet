#include <dlfcn.h>
#include <iostream>
#include <string>
#include <vector>

// Function pointer types for testing
typedef bool (*CheckDebugConsistentPtr)(bool);
typedef std::string (*GetVersionPtr)();
typedef void* (*CreateAgentPtr)();
typedef int (*DestroyAgentPtr)(void*);
typedef int (*StartAgentPtr)(void*);

struct TestResult {
    std::string function_name;
    bool closed_exists;
    bool open_exists;
    bool signature_match;
    std::string error_message;
};

bool test_function_exists(void* handle, const char* name) {
    dlerror(); // Clear existing errors
    void* sym = dlsym(handle, name);
    return (dlerror() == nullptr);
}

int main() {
    std::cout << "=== Detailed ABI Compatibility Test ===" << std::endl << std::endl;
    
    // Load both libraries
    std::cout << "Loading libraries..." << std::endl;
    void* closed_handle = dlopen("../closed_lib/libbambu_networking.dylib", RTLD_LAZY | RTLD_LOCAL);
    if (!closed_handle) {
        std::cerr << "ERROR: Failed to load closed library: " << dlerror() << std::endl;
        return 1;
    }
    std::cout << "✓ Loaded closed source library" << std::endl;
    
    void* open_handle = dlopen("target/release/libbambu_networking.dylib", RTLD_LAZY | RTLD_LOCAL);
    if (!open_handle) {
        std::cerr << "ERROR: Failed to load open source library: " << dlerror() << std::endl;
        dlclose(closed_handle);
        return 1;
    }
    std::cout << "✓ Loaded open source library" << std::endl << std::endl;
    
    // Test critical functions
    std::vector<TestResult> results;
    
    // Test bambu_network_check_debug_consistent
    {
        TestResult r;
        r.function_name = "bambu_network_check_debug_consistent";
        r.closed_exists = test_function_exists(closed_handle, r.function_name.c_str());
        r.open_exists = test_function_exists(open_handle, r.function_name.c_str());
        r.signature_match = false;
        
        if (r.closed_exists && r.open_exists) {
            CheckDebugConsistentPtr closed_fn = (CheckDebugConsistentPtr)dlsym(closed_handle, r.function_name.c_str());
            CheckDebugConsistentPtr open_fn = (CheckDebugConsistentPtr)dlsym(open_handle, r.function_name.c_str());
            
            if (closed_fn && open_fn) {
                try {
                    bool closed_result = closed_fn(false);
                    bool open_result = open_fn(false);
                    r.signature_match = true;
                    if (closed_result != open_result) {
                        r.error_message = "Different return values";
                    }
                } catch (...) {
                    r.error_message = "Exception during call";
                }
            }
        }
        results.push_back(r);
    }
    
    // Test bambu_network_get_version
    {
        TestResult r;
        r.function_name = "bambu_network_get_version";
        r.closed_exists = test_function_exists(closed_handle, r.function_name.c_str());
        r.open_exists = test_function_exists(open_handle, r.function_name.c_str());
        r.signature_match = false;
        
        if (r.closed_exists && r.open_exists) {
            GetVersionPtr closed_fn = (GetVersionPtr)dlsym(closed_handle, r.function_name.c_str());
            GetVersionPtr open_fn = (GetVersionPtr)dlsym(open_handle, r.function_name.c_str());
            
            if (closed_fn && open_fn) {
                try {
                    std::string closed_version = closed_fn();
                    std::string open_version = open_fn();
                    r.signature_match = true;
                    std::cout << "Version comparison:" << std::endl;
                    std::cout << "  Closed: " << closed_version << std::endl;
                    std::cout << "  Open:   " << open_version << std::endl << std::endl;
                } catch (...) {
                    r.error_message = "Exception during call";
                }
            }
        }
        results.push_back(r);
    }
    
    // Test bambu_network_create_agent
    {
        TestResult r;
        r.function_name = "bambu_network_create_agent";
        r.closed_exists = test_function_exists(closed_handle, r.function_name.c_str());
        r.open_exists = test_function_exists(open_handle, r.function_name.c_str());
        r.signature_match = false;
        
        if (r.closed_exists && r.open_exists) {
            CreateAgentPtr closed_fn = (CreateAgentPtr)dlsym(closed_handle, r.function_name.c_str());
            CreateAgentPtr open_fn = (CreateAgentPtr)dlsym(open_handle, r.function_name.c_str());
            
            if (closed_fn && open_fn) {
                try {
                    void* closed_agent = closed_fn();
                    void* open_agent = open_fn();
                    r.signature_match = true;
                    if (closed_agent && open_agent) {
                        std::cout << "Agent creation test:" << std::endl;
                        std::cout << "  Closed: " << closed_agent << std::endl;
                        std::cout << "  Open:   " << open_agent << std::endl << std::endl;
                    }
                } catch (...) {
                    r.error_message = "Exception during call";
                }
            }
        }
        results.push_back(r);
    }
    
    // Print results
    std::cout << "=== Test Results ===" << std::endl;
    int passed = 0;
    int failed = 0;
    
    for (const auto& r : results) {
        std::cout << r.function_name << ":" << std::endl;
        std::cout << "  Exists in closed: " << (r.closed_exists ? "✓" : "✗") << std::endl;
        std::cout << "  Exists in open:   " << (r.open_exists ? "✓" : "✗") << std::endl;
        std::cout << "  Signature match:  " << (r.signature_match ? "✓" : "✗");
        if (!r.error_message.empty()) {
            std::cout << " (" << r.error_message << ")";
        }
        std::cout << std::endl << std::endl;
        
        if (r.closed_exists && r.open_exists && r.signature_match) {
            passed++;
        } else {
            failed++;
        }
    }
    
    dlclose(closed_handle);
    dlclose(open_handle);
    
    std::cout << "=== Summary ===" << std::endl;
    std::cout << "Passed: " << passed << std::endl;
    std::cout << "Failed: " << failed << std::endl;
    
    return (failed == 0) ? 0 : 1;
}
