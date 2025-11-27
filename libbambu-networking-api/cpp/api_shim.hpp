#pragma once

// This header declares shim functions that use types from rust/cxx.h
// It's included both from cxx-generated code and from api_shim.cpp

#include <cstdint>
#include "rust/cxx.h"

// Shim functions for testing against closed-source library
void bambu_network_init_c();
int32_t bambu_network_connect_c(rust::Str dev_id, rust::Str ip, rust::Str password, bool ssl);
int32_t bambu_network_disconnect_c(rust::Str dev_id);
int32_t bambu_network_send_c(rust::Str dev_id, rust::Str data);
