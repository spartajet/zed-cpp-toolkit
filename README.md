# Zed C++ Toolkit

`cpp-toolkit` 是一个面向 Zed 的跨平台 C/C++ 辅助扩展。它通过项目内配置文件 `.zed/cpp-toolkit.toml` 选择 toolchain、构建命令、运行命令、任务生成和 `clangd` 参数，不再只绑定 Windows/MSVC。

## 核心能力

- 配置优先：使用 `.zed/cpp-toolkit.toml` 描述项目行为。
- Preset 辅助：内置常见 CMake/Ninja、Make、GCC、Clang、MSVC 工作流。
- 命令模板：构建命令支持 `{build_dir}` 和 `{build_type}`。
- 始终生成 `.clangd`：为 Zed 的 C/C++ 语言服务提供稳定配置。
- 生成 `.zed/tasks.json`：提供 Configure、Build、Clean、Run 任务。
- 保留 MSVC 支持：`msvc-cmake-ninja` 会尽量发现 Visual Studio/MSVC/Windows SDK include 路径。
- CMake LSP：继续集成 `neocmakelsp` 支持 `CMakeLists.txt`。

## Preset

| Preset | 适用场景 |
| --- | --- |
| `msvc-cmake-ninja` | Windows + MSVC + CMake + Ninja |
| `gcc-cmake-ninja` | GCC + CMake + Ninja |
| `clang-cmake-ninja` | Clang + CMake + Ninja |
| `gcc-make` | GCC + Makefile |
| `clang-make` | Clang + Makefile |
| `custom` | 完全自定义命令 |

未提供配置文件时，扩展会按平台选择默认 preset：Windows 使用 `msvc-cmake-ninja`，macOS 使用 `clang-cmake-ninja`，其他平台使用 `gcc-cmake-ninja`。

## 快速开始

在项目根目录创建 `.zed/cpp-toolkit.toml`：

```toml
preset = "gcc-cmake-ninja"

[build]
build_type = "Debug"
build_dir = "build"

[run]
command = "./build/app"

[clangd]
extra_flags = ["-std=c++20"]
```

打开 C/C++ 文件后，扩展会解析配置、合并 preset、生成或刷新自动生成的 `.clangd`、生成 `.zed/tasks.json`，并启动 `cpp-toolkit-clangd`。

## 配置示例

### CMake + Ninja

```toml
preset = "clang-cmake-ninja"

[build]
build_type = "Release"
configure = "cmake -S . -B {build_dir} -G Ninja -DCMAKE_BUILD_TYPE={build_type} -DCMAKE_EXPORT_COMPILE_COMMANDS=ON"
build = "cmake --build {build_dir}"
clean = "cmake --build {build_dir} --target clean"

[clangd]
compiler = "clang++"
compile_commands_dir = "{build_dir}"
query_driver = ["clang", "clang++"]
```

### Makefile

```toml
preset = "gcc-make"

[build]
build = "make -j"
clean = "make clean"

[run]
command = "./app"

[clangd]
compiler = "g++"
compile_commands_dir = "."
query_driver = ["gcc", "g++"]
```

### 完全自定义

```toml
preset = "custom"

[toolchain]
name = "my-toolchain"
cc = "cc"
cxx = "c++"

[build]
system = "custom"
build_dir = "out"
build_type = "Debug"
build = "python build.py --out {build_dir} --type {build_type}"
clean = "python build.py clean"

[run]
command = "./out/app"
cwd = "$ZED_WORKTREE_ROOT"

[clangd]
command = "clangd"
compiler = "c++"
compile_commands_dir = "out"
extra_flags = ["-std=c++20", "-Iinclude"]
```

## Build 目录风格

默认构建目录是 `build`。只有显式配置时才使用 CLion 风格：

```toml
[build]
build_dir_style = "clion"
build_type = "Debug"
```

此时 `Debug` 会解析为 `cmake-build-debug`，`Release` 会解析为 `cmake-build-release`。如果设置了 `build_dir`，它总是优先于 `build_dir_style`。

## `.clangd` 行为

扩展会保留用户手写的 `.clangd`。如果 `.clangd` 是旧版 `Zed MSVC C++ Assistant` 或新版 `Zed C++ Toolkit` 自动生成的，扩展会根据当前有效配置刷新它。

常用 `clangd` 配置：

```toml
[clangd]
command = "clangd"
compiler = "g++"
compile_commands_dir = "{build_dir}"
extra_flags = ["-std=c++20", "-Iinclude"]
query_driver = ["gcc", "g++"]
```

`query_driver` 会传给 `clangd` 的 `--query-driver=...`。

## 生成的任务

扩展根据配置生成 `.zed/tasks.json`，任务标签包括：

- `C++: Configure`
- `C++: Build`
- `C++: Clean`
- `C++: Run`

## CMake 语言支持

扩展继续提供 `neocmakelsp`。可在 `.zed/settings.json` 中配置：

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

## 构建扩展

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
```

## 需求

- Zed
- `clangd`
- 与所选 preset 对应的编译器和构建工具，例如 `cmake`、`ninja`、`make`、`gcc`、`clang` 或 Visual Studio/MSVC。

## License

MIT
