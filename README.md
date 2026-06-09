# Zed C++ Toolkit

`cpp-toolkit` is a cross-platform C/C++ extension for Zed. It uses a project-local
`.zed/cpp-toolkit.toml` file to describe toolchains, build commands, run commands,
generated Zed tasks, and `clangd` behavior for Windows/MSVC, Linux/WSL, macOS,
CMake/Ninja, and Makefile projects.

> 中文：`cpp-toolkit` 是一个面向 Zed 的跨平台 C/C++ 扩展。项目通过
> `.zed/cpp-toolkit.toml` 配置 toolchain、构建命令、运行命令、Zed task 和
> `clangd` 行为，支持 Windows/MSVC、Linux/WSL、macOS、CMake/Ninja 和 Makefile 项目。

## Features

- Config-driven C/C++ workspace setup via `.zed/cpp-toolkit.toml`.
- Built-in presets: `msvc-cmake-ninja`, `gcc-cmake-ninja`, `clang-cmake-ninja`,
  `gcc-make`, `clang-make`, and `custom`.
- Generates `.clangd` while preserving user-authored `.clangd` files.
- Generates `.zed/tasks.json` with Configure, Build, Clean, Build Target, and Run
  tasks.
- Discovers CMake targets from CMake file-api replies or `build.ninja`.
- Generates Run tasks only for executable CMake targets; library targets get Build
  Target tasks only.
- Keeps Windows/MSVC support, including best-effort Visual Studio, MSVC include, and
  Windows SDK include discovery.
- Provides CMake language support through `neocmakelsp` when it is available in
  `PATH`.

> 中文：核心能力包括配置驱动、内置 preset、自动生成 `.clangd` 和 `.zed/tasks.json`、
> 自动发现 CMake target、只为可执行 target 生成 Run 任务，并继续支持 Windows/MSVC 与
> `neocmakelsp`。

## Requirements

- Zed.
- `clangd` available in the Zed host or remote environment `PATH`, or configured with
  an absolute path in `[clangd].command`.
- Compiler and build tools matching the selected preset:
  - Windows/MSVC: Visual Studio 2022 or Build Tools, CMake, Ninja, `clangd`.
  - Linux/WSL GCC: `gcc`, `g++`, CMake, Ninja, `clangd`.
  - Clang: `clang`, `clang++`, CMake, Ninja, `clangd`.
  - Makefile: `make`, a matching C/C++ compiler, `clangd`.
- Optional CMake LSP support requires `neocmakelsp` in `PATH`. The extension only
  discovers an existing binary; it does not download one automatically.

> 中文：基础依赖是 Zed、`clangd` 和对应 preset 需要的编译/构建工具。CMake LSP 是可选的，
> 需要你自己安装 `neocmakelsp` 并放入 `PATH`。

## Installation

Install this extension in Zed, or load this repository as a local development
extension. For local development and manual verification, build the WASM artifact:

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
```

Build artifact:

```text
target/wasm32-unknown-unknown/release/cpp_toolkit.wasm
```

Extension metadata:

```toml
id = "cpp-toolkit"
name = "Zed C++ Toolkit"
```

> 中文：在 Zed 中安装扩展，或把本仓库作为本地开发扩展加载。手动构建产物是
> `target/wasm32-unknown-unknown/release/cpp_toolkit.wasm`。

## Quick Start

Create `.zed/cpp-toolkit.toml` in your project root:

```toml
preset = "gcc-cmake-ninja"

[build]
build_type = "Debug"
build_dir = "build"

