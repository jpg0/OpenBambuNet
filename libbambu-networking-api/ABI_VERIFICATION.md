# ABI Compatibility Verification

## Overview

This directory contains tools to verify ABI compatibility between the open-source `libbambu_networking` implementation and the closed-source reference library from Bambu Lab.

## Quick Start

```bash
# Build and run ABI verification
make verify-abi
```

## Tools

### 1. `verify_abi.sh`
Shell script that:
- Extracts all exported `bambu_*` symbols from both libraries
- Compares symbols using the closed source library as the source of truth
- Reports missing symbols (must be implemented)
- Reports extra symbols (informational)
- Runs detailed C++ signature tests

### 2. `test_abi.cpp`
C++ program that:
- Dynamically loads both libraries using `dlopen`
- Tests critical functions can be called with correct signatures
- Verifies ABI compatibility at runtime
- Tests for C++ ABI issues (std::string, calling conventions, etc.)

### 3. `Makefile`
Build targets:
- `make verify-abi` - Run full ABI verification suite
- `make test_abi` - Compile the C++ test program
- `make clean` - Clean build artifacts

## Current Status

### Library Versions
- **Closed source:** `01.10.01.01`
- **Open source:** `01.09.05.01`

### Symbol Comparison
- **Closed source (reference):** 104 symbols
- **Open source (current):** 101 symbols
- **Missing:** 8 symbols ❌
- **Extra:** 5 symbols ⚠️

### Missing Symbols (Must Implement)

These functions exist in the closed source library but are missing from the open source implementation:

1. `bambu_network_check_user_report` - User reporting functionality
2. `bambu_network_del_rating_picture_oss` - Delete rating pictures from OSS
3. `bambu_network_del_subscribe_internal` - Internal subscription management
4. `bambu_network_get_model_instance_id` - Get model instance identifier
5. `bambu_network_get_model_rating_id` - Get model rating identifier
6. `bambu_network_start_device_subscribe` - Device-specific subscription
7. `bambu_network_stop_device_subscribe` - Stop device subscription
8. `bambulib_get` - Library metadata function

### Extra Symbols (Informational)

These functions exist in the open source library but not in the closed source reference:

1. `bambu_init` - Constructor function (intentional)
2. `bambu_network_del_subscribe` - May be a newer API
3. `bambu_network_get_profile_3mf` - May be a newer API
4. `bambu_network_install_device_cert` - May be a newer API
5. `bambu_network_update_cert` - May be a newer API

**Note:** Extra symbols are generally okay as they don't break compatibility. The closed source library may be from an older version of OrcaSlicer.

## Resolving ABI Issues

### To Pass Verification

You need to implement stub functions for the 8 missing symbols. Add to `cpp/api.cpp`:

```cpp
extern "C" {

int bambu_network_check_user_report(void *agent_ptr) {
    LOG_CALL();
    return 0;
}

int bambu_network_del_rating_picture_oss(void *agent_ptr, std::string picture_id, unsigned int *http_code) {
    LOG_CALL();
    *http_code = 501; // Not implemented
    return 0;
}

// ... implement others similarly
}
```

And add to `exports.txt`:
```
_bambu_network_check_user_report
_bambu_network_del_rating_picture_oss
_bambu_network_del_subscribe_internal
_bambu_network_get_model_instance_id
_bambu_network_get_model_rating_id
_bambu_network_start_device_subscribe
_bambu_network_stop_device_subscribe
_bambulib_get
```

### Known Issues

**`bambu_network_create_agent` Test Failure:**
The detailed ABI test shows `bambu_network_create_agent` throws an exception when called from the closed source library. This is expected because we're not supposed to call it directly - OrcaSlicer handles agent creation through dlsym.

## Understanding the Error

The original error you saw:
```
get_version, get_version not supported,return 00.00.00.00!
```

This error comes from **OrcaSlicer's source code**, not from the library. It means:
1. OrcaSlicer successfully loaded the library
2. OrcaSlicer called `NetworkAgent::get_version()`
3. The NetworkAgent class checks if `get_version_ptr` is set
4. If not set, it returns "00.00.00.00"

This suggests OrcaSlicer might be:
- Finding and loading a different library (check library placement)
- Having issues with dynamic symbol loading
- Expecting a different library version

## Next Steps

1. **Implement missing symbols** to pass symbol verification
2. **Check library placement** - ensure OrcaSlicer loads your library
3. **Verify version compatibility** - closed lib is `01.10.01.01`, yours is `01.09.05.01`
4. **Test with OrcaSlicer** to confirm it loads correctly

## References

- Closed source library: `../closed_lib/libbambu_networking.dylib`
- Open source library: `target/release/libbambu_networking.dylib`
- OrcaSlicer documentation: `../ORCASLICER_OPS.md`
