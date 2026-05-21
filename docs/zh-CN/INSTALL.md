# Zed MSVC C++ Assistant - 安装与试用指南

## 构建状态

✅ WASM 模块已生成：`target/wasm32-unknown-unknown/release/zed_msvc_toolkit.wasm` (139.9K)

## 安装步骤

### 1. 本地开发安装（推荐用于测试）

在 Zed 中：

1. 打开 Zed 设置 (`Ctrl+,`)
2. 添加到 `extensions.json`：

```json
{
  "extensions": {
    "msvc-cpp": {
      "version": "0.3.0",
      "git": "E:/Rust/zed-msvc-toolkit"
    }
  }
}
```

3. 重启 Zed

### 2. 系统要求

- Windows 10/11
- Visual Studio 2022+ （含 "使用 C++ 的桌面开发" 工作负载）
- clangd（在 PATH 中可找到）
- CMake 项目

## 功能验证

### V0.1-V0.2 核心功能

打开一个 CMake 项目后，扩展会：

1. **探测 MSVC 环境**：自动查找 VS2022 和 MSVC toolset
2. **探测 Windows SDK**：自动找到 include 目录
3. **探测编译数据库**：检查 `compile_commands.json`
4. **生成 `.clangd` 配置**（如果不存在）

当前 Zed API 限制会返回提示，要求手动创建 `.clangd`：

```
当前 Zed extension API 不支持从扩展直接写入工作区 .clangd。请在工作区根目录手动创建 .clangd，内容如下：

# 由 Zed MSVC C++ Assistant 自动生成。
CompileFlags:
  Compiler: clang-cl
  Add:
    - /IC:/Program Files/Microsoft Visual Studio/2022/Community/VC/Tools/MSVC/14.40.33807/include
    - /IC:/Program Files (x86)/Windows Kits/10/Include/10.0.22621.0/ucrt
    ...
```

### 手动配置步骤

1. 复制 Zed 返回的配置内容
2. 在项目根目录创建 `.clangd` 文件
3. 粘贴配置内容
4. 重新打开 C/C++ 文件

## 已知限制

- ✅ 无法直接写入工作区文件（Zed API 限制）
- ⚠️ CMake 命令执行功能需等待 Zed API 支持
- ❌ 调试功能（DAP）未实现

## 下一步

- V0.4: 添加基础 DAP 调试支持
- V1.0: 完整的 CMake configure/build 集成
