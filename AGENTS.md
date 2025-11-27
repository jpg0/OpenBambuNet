# OpenBambuNet - Agent Documentation

This document provides architectural guidance for AI agents working on the OpenBambuNet codebase.

## Project Architecture

The OpenBambuNet project consists of three main components:

### 1. `open-bambu-core/` - Core Library
A Rust library providing MQTT and FTP client functionality for Bambu Lab 3D printers.

**Key modules:**
- `mqtt.rs`: MQTT client implementation using paho-mqtt
- `ftp.rs`: FTP client for file uploads
- `printer.rs`: Printer configuration and discovery

### 2. `libbambu-networking-api/` - Drop-In Replacement Library
A Rust + C++ hybrid dynamic library that serves as a **drop-in replacement** for Bambu Lab's closed-source `libbambu_networking.dylib`. This allows users to swap the closed-source library with our open-source version without recompiling applications.

**Core Concept:** Runtime library swapping via `DYLD_LIBRARY_PATH`:
```bash
# Use our open-source library
DYLD_LIBRARY_PATH=libbambu-networking-api/target/debug ./bambu_studio

# Use closed-source library  
DYLD_LIBRARY_PATH=closed_lib ./bambu_studio
```

**Architecture:**
```
Application (Bambu Studio)
      ↓ (links against "libbambu_networking.dylib" by name)
libbambu_networking.dylib ← swapped via DYLD_LIBRARY_PATH
      ↓
C++ API Layer (api.cpp)
  - EXPORT bambu_network_connect_printer()
  - EXPORT bambu_network_send_message_to_printer()
  - etc.
      ↓ (calls viaCXX bridge)
Rust Implementation (api.rs)
  - bambu_network_rs_connect()
  - bambu_network_rs_send()
  - etc.
      ↓
open-bambu-core Library
  - MQTT client
  - FTP client
```

**Build Output:**
- `target/debug/libbambu_networking.dylib` - Our open-source replacement (12MB)
- Compatible with closed `libbambu_networking.dylib` from `../closed_lib`

**Critical files:**
- `Cargo.toml`: Defines `crate-type = ["cdylib"]` to build shared library
- `cpp/api.hpp`: C++ header defining the library API (must match closed lib)
- `cpp/api.cpp`: C++ implementation that calls Rust via CXX bridge  
- `src/api.rs`: Rust implementation exposed to C++ via `#[cxx::bridge]`
- `build.rs`: Compiles C++ and links everything into the cdylib

### 3. `mock-printer/` - Test Server
A mock Bambu Lab printer server for integration testing (not currently used in CI).

## ABI Compatibility

### The Challenge
The closed-source `libbambu_networking.dylib` uses a non-standard ABI pattern: `extern "C"` linkage with `std::string` return types. This technically violates C linkage rules but works in practice when using the same compiler and standard library.

### Our Solution
We maintain ABI compatibility through a multi-layered approach:

#### Layer 1: C++ API (`api.hpp`, `api.cpp`)
Matches the closed library's ABI exactly:
- Uses `EXPORT` macro: `extern "C" __attribute__((visibility("default")))`
- Functions return `std::string` by value (non-standard but necessary)
- Accepts `std::string` parameters

#### Layer 2: C-Compatible Shim (`api_shim.cpp`)
Provides C-compatible wrappers for testing:
```cpp
#ifdef USE_CLOSED_LIB
// Wrapper for closed library
extern std::string bambu_network_get_version();
extern "C" const char* bambu_network_get_version_c() {
    static std::string version = bambu_network_get_version();
    return version.c_str();
}
#else
// Direct implementation
extern "C" const char* bambu_network_get_version_c() {
    return BAMBU_NETWORK_AGENT_VERSION;
}
#endif
```

#### Layer 3: Safe Rust Wrappers (`ffi_wrapper.rs`)
Typesafe Rust bindings:
```rust
extern "C" {
    fn bambu_network_get_version_c() -> *const c_char;
}

pub fn get_version() -> Result<String, FfiError> {
    unsafe {
        let ptr = bambu_network_get_version_c();
        if ptr.is_null() {
            return Err(FfiError::NullPointer);
        }
        Ok(CStr::from_ptr(ptr).to_string_lossy().to_string())
    }
}
```

## Testing ABI Compatibility

### Running Tests

**Test against our implementation:**
```bash
cd libbambu-networking-api
cargo test --lib
```

