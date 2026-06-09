# 跨平台 C++ Toolkit 设计

日期：2026-06-03

## 目标

将当前面向 Windows/MSVC 的 Zed 扩展改造成跨平台的 `cpp-toolkit` 扩展。新的扩展应以项目配置和预设为核心，让用户可以指定编译器、make/build 工具、构建命令、运行命令以及 clangd 行为。

由于当前用户较少，本设计不考虑兼容旧的 `msvc-cpp-toolkit` 标识。相比保留兼容层，直接清理命名和架构会更利于后续维护。

## 核心原则

1. 配置优先，预设辅助，自动检测兜底。
2. 用户在配置文件中编写完整的 configure/build/clean/run 命令字符串。
3. 所有受支持的项目都必须生成 `.clangd` 文件，因为 Zed 的 C/C++ 支持依赖它。
4. MSVC 继续支持，但它只是一个内置 toolchain preset，不再是整个扩展的中心假设。
5. 第一版实现要保持小而实用：支持 CMake、Make、Ninja 风格命令字符串、自定义命令，以及少量常用预设。

## 配置文件

主要项目配置文件为：

```text
.zed/cpp-toolkit.toml
```

配置文件放在 `.zed/` 目录下，这样它与生成的 `.zed/tasks.json` 和 Zed 项目行为保持一致，不会污染项目根目录。

CMake/GCC 示例：

```toml
preset = "gcc-cmake-ninja"

[toolchain]
cc = "gcc"
cxx = "g++"

[build]
system = "cmake"
build_dir_style = "build"
build_type = "Debug"
configure = "cmake -S . -B {build_dir} -G Ninja -DCMAKE_BUILD_TYPE={build_type} -DCMAKE_EXPORT_COMPILE_COMMANDS=ON"
build = "cmake --build {build_dir}"
clean = "cmake --build {build_dir} --target clean"

[run]
command = "./build/app"
cwd = "$ZED_WORKTREE_ROOT"

[clangd]
command = "clangd"
compiler = "g++"
compile_commands_dir = "build"
extra_flags = ["-std=c++20"]
query_driver = ["gcc", "g++"]
```

Makefile 示例：

```toml
preset = "gcc-make"

[build]
build = "make -j16"
clean = "make clean"

[run]
command = "./app"

[clangd]
extra_flags = ["-Iinclude", "-std=c++23"]
```

## 配置合并规则

最终生效配置按以下顺序生成：

1. 从所选 preset 开始。
2. 使用 `.zed/cpp-toolkit.toml` 中的字段覆盖 preset。
3. 对未解析字段填充平台默认值。
4. 仅对仍然缺失的字段执行有限自动检测。

用户提供的命令字符串具有最高优先级。如果用户设置了 `[build].build`，扩展不能替用户重写或重新解释它，只能在生成 Zed task 时做必要的 shell 包装。

如果没有配置文件，则根据平台选择默认 preset：

| 平台 | 默认 preset |
| --- | --- |
| Windows | `msvc-cmake-ninja` |
| Linux | `gcc-cmake-ninja` |
| macOS | `clang-cmake-ninja` |

## 预设

第一版只内置以下 preset：

| Preset | 平台 | 用途 |
| --- | --- | --- |
| `msvc-cmake-ninja` | Windows | MSVC + CMake + Ninja；承接当前扩展能力 |
| `gcc-cmake-ninja` | Linux、Windows MinGW | GCC + CMake + Ninja |
| `clang-cmake-ninja` | Linux、macOS、Windows | Clang + CMake + Ninja |
| `gcc-make` | Linux、Windows MinGW | GCC + Makefile 项目 |
| `clang-make` | Linux、macOS | Clang + Makefile 项目 |
| `custom` | 全平台 | 用户完全自定义命令和 clangd 设置 |

Preset 只提供默认值。用户可以覆盖任意字段。

## 构建目录命名风格

CMake preset 默认使用简单的 `build` 目录。只有用户或 preset 显式设置 `build_dir_style = "clion"` 时，才使用 `cmake-build-debug` 这类 CLion 风格目录。

`[build]` 支持以下字段：

```toml
[build]
build_dir_style = "build"
build_dir = ""
build_type = "Debug"
```

支持的 `build_dir_style`：

| 值 | Debug 结果 | Release 结果 | 说明 |
| --- | --- | --- | --- |
| `build` | `build` | `build` | 默认风格，最简单，跨平台一致 |
| `clion` | `cmake-build-debug` | `cmake-build-release` | 只有显式设置时启用 |
| `custom` | 使用 `build_dir` | 使用 `build_dir` | 用户完全指定目录 |

构建目录解析优先级：

1. 如果用户显式设置了 `build_dir`，使用 `build_dir`。
2. 否则根据 `build_dir_style` 推导。
3. 如果没有设置 `build_dir_style`，默认使用 `build`。

示例：

```toml
preset = "gcc-cmake-ninja"

[build]
build_dir_style = "clion"
build_type = "Debug"
```

最终推导：

```text
build_dir = "cmake-build-debug"
```

用户命令字符串和 preset 内部命令都可以使用模板变量：

