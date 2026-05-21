# Zed MSVC C++ Assistant

Windows C++ CMake 项目的 MSVC 和 clangd 助手，适用于 Zed 编辑器。

## 版本 0.5.0

### 功能特性

- **V0.1**: MSVC 工具链探测 (vswhere.exe, MSVC v143+, Windows SDK)
- **V0.2**: CMake `compile_commands.json` 自动探测
- **V0.3**: CMake 命令生成基础设施
- **V0.4**: `.zed/tasks.json` 生成，用于 CMake 操作
- **V0.5**: neocmakelsp 集成，提供 CMake 语言支持 (LSP + 语法高亮)

## 文档

- **[使用说明 (docs/USAGE.md)](../USAGE.md)** - 安装、配置和使用指南
- **[测试指南 (docs/TESTING.md)](../TESTING.md)** - 单元测试和集成测试说明
- **[English Documentation](../..)** - 英文文档

## 快速开始

### 安装

```bash
# 编译
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release

# 安装到 Zed
mkdir -p "$USERPROFILE/.zed/extensions/zed-msvc-toolkit"
cp target/wasm32-unknown-unknown/release/zed_msvc_toolkit.wasm "$USERPROFILE/.zed/extensions/zed-msvc-toolkit/"
cp extension.toml "$USERPROFILE/.zed/extensions/zed-msvc-toolkit/"
```

### CMake 任务

将任务模板复制到工作区：

```bash
cp docs/zed-tasks-example.json .zed/tasks.json
```

然后通过 `Ctrl+Shift+T` 运行任务（任务：运行）。

### CMake 语言支持

扩展包含 [neocmakelsp](https://github.com/neocmakelsp/neocmakelsp) 用于 CMake 语言支持（`CMakeLists.txt` 文件）。

**安装：**
- 如果 `neocmakelsp` 在 `PATH` 中可用，扩展直接使用它。
- 否则，扩展会从 GitHub 下载最新匹配的发布资源：
  `neocmakelsp/neocmakelsp`。
- 如果愿意，你也可以手动安装：
  ```bash
  cargo install neocmakelsp
  ```

**配置：**

neocmakelsp 可以通过两层配置：

1. **项目级别**（项目根目录下的 `.neocmake.toml`，由 neocmakelsp 本身读取）：
   ```toml
   [format]
   enable = true

   [lint]
   enable = true

   scan_cmake_in_package = true
   semantic_token = false
   ```

2. **Zed 初始化选项**（`.zed/settings.json`，由本扩展读取）：
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

## 系统要求

- Windows 11
- Visual Studio 2022+，包含"使用 C++ 的桌面开发"工作负载
- PATH 中有 clangd（来自 LLVM）
- PATH 中有 CMake（可选，用于任务）
- 包含 `CMakeLists.txt` 的 CMake 项目

## 许可证

MIT