[clangd]
extra_flags = ["-std=c++20"]
```

After opening a C or C++ file, the extension will:

1. Read `.zed/cpp-toolkit.toml`.
2. Merge the selected preset with user configuration.
3. Generate or refresh the generated `.clangd`.
4. Generate `.zed/tasks.json`.
5. Start `cpp-toolkit-clangd`.

After changing `.zed/cpp-toolkit.toml`, restart `cpp-toolkit-clangd` or reopen the
workspace so `.clangd` and tasks are regenerated.

> 中文：在项目根目录创建 `.zed/cpp-toolkit.toml` 后，打开 C/C++ 文件即可触发配置合并、
> `.clangd` 生成、task 生成和 `cpp-toolkit-clangd` 启动。修改配置后需要重启语言服务或重新打开
> workspace。

## Presets

| Preset | Use Case | Default Build System |
| --- | --- | --- |
| `msvc-cmake-ninja` | Windows + MSVC + CMake + Ninja | `cmake` |
| `gcc-cmake-ninja` | GCC + CMake + Ninja | `cmake` |
| `clang-cmake-ninja` | Clang + CMake + Ninja | `cmake` |
| `gcc-make` | GCC + Makefile | `make` |
| `clang-make` | Clang + Makefile | `make` |
| `custom` | Fully custom commands | `custom` |

When no configuration file exists, the extension infers the default preset from the
workspace path:

- Windows paths such as `C:\repo`: `msvc-cmake-ninja`.
- Other paths: `gcc-cmake-ninja`.

For macOS or Clang projects, set `preset = "clang-cmake-ninja"` explicitly.

> 中文：没有配置文件时，Windows 路径默认使用 `msvc-cmake-ninja`，其他路径默认使用
> `gcc-cmake-ninja`。macOS 或 Clang 项目建议显式配置 `clang-cmake-ninja`。

## Configuration

The configuration file path is fixed:

```text
.zed/cpp-toolkit.toml
```

Top-level preset field:

```toml
preset = "gcc-cmake-ninja"
```

### `[toolchain]`

```toml
[toolchain]
name = "gcc"
cc = "gcc"
cxx = "g++"
```

These fields are merged with the selected preset and are used by generated CMake
configure commands. The MSVC preset uses `cl` by default.

> 中文：`[toolchain]` 用于覆盖 preset 里的编译器配置。MSVC preset 默认使用 `cl`。

### `[build]`

```toml
[build]
system = "cmake"
build_dir_style = "build"
build_dir = "build"
build_type = "Debug"
configure = "cmake -S . -B {build_dir} -G Ninja -DCMAKE_BUILD_TYPE={build_type} -DCMAKE_EXPORT_COMPILE_COMMANDS=ON"
build = "cmake --build {build_dir}"
clean = "cmake --build {build_dir} --target clean"
```

Supported command templates:

- `{build_dir}`: final build directory.
- `{build_type}`: for example `Debug`, `Release`, or `RelWithDebInfo`.

Supported `build_dir_style` values:

- `build`: default, resolves to `build`.
- `clion`: resolves to paths such as `cmake-build-debug` and `cmake-build-release`.
- `custom`: requires an explicit `build_dir`.

If `build_dir` is set, it always wins over `build_dir_style`.

> 中文：`[build]` 定义 Configure/Build/Clean 命令。命令支持 `{build_dir}` 和
> `{build_type}`。显式设置 `build_dir` 时，它优先于 `build_dir_style`。

### `[run]`

```toml
[run]
command = "./build/my-app"
cwd = "$ZED_WORKTREE_ROOT"
```

If `[run].command` is not configured for a CMake project, the extension tries to
discover executable targets and generates `C++: Run: <target>` tasks. If
`[run].command` is configured, the manual Run task takes precedence.

> 中文：CMake 项目未配置 `[run].command` 时会自动发现可执行 target；配置了
> `[run].command` 时，以手动 Run 任务为准。

### `[clangd]`

```toml
[clangd]
command = "clangd"
compiler = "g++"
compile_commands_dir = "{build_dir}"
extra_flags = ["-std=c++20", "-Iinclude"]
query_driver = ["gcc", "g++"]
```

Field behavior:

- `command`: `clangd`, a versioned binary such as `clangd-18`, or an absolute path.
- `compiler`: written to `.clangd` as `CompileFlags.Compiler`.
- `compile_commands_dir`: written to `.clangd` as
  `CompileFlags.CompilationDatabase`.
- `extra_flags`: written to `.clangd` as `CompileFlags.Add`.
- `query_driver`: passed to `clangd` as `--query-driver=...`.

Generated `.clangd` files also remove CMake/GCC flags that common clangd drivers do
not understand:

```yaml
Remove:
  - -fdeps-format=*
  - -fmodules-ts
  - -fmodule-mapper=*
