# Zed MSVC C++ Assistant 设计文档

日期：2026-05-20

## 目标

开发一个面向 Windows C++ 项目的 Zed 扩展，目标环境是 Visual Studio 2022 或更新版本、MSVC 工具链和 CMake 项目。完整产品最终覆盖 MSVC 环境探测、clangd 配置、CMake 辅助和基于 DAP 的 MSVC 调试。

第一阶段实现目标是 V0.1。V0.1 只交付智能编辑路径：探测本机 MSVC 环境，在合适时保守生成 `.clangd` 配置，并以 MSVC 兼容方式启动 `clangd`。

## 完整实现路径

### 阶段 0：扩展基础设施

把当前 Rust crate 改造成标准 Zed Rust 扩展。

交付内容：

- 添加 `extension.toml`，包含扩展元数据、语言服务器声明和必要 capability。
- 配置 `Cargo.toml`，将 crate 调整为 WebAssembly 扩展所需的 `crate-type = ["cdylib"]`。
- 接入 `zed_extension_api`。
- 在 `src/lib.rs` 注册扩展。
- 建立 `environment`、`lsp`、`cmake`、`debug` 等模块边界。

### 阶段 1：V0.1 智能感知 MVP

让 clangd 能在 Windows MSVC 项目中可用。

交付内容：

- 通过 `C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe` 探测 Visual Studio 2022+。
- 在探测到的 Visual Studio 安装目录下选择最高版本 MSVC toolset。
- 在可用时探测 Windows SDK include 目录。
- 从 PATH 探测 `clangd`。
- 只在工作区不存在 `.clangd` 时生成配置文件。
- 启动 clangd 时加入 `--header-insertion=never`。
- 用单元测试覆盖路径排序、`.clangd` 渲染和 SDK 缺失时的降级输出。

### 阶段 2：V0.2 CMake 编译数据库协同

让 clangd 优先使用真实编译数据库，减少 include 和宏识别误差。

交付内容：

- 探测工作区根目录和 `build/` 下的 `compile_commands.json`。
- 在生成的 `.clangd` 中加入 `CompilationDatabase`。
- 尽可能探测 CMake Visual Studio generator 元数据。
- 在需要时提示用户使用 Ninja 生成编译数据库。

### 阶段 3：V0.3 CMake 命令

从 Zed 内暴露 configure 和 build 工作流。

交付内容：

- 探测 `cmake` 和 `ninja`。
- Ninja 存在时使用 `cmake -B build -G "Ninja" -DCMAKE_BUILD_TYPE=Debug`。
- Ninja 不存在时回退到 Visual Studio 2022 generator。
- 使用 `cmake --build build --config Debug` 构建。
- 将命令输出呈现在 Zed 的终端或 task UI 中。

**状态**: ✅ 完成。命令生成基础设施已实现，待 Zed API 支持后即可暴露为用户命令。

### 阶段 4：V0.4 任务系统

绕过 Zed API 限制，通过 .zed/tasks.json 提供 CMake 命令。

交付内容：

- 生成 `.zed/tasks.json` 格式的 CMake configure 和 build 任务。
- 使用 `$ZED_WORKTREE_ROOT` 变量引用工作区路径。
- 支持 Debug/Release 构建类型配置。

**状态**: ✅ 完成。`generate_tasks_json()` 函数已实现，用户可手动创建任务文件或等待未来 API 支持自动写入。

### 阶段 5：V0.5 调试基础

引入 MSVC DAP 支持，但暂不自动下载调试器。

交付内容：

- 注册 `msvc-cpp` debug adapter 类型。
- 支持用户提供或本机探测到的 `vsdbg.exe`。
- 将调试配置映射为 DAP launch 请求。
- 支持 `program`、`cwd`、`symbolPath`。

### 阶段 5：V0.5 调试基础

引入 MSVC DAP 支持，但暂不自动下载调试器。

交付内容：

- 注册 `msvc-cpp` debug adapter 类型。
- 支持用户提供或本机探测到的 `vsdbg.exe`。
- 将调试配置映射为 DAP launch 请求。
- 支持 `program`、`cwd`、`symbolPath`。

### 阶段 6：V1.0 零配置调试

完成可发布级别的完整工作流。

交付内容：

- 通过 HTTPS 下载 `vsdbg.exe`。
- 使用微软发布的 SHA256 哈希校验下载内容。
- 从 CMake 输出推断可执行文件和 PDB 路径。
- 为常见 CMake 项目生成可用的 F5 调试场景。
- 提供完整的用户诊断信息和文档。

## V0.1 架构

V0.1 按最终架构铺开模块，但只实现智能编辑路径。

### 文件与模块

- `src/lib.rs`：Zed 扩展入口。注册扩展，并把具体行为转发到聚焦模块。
- `src/environment/mod.rs`：环境探测的公共 API。
- `src/environment/vswhere.rs`：调用固定路径的 `vswhere.exe`，解析 Visual Studio 安装路径。
- `src/environment/msvc.rs`：定位并选择最高版本 MSVC toolset。
- `src/environment/windows_sdk.rs`：定位 Windows SDK include 目录；探测失败时返回可降级状态。
- `src/environment/tools.rs`：查找外部工具，例如 `clangd`。后续阶段扩展到 `cmake`、`ninja` 和 `vsdbg`。
- `src/lsp/mod.rs`：LSP 集成的公共 API。
- `src/lsp/clangd_config.rs`：根据探测结果渲染 `.clangd` 内容。
- `src/lsp/server.rs`：构建 clangd 启动命令和参数。
- `src/cmake/mod.rs`：后续 CMake 功能的模块边界。
- `src/debug/mod.rs`：后续 DAP 功能的模块边界。
- `src/error.rs`：轻量的用户可读错误类型。
- `src/paths.rs`：版本排序和路径格式化辅助函数。

