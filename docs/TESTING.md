# Zed MSVC C++ Assistant - 测试指南

## 运行单元测试

```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test --lib cmake::tasks
cargo test --lib environment::msvc
cargo test --lib lsp::clangd_config

# 显示测试输出
cargo test -- --nocapture

# 运行特定测试
cargo test test_ninja_generator_produces_correct_args
```

## 测试覆盖

### CMake 模块测试

**文件**: `src/cmake/tools.rs`

- `ninja_generator_produces_correct_args` - Ninja 生成器参数
- `visual_studio_generator_produces_correct_args` - VS 生成器参数
- `build_type_produces_correct_cmake_var` - 构建类型变量
- `build_type_produces_correct_build_arg` - 构建参数
- `configure_command_for_ninja` - Ninja configure 命令
- `configure_command_for_visual_studio` - VS configure 命令
- `build_command_includes_config` - build 命令格式
- `configure_command_arguments_are_separate` - 参数分离验证
- `source_dir_with_spaces_is_separate_argument` - 路径空格处理

**文件**: `src/cmake/tasks.rs`

- `generate_tasks_json_creates_valid_json` - JSON 格式验证
- `tasks_include_configure_and_build` - 任务完整性
- `tasks_use_workspace_root_variable` - 变量使用
- `custom_build_dir_and_type` - 自定义配置

**文件**: `src/cmake/compile_db.rs`

- `find_compile_commands_in_root` - 根目录探测
- `find_compile_commands_in_build_subdir` - build 子目录探测
- `returns_none_when_not_found` - 文件不存在处理
- `parent_directory_is_root` - 返回父目录
- `parent_directory_is_build_subdir` - 返回 build 子目录

### Environment 模块测试

**文件**: `src/environment/msvc.rs`

- `select_latest_toolset_version` - 工具链版本选择
- `select_toolset_from_directories` - 目录选择
- `empty_directory_list_returns_none` - 空列表处理
- `single_directory_is_selected` - 单目录处理
- `non_numeric_directories_are_ignored` - 非数字目录过滤

**文件**: `src/environment/windows_sdk.rs`

- `sdk_paths_with_all_components` - 完整 SDK 路径
- `sdk_paths_with_missing_shared` - 缺少 shared 组件
- `empty_sdk_directories_returns_none` - 空 SDK 处理
- `sdk_version_sorting` - 版本排序

### LSP 模块测试

**文件**: `src/lsp/clangd_config.rs`

- `generates_clangd_config_with_msvc_paths` - MSVC 路径配置
- `generates_fallback_config_without_sdk` - 无 SDK 降级
- `clangd_config_without_compile_db` - 无编译数据库
- `clangd_config_with_compile_db` - 有编译数据库
- `paths_with_spaces_are_quoted` - 路径引号处理
- `paths_without_spaces_are_not_quoted` - 无空格路径

## 集成测试

### 准备测试环境

1. 安装必要工具：
   ```bash
   # 检查 Visual Studio
   "C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe"

   # 检查 clangd
   clangd --version

   # 检查 CMake
   cmake --version
   ```

2. 创建测试 CMake 项目：
   ```bash
   mkdir test-cmake-project
   cd test-cmake-project
   cat > CMakeLists.txt << 'EOF'
   cmake_minimum_required(VERSION 3.15)
   project(TestProject)

   add_executable(main main.cpp)
   EOF

   cat > main.cpp << 'EOF'
   #include <iostream>

   int main() {
       std::cout << "Hello, MSVC!" << std::endl;
       return 0;
   }
   EOF
   ```

3. 在 Zed 中打开测试项目

### 测试步骤

#### 1. 语言服务器启动

1. 打开任意 `.c` 或 `.cpp` 文件
2. 打开 Zed 的 "Outline" 面板
3. 验证 clangd 正在运行（应有符号索引显示）

#### 2. CMake 任务运行

1. 复制任务文件：
   ```bash
   cp docs/zed-tasks-example.json .zed/tasks.json
   ```

2. 打开任务面板：`Ctrl+Shift+T`

3. 运行 "CMake: Configure (Debug)"

4. 验证 `build/` 目录生成

5. 运行 "CMake: Build (Debug)"

6. 验证可执行文件生成

#### 3. 编译数据库测试

1. 配置项目（如果尚未配置）：
   ```bash
   cmake -B build -DCMAKE_EXPORT_COMPILE_COMMANDS=ON
   ```

2. 验证 `build/compile_commands.json` 存在

3. 在 Zed 中打开 C++ 文件

4. 验证代码跳转和自动补全工作正常

## 手动验证清单

- [ ] clangd 在打开 C/C++ 文件时自动启动
- [ ] 头文件跳转（F12）工作正常
- [ ] 代码补全显示 MSVC 标准库符号
- [ ] 任务面板显示 CMake 任务
- [ ] CMake Configure 成功生成构建文件
- [ ] CMake Build 成功生成可执行文件
- [ ] `compile_commands.json` 被自动探测
- [ ] clangd 使用编译数据库进行代码分析

## 调试测试失败

### WASM 测试限制

单元测试无法在 WASM 目标上直接运行：
```bash
# 这会失败
cargo test --target wasm32-unknown-unknown
```

使用主机目标运行：
```bash
# 这会工作
cargo test
```

### 查看详细输出

```bash
# 显示测试输出
cargo test -- --nocapture

# 显示详细测试信息
cargo test -- --show-output

# 运行但忽略错误（查看所有结果）
cargo test -- --no-fail-fast
```

## 性能测试

### 测量扩展启动时间

1. 打开 Zed 日志：`Ctrl+Shift+P` → "Zed: Open Logs"
2. 搜索 "zed-msvc-toolkit" 相关消息
3. 检查语言服务器启动耗时

### 测量 clangd 索引时间

1. 打开大型 C++ 项目
2. 观察 clangd 日志中的索引进度
3. 记录完整索引耗时

## 持续集成

本地 CI 测试流程：
```bash
# 格式检查
cargo fmt -- --check

# Clippy 检查
cargo clippy -- -D warnings

# 单元测试
cargo test

# WASM 编译检查
cargo build --target wasm32-unknown-unknown --release
```

## 报告问题

测试失败时，请包含：
1. Zed 版本
2. Windows 版本
3. Visual Studio 版本
4. 错误消息或日志
5. 复现步骤
6. 最小测试项目（如适用）