```

This addresses diagnostics such as
`Unknown argument: '-fdeps-format=p1689r5'` when clangd reads
`compile_commands.json`.

> 中文：`[clangd]` 控制 clangd 命令、编译器模式、编译数据库目录、额外参数和
> `--query-driver`。自动生成的 `.clangd` 会过滤 clangd 常见不支持的 GCC/CMake 参数。

## Generated Files

The extension writes these workspace files:

```text
.clangd
.zed/tasks.json
```

`.clangd` protection rules:

- User-authored `.clangd` files are preserved.
- Only files marked with `# Auto-generated by Zed C++ Toolkit.` or the legacy
  generated marker are refreshed.

`.zed/tasks.json` is refreshed from the current effective configuration. Common task
labels:

- `C++: Configure`
- `C++: Build`
- `C++: Build Target: <target>`
- `C++: Clean`
- `C++: Run`
- `C++: Run: <target>`

> 中文：扩展会写 `.clangd` 和 `.zed/tasks.json`。用户手写 `.clangd` 不会被覆盖；只有自动生成标记的
> `.clangd` 会被刷新。

## CMake Target Discovery

CMake target discovery requires running `C++: Configure` at least once so CMake can
create the build directory.

Discovery order:

1. Read CMake file-api replies.
2. Fall back to parsing `build.ninja` in the build directory.

Discovery rules:

- Executable targets get both `C++: Build Target: <target>` and
  `C++: Run: <target>`.
- Library targets get `C++: Build Target: <target>` only.
- Internal CMake targets, `all`, `clean`, `edit_cache`, `rebuild_cache`,
  `*_autogen`, `CMakeFiles/...`, and path-like phony outputs are filtered out.
- Linux/WSL executables without `.exe` suffix are supported.

> 中文：CMake target 发现依赖 Configure 后生成的构建目录。扩展优先读 CMake file-api，
> 回退解析 `build.ninja`。可执行 target 会生成 Run 任务；库 target 不会。

## Common Configurations

### Linux/WSL + GCC + CMake + Ninja

```toml
preset = "gcc-cmake-ninja"

[build]
build_type = "Debug"
build_dir_style = "clion"

[clangd]
compiler = "g++"
compile_commands_dir = "{build_dir}"
query_driver = ["gcc", "g++"]
```

### Qt + CMake

Qt projects usually use the normal CMake/Ninja workflow. Configure the Qt path in
your CMake project or command line:

```toml
preset = "gcc-cmake-ninja"

[build]
build_type = "Debug"
build_dir = "cmake-build-debug"
configure = "cmake -S . -B {build_dir} -G Ninja -DCMAKE_BUILD_TYPE={build_type} -DCMAKE_C_COMPILER=gcc -DCMAKE_CXX_COMPILER=g++ -DCMAKE_EXPORT_COMPILE_COMMANDS=ON"

[clangd]
compiler = "g++"
compile_commands_dir = "{build_dir}"
query_driver = ["gcc", "g++"]
```

If Qt is installed outside system paths, set `CMAKE_PREFIX_PATH` in `CMakeLists.txt`
or add it to the configure command:

```toml
configure = "cmake -S . -B {build_dir} -G Ninja -DCMAKE_BUILD_TYPE={build_type} -DCMAKE_PREFIX_PATH=/path/to/Qt/6.x/gcc_64 -DCMAKE_EXPORT_COMPILE_COMMANDS=ON"
```

> 中文：Qt 项目仍走普通 CMake/Ninja 流程。Qt 不在系统路径时，在 `CMakeLists.txt` 或
> configure 命令里设置 `CMAKE_PREFIX_PATH`。

### Windows + MSVC + CMake + Ninja

```toml
preset = "msvc-cmake-ninja"

[build]
build_type = "Debug"
build_dir = "build"

[clangd]
compiler = "clang-cl"
compile_commands_dir = "{build_dir}"
query_driver = []
```

### Makefile

