# OpenBambuNet

## What is Bambu Farm?
# OpenBambuNet Overview
OpenBambuNet is an open‑source library that replaces the proprietary networking plugin (`libbambu_networking.so`) used by **Bambu Studio**. It enables LAN‑mode printers to communicate directly with Bambu Studio, allowing:
- Remote control of multiple printers
- Fine‑grained access control
- Secure communication via MQTT and FTP

## High‑Level Architecture
```
+-------------------+        +-------------------+        +-------------------+
| Bambu Studio UI  | <---> |  libbambu_networking.so (Rust)  | <---> | Bambu Printer (MQTT/FTP) |
+-------------------+        +-------------------+        +-------------------+
```
- **Bambu Studio UI** loads the plugin (`libbambu_networking.so`).
- The plugin is a Rust wrapper that implements the C-API expected by Bambu Studio.
- The plugin directly communicates with printers using `paho-mqtt` for commands/status and `rust-ftp` for file uploads.

## Repository Layout
| Path | Description |
|------|-------------|
| `bambu-farm/` | Core Rust library containing printer management, MQTT, and FTP logic. |
| `bambu-farm-client/` | Rust library compiled into `libbambu_networking.so`. Contains the C++ bridge (`cpp/`) and FFI exposed to Bambu Studio. |
| `bambu-farm-client/cpp/` | C++ header (`api.hpp`) and implementation (`api.cpp`) that expose the C‑API used by Bambu Studio. |
| `bambu-farm-client/src/` | Core Rust code (`api.rs`, `lib.rs`). Handles initialization and calls into `bambu-farm` library. |
| `README.md` | High‑level user‑facing documentation. |
| `LICENSE.md` | AGPL‑v3 license. |
| `Makefile` | Convenience targets to build the client plugin and install it into `~/.config/BambuStudio/plugins/`. |

## Key Modules (bambu-farm)
- **`lib.rs`** – Public API re-exports.
- **`printer.rs`** – Printer configuration loading and management.
- **`mqtt.rs`** – MQTT client wrapper using `paho-mqtt`.
- **`ftp.rs`** – FTP client wrapper using `rust-ftp`.

## Key Modules (bambu-farm-client)
- **`api.rs`** – Orchestrates the async runtime, maintains global state (`PRINTERS`, `CONNECTIONS`), and implements the public FFI functions.
- **`ffi` bridge** – `cxx::bridge` definition exposing Rust functions to C++ and vice‑versa.

## Build & Run Quick Start
```bash
# Build client plugin (produces libbambu_networking.so)
cd bambu-farm-client
make
# Install plugin for Bambu Studio (creates symlink in $HOME/.config/BambuStudio/plugins/)
make install
```
The client reads printer configuration from `bambufarm.toml` in the current working directory (usually where Bambu Studio is launched from).

## Contributing
- Follow the existing Rust coding style (`cargo fmt`, `cargo clippy`).
- Keep the C++ bridge thin – most logic should stay in Rust.
- When adding features, update this overview accordingly.
