# Zed MSVC C++ Assistant - 使用说明

## 安装

### 1. 编译扩展

```bash
# 添加 WASM 目标（必须通过 rustup 安装 Rust）
rustup target add wasm32-unknown-unknown

# 编译 Release 版本
cargo build --target wasm32-unknown-unknown --release
```

### 2. 作为 Dev Extension 安装

Zed 支持本地开发扩展（Dev Extension），无需发布即可使用：

1. 在 Zed 中打开扩展面板：`Ctrl+Shift+X`
2. 点击 `Install Dev Extension` 按钮
3. 选择项目根目录 `E:\Rust\zed-msvc-toolkit`
4. Zed 会自动编译并加载扩展

**注意**：
- 项目必须是 Git 仓库
- Rust 必须通过 rustup 安装（不能通过其他方式如 homebrew）
- 如需调试日志，使用 `zed --foreground` 启动 Zed

### 3. 发布到扩展市场

发布扩展需要 PR 到 `zed-industries/extensions` 仓库：

1. 将项目推送到公开的 GitHub 仓库
2. Fork `zed-industries/extensions` 仓库
3. 作为 Git submodule 添加你的扩展
4. 更新 `extensions.toml`
5. 提交 PR

## 系统要求

- **操作系统**: Windows 11
- **Visual Studio**: 2022 或更新版本
  - 需要安装 "使用 C++ 的桌面开发" 工作负载
- **LLVM**: 需要安装 clangd 并加入 PATH
- **CMake** (可选): 如需使用任务系统

## 功能

### V0.1: MSVC 环境探测

扩展自动探测以下内容：
- Visual Studio 2022+ 安装路径（通过 vswhere.exe）
- MSVC v143+ 工具链
- Windows SDK 包含目录
- clangd 可执行文件

### V0.2: 编译数据库支持

自动探测以下位置的 `compile_commands.json`：
- 工作区根目录
- `build/` 子目录

### V0.4: CMake 任务系统

通过 Zed 任务系统提供 CMake 操作。

## 配置

### Language Server

扩展自动为 C/C++ 文件提供 clangd 语言服务器。无需额外配置。

### CMake 任务

1. 复制任务文件模板到工作区：
   ```
   cp docs/zed-tasks-example.json .zed/tasks.json
   ```

2. 在 Zed 中打开任务面板：`Ctrl+Shift+T`

3. 选择要运行的任务：
   - `CMake: Configure (Debug)` - 配置 CMake 项目
   - `CMake: Build (Debug)` - 构建 Debug 版本
   - `CMake: Configure (Release)` - 配置 Release 版本
   - `CMake: Build (Release)` - 构建 Release 版本

### 自定义构建目录

如果使用不同的构建目录，编辑 `.zed/tasks.json`：

```json
{
  "label": "CMake: Configure (Debug)",
  "command": "cmake",
  "args": [
    "-S",
    "$ZED_WORKTREE_ROOT",
    "-B",
    "$ZED_WORKTREE_ROOT/cmake-build-debug",  // 修改此处
    "-DCMAKE_BUILD_TYPE=Debug"
  ]
}
```

## 故障排除

### clangd 无法启动

**错误**: "找不到 clangd"

**解决方案**:
1. 安装 LLVM: https://llvm.org/builds/
2. 确保 `clangd.exe` 在 PATH 中
3. 重启 Zed

### 找不到 Visual Studio

**错误**: "找不到 Visual Studio 2022+"

**解决方案**:
1. 安装 Visual Studio 2022
2. 确保安装 "使用 C++ 的桌面开发" 工作负载
3. 重启 Zed

### 找不到 MSVC 工具链

**错误**: "找不到 MSVC v143+ toolset"

**解决方案**:
1. 打开 Visual Studio Installer
2. 修改 Visual Studio 2022 安装
3. 确保选中 "MSVC v143 - VS 2022 C++ x64/x86 生成工具"

### 找不到 Windows SDK

**错误**: 扩展生成降级配置，SDK 路径注释显示

**解决方案**:
1. 打开 Visual Studio Installer
2. 修改 Visual Studio 2022 安装
3. 确保选中 "Windows 11 SDK" 或 "Windows 10 SDK"

## 项目结构

```
zed-msvc-toolkit/
├── src/
│   ├── cmake/
│   │   ├── compile_db.rs    # compile_commands.json 探测
│   │   ├── tasks.rs         # 任务文件生成 (V0.4)
│   │   └── tools.rs         # CMake 工具探测
│   ├── environment/
│   │   ├── msvc.rs          # MSVC 工具链探测
│   │   ├── vswhere.rs       # vswhere.exe 调用
│   │   ├── windows_sdk.rs   # Windows SDK 探测
│   │   └── tools.rs         # 工具查找辅助
│   ├── lsp/
│   │   ├── clangd_config.rs # .clangd 配置生成
│   │   └── server.rs        # 语言服务器启动
│   ├── debug/               # 调试支持 (V0.5 待实现)
│   ├── error.rs             # 错误类型
│   ├── lib.rs               # 扩展入口
│   └── paths.rs             # 路径处理
├── docs/
│   ├── USAGE.md             # 本文档
│   ├── TESTING.md           # 测试指南
│   └── zed-tasks-example.json # 任务文件示例
├── extension.toml           # 扩展清单
├── Cargo.toml               # Rust 项目配置
└── README.md                # 项目说明
```

## 版本历史

- **V0.1** (2026-05-20): MSVC 环境探测和 clangd 配置
- **V0.2** (2026-05-20): 编译数据库支持
- **V0.3** (2026-05-20): CMake 命令生成基础设施
- **V0.4** (2026-05-20): 任务系统集成
- **V0.5** (计划): DAP 调试支持
- **V1.0** (计划): 零配置调试

## 许可证

MIT License