**Test against closed-source library:**
```bash
cd libbambu-networking-api
DYLD_LIBRARY_PATH=../closed_lib USE_CLOSED_LIB=1 cargo test --lib
```

### How It Works

1. **Build Configuration** (`build.rs`):
   - If `USE_CLOSED_LIB=1`: Links against `libbambu_networking.dylib`, compiles shim with `-DUSE_CLOSED_LIB`
   - Otherwise: Compiles our `api.cpp` implementation

2. **Shim Layer** (`api_shim.cpp`):
   - Conditionally calls either the closed library or our implementation
   - Returns C-compatible `const char*` instead of `std::string`

3. **Safe Wrappers** (`ffi_wrapper.rs`):
   - Declares raw `extern "C"` bindings to `*_c()` functions
   - Provides safe Rust functions with error handling
   - Validates pointers and UTF-8 encoding

4. **Tests**:
   - Written once in `ffi_wrapper.rs`
   - Run against both implementations using identical code
   - Proves ABI compatibility

### Verification Process

Symbol comparison:
```bash
# Our library
nm -gU libbambu-networking-api/target/debug/build/.../libbambu_net_api.a | grep bambu_network

# Closed library
nm -gU closed_lib/libbambu_networking.dylib | grep bambu_network
```

Both should export symbols with C linkage (e.g., `_bambu_network_get_version`).

## Development Workflow

### Adding New API Functions

1. **Add to `api.hpp`:**
   ```cpp
   EXPORT std::string bambu_network_new_function(void *agent);
   ```

2. **Implement in `api.cpp`:**
   ```cpp
   std::string bambu_network_new_function(void *agent) {
       LOG_CALL();
       return "result";
   }
   ```

3. **Add C-compatible wrapper to `api_shim.cpp`:**
   ```cpp
   #ifdef USE_CLOSED_LIB
   extern std::string bambu_network_new_function(void *agent);
   extern "C" const char* bambu_network_new_function_c(void *agent) {
       static std::string result = bambu_network_new_function(agent);
       return result.c_str();
   }
   #else
   extern "C" const char* bambu_network_new_function_c(void *agent) {
       return "result";
   }
   #endif
   ```

4. **Add safe Rust wrapper to `ffi_wrapper.rs`:**
   ```rust
   extern "C" {
       fn bambu_network_new_function_c(agent: *mut c_void) -> *const c_char;
   }
   
   pub fn new_function(agent: *mut c_void) -> Result<String, FfiError> {
       unsafe {
           let ptr = bambu_network_new_function_c(agent);
           if ptr.is_null() {
               return Err(FfiError::NullPointer);
           }
           Ok(CStr::from_ptr(ptr).to_string_lossy().to_string())
       }
   }
   ```

5. **Add tests:**
   ```rust
   #[test]
   fn test_new_function() {
       let result = new_function(std::ptr::null_mut()).unwrap();
       assert!(!result.is_empty());
   }
   ```

6. **Verify against both implementations:**
   ```bash
   cargo test --lib
   DYLD_LIBRARY_PATH=../closed_lib USE_CLOSED_LIB=1 cargo test --lib
   ```

### Build System Notes

- The `cxx` crate automatically generates C++ bridge code in `target/debug/build/.../cxxbridge/`
- Compiler warnings about `extern "C"` with `std::string` are expected and safe to ignore
- The closed library is NOT checked into git (add to `.gitignore`)
- Static linking is used for our implementation; dynamic linking for the closed library

### Debugging Tips

1. **Symbol mismatches**: Use `nm -gU` to compare exported symbols
2. **Segfaults**: Check pointer validity and string lifetime in shim layer
3. **Link errors**: Verify `build.rs` configuration and `USE_CLOSED_LIB` flag
4. **UTF-8 errors**: Validate string encoding in safe wrappers

## Key Constraints

1. **Cannot modify closed library**: Must match its ABI exactly
2. **Must use `extern "C"` with `std::string`**: Non-standard but required for compatibility
3. **Same compiler required**: Closed library requires clang/libc++ (macOS default)
4. **Tests must be identical**: Same test code runs against both implementations

## Future Work

- Add more wrapper functions as needed for Bambu Studio integration
- Implement remaining functions from closed library (see walkthrough.md for missing symbols)
- Add integration tests with mock-printer
- Consider CI/CD pipeline for automated ABI verification
