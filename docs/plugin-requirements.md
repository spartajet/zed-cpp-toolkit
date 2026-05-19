这是一份为 Zed 编辑器开发的 **MSVC C++ 支持插件** 的软件需求说明书（SRS）。本规范聚焦于 Windows 平台下，基于 Visual Studio 2022+ 和 CMake 的 C++ 开发与调试工作流。
---
# 软件需求说明书 (SRS)：Zed MSVC C++ Assistant
## 1. 引言
### 1.1 目的
本文档旨在明确 Zed 编辑器扩展 `Zed MSVC C++ Assistant` 的功能需求、非功能需求及约束条件。该插件旨在弥补 Zed 在 Windows 平台下对 MSVC 工具链和 CMake 原生支持的不足，提供开箱即用的 C++ 智能编辑、构建与调试体验。
### 1.2 范围
本插件适用于以下技术栈：
- **操作系统**: Windows 10/11
- **编译器工具链**: MSVC (随 Visual Studio 2022 v143+ 或更高版本安装)
- **构建系统**: CMake (3.15+)
- **编辑器**: Zed (最新稳定版)
- **语言**: C/C++
**包含功能**：
- 自动探测 VS2022+ 安装路径及 Windows SDK。
- 自动生成并维护 `.clangd` 配置，确保 `clangd` 在 MSVC 环境下精准工作。
- 辅助 CMake 配置与 `compile_commands.json` 的生成/对接。
- 集成基于 DAP 的 MSVC 调试器 (`vsdbg`)，支持 PDB 断点调试。
**不包含功能**：
- 旧版 Visual Studio (2019 及以下) 的主动支持（可能兼容但不保证）。
- 非 CMake 项目（如纯 `.sln`/`.vcxproj`）的智能感知支持。
- GUI 化的 CMake 项目配置面板（如 VSCode 的 CMake Tools 侧边栏，V1 版本暂不实现）。
---
## 2. 总体描述
### 2.1 用户特征与场景
目标用户为在 Windows 环境下使用 Zed 进行 C++ 开发的工程师。他们熟悉 CMake 和 MSVC，但受够了 Zed 默认环境下头文件标红、无法识别 MSVC 特有宏（如 `__declspec`）、以及无法调试 PDB 程序的痛点。
### 2.2 运行环境与约束
- **沙箱约束**: 插件运行于 Zed 的 Wasm 沙箱中，无法直接执行 `vcvarsall.bat` 设置环境变量。必须通过解析文件系统或调用外部可执行文件获取路径，并将结果写入配置文件或作为命令行参数传递。
- **依赖要求**: 用户必须已安装 Visual Studio 2022 及其 "使用 C++ 的桌面开发" 工作负载，且系统 PATH 中包含 `cmake.exe`。
---
## 3. 功能需求
### 3.1 环境探测模块
**FR1.1: Visual Studio 实例探测**
- 插件需通过执行 `vswhere.exe`（路径硬编码为 `C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe`）获取 VS2022+ 的安装路径。
- 需提取最高版本的 MSVC 编译器路径（如 `MSVC/14.xx.xxxxx`）及对应的 `include` 和 `lib` 目录。
**FR1.2: Windows SDK 探测**
- 解析 VS 安装目录或注册表，获取当前安装的 Windows SDK 版本及 `um`、`shared`、`ucrt` 等头文件路径。
**FR1.3: CMake 探测**
- 验证 `cmake.exe` 是否可用，若不可用，在 Zed 内弹出错误提示。
---
### 3.2 LSP 智能感知支持
**FR2.1: 动态 `.clangd` 配置生成**
- 当工作区中不存在 `.clangd` 文件时，插件自动在项目根目录生成 `.clangd` 文件。
- 配置内容需包含：
  - `CompileFlags.DriverMode: cl`：指示 clangd 使用 MSVC 兼容模式。
  - `CompileFlags.Add`：自动注入探测到的 MSVC 及 Windows SDK 的 `/I`（包含目录）参数。