```toml
[build]
configure = "cmake -S . -B {build_dir} -G Ninja -DCMAKE_BUILD_TYPE={build_type}"
build = "cmake --build {build_dir}"
clean = "cmake --build {build_dir} --target clean"
```

第一版只需要支持以下模板变量：

- `{build_dir}`：解析后的构建目录。
- `{build_type}`：当前构建类型，例如 `Debug` 或 `Release`。

## `.clangd` 生成规则

扩展必须为每个受支持项目生成 `.clangd`。

通用 GCC/Clang 输出示例：

```yaml
CompileFlags:
  CompilationDatabase: build
  Compiler: g++
  Add:
    - -std=c++20
```

MSVC 输出示例：

```yaml
CompileFlags:
  CompilationDatabase: build
  Compiler: clang-cl
  Add:
    - /std:c++20
    - -isystem
    - C:/path/to/msvc/include
    - -isystem
    - C:/path/to/windows/sdk/include
```

`query_driver` 应作为 clangd 启动参数传入，而不是写入 `.clangd`：

```text
clangd --query-driver=gcc,g++
```

第一版中，GCC/Clang 的 include 处理保持简单，主要依赖 `compile_commands.json` 和用户提供的 `extra_flags`。MSVC 继续复用现有 Visual Studio 和 Windows SDK 检测逻辑，因为 clangd 通常需要这些 include 目录才能正确分析 MSVC 项目。

## Zed task 生成规则

扩展根据 `[build]` 和 `[run]` 中的命令字符串生成 `.zed/tasks.json`。

支持的任务标签：

- 如果设置了 `[build].configure`，生成 `C++: Configure`
- 如果设置了 `[build].build`，生成 `C++: Build`
- 如果设置了 `[build].clean`，生成 `C++: Clean`
- 如果设置了 `[run].command`，生成 `C++: Run`

Linux/macOS shell 包装示例：

```json
{
  "label": "C++: Build",
  "command": "sh",
  "args": ["-lc", "cmake --build build"],
  "cwd": "$ZED_WORKTREE_ROOT"
}
```

Windows shell 包装示例：

```json
{
  "label": "C++: Build",
  "command": "powershell",
  "args": ["-NoProfile", "-Command", "cmake --build build"],
  "cwd": "$ZED_WORKTREE_ROOT"
}
```

MSVC task 可能需要额外的开发者环境包装：先运行检测到的 Visual Studio developer command 脚本，再执行用户命令。这个包装逻辑应只存在于 MSVC toolchain provider 中，不应污染通用 task 生成逻辑。

## 建议模块结构

```text
src/
  config/
    mod.rs
    schema.rs
    loader.rs
    presets.rs
    merge.rs

  toolchain/
    mod.rs
    msvc.rs
    gcc.rs
    clang.rs
    custom.rs

  build/
    mod.rs
    tasks.rs
    shell.rs

  clangd/
    mod.rs
    config.rs
    server.rs

  platform/
    mod.rs
    paths.rs
```

当前 `src/environment/` 下的 MSVC 相关文件可以后续移动到 `src/toolchain/msvc.rs`。第一阶段也可以先保留原位置，通过新的 MSVC provider 调用它们，以降低重构风险。

## 实施阶段

### 阶段 1：重命名与配置模型

- 将扩展元数据重命名为 `cpp-toolkit`。
- 添加配置 schema 类型。
- 添加 preset 定义。
- 添加 `.zed/cpp-toolkit.toml` 配置加载器。
- Windows 上暂时保持当前 MSVC 行为作为默认行为。

### 阶段 2：通用 clangd 生成

- 将 MSVC 专用 `.clangd` 生成逻辑改为配置驱动。
- 始终生成 `.clangd`。
- 支持 `compiler`、`compile_commands_dir`、`extra_flags` 和 `query_driver`。

### 阶段 3：通用 task 生成

- 用命令字符串 task 生成器替换 CMake/MSVC 专用 task 生成逻辑。
- 添加平台 shell 包装。
- 仅在 MSVC provider 中保留 Visual Studio developer environment 包装。

### 阶段 4：Toolchain provider

- 引入 `msvc`、`gcc`、`clang` 和 `custom` provider。
- MSVC provider 复用现有 Visual Studio 与 Windows SDK 检测逻辑。
- GCC/Clang provider 保持轻量，主要依赖用户配置命令。

### 阶段 5：文档

- 围绕 `cpp-toolkit` 重写 README。
- 为所有内置 preset 添加示例。
- 记录 `.zed/cpp-toolkit.toml` schema。
- 说明 `.clangd` 和 `.zed/tasks.json` 的生成规则。

## 第一版不做的事情

- 兼容旧扩展 ID。
- 自动发现系统上所有已安装编译器。
- 为所有构建系统做复杂 target 自动发现。
- Conan、vcpkg、Meson、Bazel 或其他包管理/构建系统自动集成。
- 高级 shell 选择配置。

这些能力可以在配置驱动基础稳定之后再添加。

## 待决策事项

暂无。初始设计已经确定：配置文件使用完整命令字符串，常见工作流使用内置 preset，所有项目必须生成 `.clangd`。
