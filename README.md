# OpenBambuNet �

**OpenBambuNet** is a free and open-source (FOSS) alternative to the proprietary `libbambu_networking.so` library used by Bambu Studio. It allows for direct LAN-mode communication with Bambu Lab printers without relying on Bambu Lab's cloud services or closed-source plugins.

> [!NOTE]
> This project is a fork of the original source, aiming to provide a community-driven, drop-in replacement library.

## Overview

Bambu Studio normally loads a proprietary plugin (`libbambu_networking.so`) to handle network communication with printers. OpenBambuNet provides a compatible shared library that Bambu Studio can load instead.

By using OpenBambuNet, you gain:
- **Transparency**: Know exactly what data is being sent to your printer.
- **Control**: Operate entirely in LAN mode with no external dependencies.
- **Freedom**: A truly open stack for your 3D printing workflow.

## How it Works

The library is written in Rust and exposes a C++ ABI compatible with the interface Bambu Studio expects. It utilizes `paho-mqtt` for command/control and FTP for file transfers, communicating directly with the printer's local interfaces.

## Installation

To use OpenBambuNet, you need to build the shared library and replace the proprietary plugin in your Bambu Studio installation.

### Prerequisites
- Rust (stable)
- C++ compiler (GCC/Clang)
- CMake
- OpenSSL development headers

### Building

```bash
cargo build --release
```

### Installing

You can symlink the built library to your Bambu Studio plugins directory.

**macOS Example:**
```bash
ln -sf $(pwd)/target/release/libbambu_networking.dylib ~/Library/Application\ Support/BambuStudio/plugins/libbambu_networking.dylib
```

**Linux Example:**
```bash
ln -sf $(pwd)/target/release/libbambu_networking.so ~/.config/BambuStudio/plugins/libbambu_networking.so
```

> [!WARNING]
> Bambu Studio may attempt to overwrite this plugin during updates. You may need to reinstall the symlink after updating Bambu Studio.

## Development Status

The project is currently in active development.
- **Implemented**: Basic printer status monitoring, axis control, print job start.
- **Missing**: Camera feed, advanced authentication handling, robust error handling.

## Testing

This repository includes a `mock-printer` to simulate a Bambu Lab printer for integration testing.

```bash
cd mock-printer
npm install
npm start
```

See `mock-printer/README.md` for more details.

## License

This project is licensed under the **AGPLv3**.
