# Zed MSVC C++ Assistant

MSVC and clangd assistant for Windows C++ CMake projects in Zed.

## Version 0.4.0

### Features

- **V0.1**: MSVC toolchain detection (vswhere.exe, MSVC v143+, Windows SDK)
- **V0.2**: CMake `compile_commands.json` auto-detection
- **V0.3**: CMake command generation infrastructure
- **V0.4**: `.zed/tasks.json` generation for CMake operations
- **V0.5**: neocmakelsp integration for CMake language support

## Documentation

- **[使用说明 (USAGE.md)](docs/USAGE.md)** - 安装、配置和使用指南
- **[测试指南 (TESTING.md)](docs/TESTING.md)** - 单元测试和集成测试说明

## Quick Start

### Installation

```bash
# 编译
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release

# 安装到 Zed
mkdir -p "$USERPROFILE/.zed/extensions/zed-msvc-toolkit"
cp target/wasm32-unknown-unknown/release/zed_msvc_toolkit.wasm "$USERPROFILE/.zed/extensions/zed-msvc-toolkit/"
cp extension.toml "$USERPROFILE/.zed/extensions/zed-msvc-toolkit/"
```

### CMake Tasks

Copy the task template to your workspace:

```bash
cp docs/zed-tasks-example.json .zed/tasks.json
```

Then run tasks via `Ctrl+Shift+T` (Task: Run).

### CMake Language Support

The extension includes [neocmakelsp](https://github.com/neocmakelsp/neocmakelsp) for CMake language support (`CMakeLists.txt` files).

**Installation:**
- If `neocmakelsp` is in your PATH, it will be used directly.
- Otherwise, the extension will automatically download it from GitHub Releases to `%LOCALAPPDATA%\zed-msvc-toolkit\neocmakelsp\`.

**Configuration:**

neocmakelsp can be configured via two sources (settings.json overrides .neocmake.toml):

1. **Project-level** (`.neocmake.toml` in project root):
   ```toml
   [format]
   enable = true

   [lint]
   enable = true

   scan_cmake_in_package = true
   semantic_token = false
   ```

2. **Workspace-level** (`.zed/settings.json`):
   ```json
   {
     "lsp": {
       "msvc-cmake-neocmake": {
         "format": { "enable": false },
         "lint": { "enable": true }
       }
     }
   }
   ```

## Requirements

- Windows 11
- Visual Studio 2022+ with "Desktop development with C++" workload
- clangd (from LLVM) in PATH
- CMake (optional, for tasks) in PATH
- CMake project with `CMakeLists.txt`

## License

MIT