**FR2.2: CMake 编译数据库 协同**
- **FR2.2.1**: 如果工作区根目录或 `build/` 目录下存在 `compile_commands.json`，`.clangd` 配置中需添加 `CompilationDatabase: <path>` 指向该文件。
- **FR2.2.2**: CMake 使用 Visual Studio 生成器（如 `-G "Visual Studio 17 2022"`）时，默认不会在源码根目录生成 `compile_commands.json`。插件需检测 `build/.cmake/api/v1/reply` 或提示用户使用 Ninja 生成器（`-G "Ninja"`）以生成数据库。
**FR2.3: 语言服务器启动**
- 使用系统 PATH 中的 `clangd`（若找不到，提示用户安装）。
- 启动参数需包含 `--header-insertion=never`（避免 clangd 自动插入头文件与 MSVC 宏冲突）。
---
### 3.3 构建辅助模块
**FR3.1: CMake Configure 命令**
- 注册 Zed 命令 `msvc-cpp: cmake configure`。
- 执行时，自动构建 `cmake -B build -G "Ninja" -DCMAKE_BUILD_TYPE=Debug` 命令（优先使用 Ninja 以确保生成编译数据库，若系统无 Ninja 则 fallback 到 VS 生成器）。
- 在 Zed 底部终端面板输出结果。
**FR3.2: CMake Build 命令**
- 注册 Zed 命令 `msvc-cpp: cmake build`。
- 执行 `cmake --build build --config Debug`。
---
### 3.4 调试支持模块
**FR4.1: 调试适配器 管理**
- 插件需内嵌或自动下载微软官方的 `vsdbg.exe`（Visual Studio Code 的 DAP 调试核心）。
- 下载目标路径：`<Zed Extension Data Dir>/msvc-cpp/vsdbg.exe`。
**FR4.2: 调试配置 启动**
- 注册 DAP 类型 `msvc-cpp`。
- 当用户按 F5 或通过调试面板启动时，插件根据当前打开的文件或项目配置，自动生成 `launch.json` 或在内存中构建 DAP 启动参数：
  - `program`: 指向 `build/Debug/<target>.exe`（需结合 CMake 的 `CMAKE_RUNTIME_OUTPUT_DIRECTORY` 或默认路径推测）。
  - `symbolPath`: 指向对应的 `.pdb` 文件。
  - `cwd`: 工作区路径。
**FR4.3: 断点与变量查看**
- 依赖 Zed 原生的 DAP 客户端能力，通过 `vsdbg.exe` 实现源码级断点、单步执行、局部变量与监视查看。
---
## 4. 非功能需求
### 4.1 性能需求
- **NFR1.1**: 环境探测（`vswhere` 等）总耗时不得超过 2 秒，避免阻塞编辑器启动。
- **NFR1.2**: 自动生成的 `.clangd` 文件大小应控制在合理范围，避免注入过多无效路径导致 `clangd` 启动缓慢。
### 4.2 可用性需求
- **NFR2.1 (零配置理念)**: 用户安装插件并打开包含 `CMakeLists.txt` 的目录后，无需手动配置任何路径，代码高亮与跳转应立即生效。
- **NFR2.2**: 所需的外部依赖（如 Ninja, clangd）缺失时，需通过 Zed 的通知系统给出明确的一键安装指引或报错信息，而非静默失败。
### 4.3 安全性需求
- **NFR3.1**: `vsdbg.exe` 的下载必须使用 HTTPS，并校验微软官方的 SHA256 哈希值，防止供应链攻击。
### 4.4 兼容性需求
- **NFR4.1**: 插件生成的 `.clangd` 文件不应覆盖用户手动修改的自定义配置，若已存在 `.clangd`，应采用合并策略或在追加内容前提示用户。
---
## 5. 数据与接口设计
### 5.1 外部接口调用
| 接口名称 | 调用方式 | 用途 |
| :--- | :--- | :--- |
| `vswhere.exe` | 同步执行 Shell | 获取 VS 及 MSVC 安装绝对路径 |
| `cmake.exe` | Zed 异步终端任务 | 执行项目的 Configure/Build |
| `clangd.exe` | Zed Language Server 接口 | 提供 LSP 服务 |
| `vsdbg.exe` | Zed Debug Adapter 接口 | 提供 DAP 调试服务 |
### 5.2 生成文件格式示例 (`.clangd`)
```yaml
# 由 Zed MSVC C++ Assistant 自动生成
CompileFlags:
  DriverMode: cl
  Add:
    # MSVC 标准库
    - /IC:/Program Files/Microsoft Visual Studio/2022/Community/VC/Tools/MSVC/14.38.33130/include
    # Windows SDK
    - /IC:/Program Files (x86)/Windows Kits/10/Include/10.0.22621.0/ucrt
    - /IC:/Program Files (x86)/Windows Kits/10/Include/10.0.22621.0/um
    - /IC:/Program Files (x86)/Windows Kits/10/Include/10.0.22621.0/shared
Diagnostics:
  Suppress: ['pp_file_not_found'] # 优化：抑制部分因 SDK 深层依赖未找到导致的误报
```
---
## 6. 里程碑规划 (建议)
- **V0.1 (MVP)**: 实现 `vswhere` 探测与 `.clangd` 自动生成，打通 MSVC 环境下的 `clangd` 智能提示。
- **V0.2**: 集成 `vsdbg`，实现基础的 PDB 调试（手动配置 `launch.json`）。
- **V1.0 (Release)**: 增加 CMake Configure/Build 的 Zed 命令绑定，实现根据 CMake 输出目录自动推测并启动调试（零配置调试）。完善错误处理与用户提示。
