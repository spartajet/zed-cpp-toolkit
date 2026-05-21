# neocmakelsp 集成测试笔记

测试日期: 2026-05-20

## 测试步骤

### 1. 构建扩展
```bash
cargo build
```
结果: ✅ 通过 (0 errors, 18 warnings)

### 2. 在 Zed 中测试

#### 手动测试清单:

1. **CMake LSP 激活**
   - 打开一个 CMake 项目（包含 CMakeLists.txt）
   - 打开 CMakeLists.txt 文件
   - 检查 LSP 日志: `dev: open language server logs`
   - 预期: 看到 "msvc-cmake-neocmake" LSP 启动

2. **PATH 查找**
   - 如果 neocmakelsp 在 PATH 中
   - 预期: 日志显示 "在 PATH 中找到 neocmakelsp: ..."

3. **GitHub 下载回退**
   - 如果 neocmakelsp 不在 PATH 中
   - 预期: 日志显示 "PATH 中未找到 neocmakelsp，尝试下载"
   - 预期: 下载 `neocmakelsp/neocmakelsp` 最新 release 中匹配平台的资产
   - Windows 预期资产: `neocmakelsp-x86_64-pc-windows-msvc.zip`

4. **.neocmake.toml 配置**
   - 创建项目根目录的 `.neocmake.toml`:
     ```toml
     [format]
     enable = false

     [lint]
     enable = true
     ```
   - 预期: neocmakelsp 自己读取该文件
   - 预期: 扩展不会解析或合并 `.neocmake.toml`

5. **settings.json 覆盖**
   - 创建 `.zed/settings.json`:
     ```json
     {
       "lsp": {
         "msvc-cmake-neocmake": {
           "format": {
             "enable": true
           }
         }
       }
     }
     ```
   - 预期: 日志显示 "settings.json 覆盖: format.enable = true"

6. **clangd 仍然有效**
   - 打开 C/C++ 文件
   - 预期: clangd LSP 正常工作

## 测试结果

| 测试项 | 状态 |
|--------|------|
| 扩展编译 | ✅ 通过 |
| CMake LSP 激活 | ⏳ 待用户在 Zed 中测试 |
| PATH 查找 | ⏳ 待用户在 Zed 中测试 |
| GitHub 下载 | ⏳ 待用户在 Zed 中测试 |
| .neocmake.toml 配置 | ⏳ 待用户在 Zed 中测试 |
| settings.json 覆盖 | ⏳ 待用户在 Zed 中测试 |
| clangd 兼容性 | ⏳ 待用户在 Zed 中测试 |
| **CMake 语言识别** | ⏳ 待用户在 Zed 中测试 |

## V0.5.0 更新 (2026-05-20)

### 修复内容
1. **添加 CMake 语言定义**：创建 `languages/cmake/` 目录及配置文件
   - `config.toml` - 语言配置（匹配 CMakeLists.txt 文件）
   - `highlights.scm` - 语法高亮规则
   - `indents.scm` - 缩进规则
   - `injections.scm` - 注入规则
   - `textobjects.scm` - 文本对象规则

2. **修改 extension.toml**：
   - 将 `languages = ["cmake"]` 改为 `language = "CMake"`
   - 添加 `[grammars.cmake]` 条目用于 Tree-sitter 语法

### 测试步骤
1. 重新编译扩展：`cargo build --target wasm32-unknown-unknown --release`
2. 安装到 Zed 扩展目录
3. 打开 CMakeLists.txt 文件
4. **预期结果**：文件应被识别为 CMake 语言（不再是 "plain text"）
5. 检查 LSP 日志：`dev: open language server logs`

## 注意事项

- 扩展编译为 `target/debug/zed-msvc-toolkit.dll`
- 需要将此 DLL 复制到 Zed 扩展目录进行测试
- Zed 扩展目录通常位于 `%ZED_USER_EXTENSIONS_DIR%`
- neocmakelsp 优先使用 PATH 中的二进制；PATH 中不存在时自动下载 GitHub Release。