### 数据流

当 Zed 请求 C/C++ 语言服务器时：

1. 扩展从当前 worktree 解析工作区根目录。
2. 扩展通过 `vswhere` 探测 Visual Studio。
3. 扩展找到最高版本 MSVC toolset include 目录。
4. 扩展尝试探测 Windows SDK include 目录。
5. 扩展根据探测结果渲染 `.clangd` 内容。
6. 如果 `.clangd` 不存在，扩展写入生成文件。
7. 如果 `.clangd` 已存在，扩展不修改文件，并返回用户可读提示。
8. 扩展从 PATH 解析 `clangd`。
9. 扩展返回带 `--header-insertion=never` 的 clangd 命令。

如果 Zed 当前扩展 API 不允许在语言服务器准备阶段直接创建工作区文件，V0.1 仍保留可测试的 `.clangd` 渲染逻辑，并返回清晰提示，说明用户应把生成内容放到哪里。实现不能静默假装文件已经写入。

## V0.1 行为

### `.clangd` 策略

V0.1 使用保守生成策略。

- 如果工作区根目录没有 `.clangd`，生成一个。
- 如果 `.clangd` 已存在，不覆盖。
- V0.1 不合并 YAML。
- V0.1 不维护标记块。

生成文件包含简短头部注释，说明该文件由扩展生成。

### Visual Studio 探测

扩展调用：

```text
C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe
```

探测逻辑只主动支持 Visual Studio 2022 或更新版本。更旧版本可能偶然可用，但不属于 V0.1 支持承诺。

### MSVC Toolset 选择

扩展扫描已发现 Visual Studio 安装目录下的 MSVC toolset 目录，并选择最高版本。版本比较尽量按路径片段中的数字比较，而不是普通字符串字典序。

### Windows SDK 探测

扩展尝试定位以下 SDK include 目录：

- `ucrt`
- `um`
- `shared`

如果 SDK 探测成功，这些 include 目录会写入 `.clangd`。

如果 SDK 探测失败，扩展仍生成有用的 `.clangd`：正常写入 MSVC include 路径，并用注释说明用户应手动补充 SDK include 项。

### clangd 探测

扩展从 PATH 探测 `clangd`。

如果缺少 `clangd`：

- `.clangd` 仍可在条件满足时生成。
- LSP 启动返回清晰错误，提示用户安装 LLVM 或把 `clangd.exe` 加入 PATH。
- V0.1 不下载 LLVM 或 clangd。

### clangd 命令

clangd 命令包含：

```text
--header-insertion=never
```

生成的 `.clangd` 使用：

```yaml
CompileFlags:
  DriverMode: cl
  Add:
    - /I...
```

## 错误处理

V0.1 使用用户可读错误，不静默失败。

- 缺少 `vswhere.exe`：提示需要 Visual Studio Installer 或 VS2022+。
- 找不到 VS2022+：提示需要安装 Visual Studio 2022+ 和“使用 C++ 的桌面开发”工作负载。
- 缺少 MSVC toolset：提示需要安装 MSVC v143+ build tools。
- 缺少 Windows SDK：生成降级 `.clangd` 模板，并用注释提示手动补充 SDK 路径。
- 缺少 `clangd`：尽可能生成 `.clangd`，但 LSP 启动失败时提示安装或配置 PATH。
- `.clangd` 已存在：不覆盖，并提示已保留用户配置。

## 测试策略

单元测试覆盖不依赖真实 Zed 运行时的逻辑：

- 版本目录排序。
- 从示例目录名中选择 MSVC include 路径。
- SDK 路径存在时的 Windows SDK include 渲染。
- SDK 路径缺失时的降级 `.clangd` 渲染。
- `.clangd` 已存在时的策略决策。
- clangd 参数构建。

真实 Zed 运行时集成测试推迟到扩展骨架能编译并能本地加载之后。

## V0.1-V0.4 已完成

- ✅ V0.1: MSVC 环境探测和 clangd 配置
- ✅ V0.2: CMake `compile_commands.json` 自动探测
- ✅ V0.3: CMake 命令生成基础设施
- ✅ V0.4: `.zed/tasks.json` 任务系统支持

## V0.5 明确不做

- 不自动下载 `vsdbg.exe`。
- 不合并已有 `.clangd` YAML。
- 不承诺支持 Visual Studio 2019 或更旧版本。

## 实现风险

- Zed 当前扩展 API 可能限制从语言服务器准备回调中直接写工作区文件。
- 首次编译后可能需要调整 Zed extension capability 声明。
- 需要用当前 Zed 扩展示例验证语言服务器 ID 和 C/C++ 语言绑定的准确写法。

这些风险不改变总体架构。它们只影响 `src/lib.rs` 和 `src/lsp/server.rs` 的适配层；`environment` 和配置渲染模块应保持稳定。
