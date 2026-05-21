# Zed MSVC C++ Assistant - Usage Guide

## Installation

### 1. Build the Extension

```bash
# Add WASM target (Rust must be installed via rustup)
rustup target add wasm32-unknown-unknown

# Build Release version
cargo build --target wasm32-unknown-unknown --release
```

### 2. Install as Dev Extension

Zed supports local development extensions (Dev Extension), no need to publish:

1. Open extension panel in Zed: `Ctrl+Shift+X`
2. Click `Install Dev Extension` button
3. Select project root directory `E:\Rust\zed-msvc-toolkit`
4. Zed will automatically compile and load the extension

**Note**:
- Project must be a Git repository
- Rust must be installed via rustup (not via other methods like homebrew)
- For debug logs, start Zed with `zed --foreground`

### 3. Publish to Extension Marketplace

Publishing extension requires PR to `zed-industries/extensions` repository:

1. Push project to a public GitHub repository
2. Fork `zed-industries/extensions` repository
3. Add your extension as a Git submodule
4. Update `extensions.toml`
5. Submit PR

## System Requirements

- **Operating System**: Windows 11
- **Visual Studio**: 2022 or newer
  - "Desktop development with C++" workload must be installed
- **LLVM**: clangd must be installed and in PATH
- **CMake** (optional): Required for using task system

## Features

### V0.1: MSVC Environment Detection

Extension automatically detects:
- Visual Studio 2022+ installation path (via vswhere.exe)
- MSVC v143+ toolchain
- Windows SDK include directories
- clangd executable

### V0.2: Compile Database Support

Automatically detects `compile_commands.json` in:
- Workspace root directory
- `build/` subdirectory

### V0.4: CMake Task System

Provides CMake operations through Zed task system.

## Configuration

### Language Server

Extension automatically provides clangd language server for C/C++ files. No additional configuration needed.

### CMake Tasks

1. Copy task template to workspace:
   ```
   cp docs/zed-tasks-example.json .zed/tasks.json
   ```

2. Open task panel in Zed: `Ctrl+Shift+T`

3. Select task to run:
   - `CMake: Configure (Debug)` - Configure CMake project
   - `CMake: Build (Debug)` - Build Debug version
   - `CMake: Configure (Release)` - Configure Release version
   - `CMake: Build (Release)` - Build Release version

### Custom Build Directory

If using a different build directory, edit `.zed/tasks.json`:

```json
{
  "label": "CMake: Configure (Debug)",
  "command": "cmake",
  "args": [
    "-S",
    "$ZED_WORKTREE_ROOT",
    "-B",
    "$ZED_WORKTREE_ROOT/cmake-build-debug",  // Modify here
    "-DCMAKE_BUILD_TYPE=Debug"
  ]
}
```

## Troubleshooting

### clangd Cannot Start

**Error**: "clangd not found"

**Solution**:
1. Install LLVM: https://llvm.org/builds/
2. Ensure `clangd.exe` is in PATH
3. Restart Zed

### Visual Studio Not Found

**Error**: "Visual Studio 2022+ not found"

**Solution**:
1. Install Visual Studio 2022
2. Ensure "Desktop development with C++" workload is installed
3. Restart Zed

### MSVC Toolchain Not Found

**Error**: "MSVC v143+ toolset not found"

**Solution**:
1. Open Visual Studio Installer
2. Modify Visual Studio 2022 installation
3. Ensure "MSVC v143 - VS 2022 C++ x64/x86 build tools" is checked

### Windows SDK Not Found

**Error**: Extension generates degraded config, SDK paths commented

**Solution**:
1. Open Visual Studio Installer
2. Modify Visual Studio 2022 installation
3. Ensure "Windows 11 SDK" or "Windows 10 SDK" is checked

## Project Structure

```
zed-msvc-toolkit/
├── src/
│   ├── cmake/
│   │   ├── compile_db.rs    # compile_commands.json detection
│   │   ├── tasks.rs         # Task file generation (V0.4)
│   │   └── tools.rs         # CMake tool detection
│   ├── environment/
│   │   ├── msvc.rs          # MSVC toolchain detection
│   │   ├── vswhere.rs       # vswhere.exe invocation
│   │   ├── windows_sdk.rs   # Windows SDK detection
│   │   └── tools.rs         # Tool lookup helpers
│   ├── lsp/
│   │   ├── clangd_config.rs # .clangd config generation
│   │   └── server.rs        # Language server startup
│   ├── debug/               # Debug support (V0.5 TODO)
│   ├── error.rs             # Error types
│   ├── lib.rs               # Extension entry
│   └── paths.rs             # Path handling
├── docs/
│   ├── USAGE.md             # This document
│   ├── TESTING.md           # Testing guide
│   └── zed-tasks-example.json # Task file example
├── extension.toml           # Extension manifest
├── Cargo.toml               # Rust project config
└── README.md                # Project description
```

## Version History

- **V0.1** (2026-05-20): MSVC environment detection and clangd configuration
- **V0.2** (2026-05-20): Compile database support
- **V0.3** (2026-05-20): CMake command generation infrastructure
- **V0.4** (2026-05-20): Task system integration
- **V0.5** (Planned): DAP debugging support
- **V1.0** (Planned): Zero-config debugging

## License

MIT License