```toml
preset = "gcc-make"

[build]
build = "make -j"
clean = "make clean"

[run]
command = "./app"
cwd = "$ZED_WORKTREE_ROOT"

[clangd]
compiler = "g++"
compile_commands_dir = "."
query_driver = ["gcc", "g++"]
```

### Fully Custom

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
query_driver = []
```

## CMake LSP

The CMake language server ID is `cpp-toolkit-neocmake`. It requires
`neocmakelsp` to be discoverable in `PATH`.

Optional `.zed/settings.json` configuration:

```json
{
  "lsp": {
    "cpp-toolkit-neocmake": {
      "format": { "enable": true },
      "lint": { "enable": true },
      "scan_cmake_in_package": true,
      "semantic_token": true
    }
  }
}
```

> 中文：CMake LSP 的 ID 是 `cpp-toolkit-neocmake`。如果需要 CMake 语言服务，请确保
> `neocmakelsp` 在 `PATH` 中。

## Troubleshooting

### No Run Task

Run `C++: Configure` first. CMake target discovery depends on `build.ninja` or CMake
file-api replies. Library-only projects do not generate Run tasks. You can always
set `[run].command` manually.

> 中文：先运行 `C++: Configure`。只有可执行 target 才会生成 Run 任务；也可以手动配置
> `[run].command`。

### Unexpected `C++: Build Target: /path/to/CMakeLists.txt`

Older versions could treat path-like phony outputs in `build.ninja` as targets.
Version 1.0.0 filters phony targets containing `/` or `\`. Re-run Configure and
restart `cpp-toolkit-clangd` to refresh tasks.

> 中文：旧版本可能把路径型 phony 输出识别成 target。1.0.0 会过滤这类 target。

### `.clangd` Warns `Expected scalar or list of scalars`

Older versions could generate an empty `CompileFlags.Add:` node. Version 1.0.0 omits
`Add` when there are no extra flags or include paths. Delete the old generated
`.clangd` or restart `cpp-toolkit-clangd` so the extension can refresh it.

> 中文：旧版本可能生成空 `Add:`。1.0.0 没有额外 flags/includes 时不会写空 `Add`。

### clangd Reports `Unknown argument: '-fdeps-format=p1689r5'`

clangd reads compiler arguments from `compile_commands.json`, but the active clangd
driver may not understand some GCC/CMake flags. Version 1.0.0 writes
`CompileFlags.Remove` for `-fdeps-format=*`, `-fmodules-ts`, and
`-fmodule-mapper=*`.

> 中文：这是 clangd 不认识编译数据库里的某些参数。1.0.0 会通过 `.clangd` 的
> `CompileFlags.Remove` 过滤这些参数。

### Zed Stays at `indexing(0%)`

Restart Zed or restart the workspace language server. If logs only show `fs_watcher`
messages such as `No watch was found` for temporary CMake scratch directories, that
is usually file-watcher noise from short-lived CMake directories, not necessarily a
clangd deadlock. If it reproduces, inspect the `cpp-toolkit-clangd` startup args and
generated `.clangd`.

> 中文：先重启 Zed 或语言服务。CMake 临时目录触发的 `No watch was found` 多数是监听噪声；
> 复现时再检查 clangd 启动参数和 `.clangd`。

### MSVC Includes Are Missing on Windows

Confirm Visual Studio 2022 or Build Tools is installed and `vswhere.exe` exists at:

```text
C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe
```

You can also provide a user-authored `.clangd` with manual include paths. The
extension will not overwrite user-authored `.clangd` files.

> 中文：确认安装 VS 2022/Build Tools 且 `vswhere.exe` 在默认路径。也可以手写 `.clangd`，
> 扩展不会覆盖用户手写文件。

## Development

Useful checks:

```bash
cargo fmt --check
cargo test
cargo check
cargo build --target wasm32-unknown-unknown --release
```

Release checklist:

- `extension.toml` version is updated.
- `Cargo.toml` version is updated.
- `README.md` covers installation, configuration, generated tasks, and
  troubleshooting.
- The `vX.Y.Z` tag points at the release commit on `master`.

> 中文：发布前检查版本号、README、测试、wasm release 构建，并确认 tag 指向 master 上的发布提交。

## License

MIT
