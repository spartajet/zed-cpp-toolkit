# V0.1 MSVC clangd 智能感知实现计划

> **给自动化执行代理：** 必须使用 `superpowers:subagent-driven-development`（推荐）或 `superpowers:executing-plans` 按任务逐步执行本计划。步骤使用 checkbox（`- [ ]`）语法跟踪。

**目标：** 把当前 Rust crate 改造成 Zed 扩展，并实现 Windows + Visual Studio 2022+ + MSVC 项目的 clangd 智能感知 MVP。

**架构：** 先建立面向 V1 的模块边界，但 V0.1 只实现环境探测、`.clangd` 渲染和 clangd 启动命令。业务逻辑保持在可单元测试的纯 Rust 模块中，Zed API 适配只留在 `src/lib.rs` 和 `src/lsp/server.rs`。

**技术栈：** Rust 2024、`zed_extension_api`、Zed Rust extension、clangd、Visual Studio 2022+、MSVC、Windows SDK。

---

## 文件结构

V0.1 计划创建或修改以下文件：

- 修改 `Cargo.toml`：添加 `zed_extension_api`，配置 `cdylib`。
- 创建 `extension.toml`：声明 Zed 扩展和 C/C++ language server。
- 替换 `src/lib.rs`：注册扩展，转发 `language_server_command`。
- 创建 `src/error.rs`：定义用户可读错误类型和 `Result` 别名。
- 创建 `src/paths.rs`：版本排序和 Windows 路径格式化。
- 创建 `src/environment/mod.rs`：导出环境探测模块。
- 创建 `src/environment/vswhere.rs`：封装 `vswhere.exe` 常量和输出解析。
- 创建 `src/environment/msvc.rs`：选择最高 MSVC toolset include 目录。
- 创建 `src/environment/windows_sdk.rs`：选择 Windows SDK include 目录，支持降级。
- 创建 `src/environment/tools.rs`：封装 `clangd` 探测。
- 创建 `src/lsp/mod.rs`：导出 LSP 模块。
- 创建 `src/lsp/clangd_config.rs`：渲染 `.clangd` 内容。
- 创建 `src/lsp/server.rs`：构造 clangd 启动命令。
- 创建 `src/cmake/mod.rs`：保留后续 CMake 模块边界。
- 创建 `src/debug/mod.rs`：保留后续 DAP 模块边界。

---

### Task 1: 建立 Zed 扩展骨架

**Files:**
- Modify: `Cargo.toml`
- Create: `extension.toml`
- Modify: `src/lib.rs`
- Create: `src/cmake/mod.rs`
- Create: `src/debug/mod.rs`

- [ ] **Step 1: 写入扩展基础配置**

将 `Cargo.toml` 改为：

```toml
[package]
name = "zed-msvc-toolkit"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
zed_extension_api = "0.6.0"
```

创建 `extension.toml`：

```toml
id = "zed-msvc-toolkit"
name = "Zed MSVC C++ Assistant"
description = "MSVC and clangd assistant for Windows C++ CMake projects in Zed."
version = "0.1.0"
schema_version = 1
authors = ["XRZB"]
repository = "https://github.com/XRZB/zed-msvc-toolkit"

[language_servers.msvc-cpp-clangd]
name = "MSVC clangd"
languages = ["C", "C++"]
```

- [ ] **Step 2: 创建最小扩展入口和预留模块**

将 `src/lib.rs` 替换为：

```rust
use zed_extension_api as zed;

mod cmake;
mod debug;

#[derive(Default)]
struct MsvcToolkitExtension;

impl zed::Extension for MsvcToolkitExtension {}

zed::register_extension!(MsvcToolkitExtension);
```

创建 `src/cmake/mod.rs`：

```rust
//! CMake 集成模块边界。
//!
//! V0.1 不实现 CMake configure/build 命令。
```

创建 `src/debug/mod.rs`：

```rust
//! Debug adapter 集成模块边界。
//!
//! V0.1 不注册或启动 vsdbg。
```

- [ ] **Step 3: 运行基础编译检查**

Run:

```powershell
cargo check
```

Expected:

```text
Finished `dev` profile
```

如果 `zed_extension_api` 需要下载依赖而网络被沙箱限制，按权限规则重新运行同一命令并请求提升权限。

- [ ] **Step 4: 提交扩展骨架**

Run:

```powershell
git add Cargo.toml extension.toml src/lib.rs src/cmake/mod.rs src/debug/mod.rs
git commit -m "chore: scaffold Zed extension"
```

---

### Task 2: 实现路径与错误基础模块

**Files:**
- Create: `src/error.rs`
- Create: `src/paths.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: 为版本排序写失败测试**

创建 `src/paths.rs`，先写测试和函数签名：

```rust
use std::cmp::Ordering;

pub fn compare_version_like(left: &str, right: &str) -> Ordering {
    version_parts(left).cmp(&version_parts(right))
}

pub fn highest_version_dir<'a>(dirs: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    dirs.into_iter().max_by(|left, right| compare_version_like(left, right))
}

pub fn clangd_include_arg(path: &str) -> String {
    format!("/I{}", path.replace('\\', "/"))
}

fn version_parts(value: &str) -> Vec<u32> {
    value
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<u32>().unwrap_or(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_highest_numeric_version() {
        let selected = highest_version_dir([
            "14.9.99999",
            "14.38.33130",
            "14.40.33807",
            "14.10.25017",
        ]);

        assert_eq!(selected, Some("14.40.33807"));
    }

    #[test]
    fn formats_windows_include_path_for_clangd() {
        let arg = clangd_include_arg(
            r"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.40.33807\include",
        );

        assert_eq!(
            arg,
            "/IC:/Program Files/Microsoft Visual Studio/2022/Community/VC/Tools/MSVC/14.40.33807/include"
        );
    }
}
```

- [ ] **Step 2: 添加错误类型**

创建 `src/error.rs`：

```rust
use std::fmt;

pub type ToolkitResult<T> = Result<T, ToolkitError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolkitError {
    UnsupportedLanguageServer(String),
    MissingVswhere,
    MissingVisualStudio,
    MissingMsvcToolset,
    MissingClangd,
    IoMessage(String),
}

impl ToolkitError {
    pub fn user_message(&self) -> String {
        match self {
            Self::UnsupportedLanguageServer(id) => {
                format!("不支持的 language server: {id}")
            }
            Self::MissingVswhere => {
                "找不到 vswhere.exe。请确认已安装 Visual Studio Installer 和 Visual Studio 2022+。".to_string()
            }
            Self::MissingVisualStudio => {
                "找不到 Visual Studio 2022+。请安装 Visual Studio 2022 或更新版本，并包含“使用 C++ 的桌面开发”工作负载。".to_string()
            }
            Self::MissingMsvcToolset => {
                "找不到 MSVC v143+ toolset。请在 Visual Studio Installer 中安装 MSVC C++ build tools。".to_string()
            }
            Self::MissingClangd => {
                "找不到 clangd。请安装 LLVM，或将 clangd.exe 加入 PATH。".to_string()
            }
            Self::IoMessage(message) => message.clone(),
        }
    }
}

impl fmt::Display for ToolkitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.user_message())
    }
}
```

- [ ] **Step 3: 在入口模块声明新模块**

将 `src/lib.rs` 改为：

```rust
use zed_extension_api as zed;

mod cmake;
mod debug;
mod error;
mod paths;

#[derive(Default)]
struct MsvcToolkitExtension;

impl zed::Extension for MsvcToolkitExtension {}

zed::register_extension!(MsvcToolkitExtension);
```

- [ ] **Step 4: 运行测试**

Run:

```powershell
cargo test paths
```

Expected:

```text
test result: ok. 2 passed
```

- [ ] **Step 5: 提交基础模块**

Run:

```powershell
git add src/lib.rs src/error.rs src/paths.rs
git commit -m "feat: add toolkit path and error helpers"
```

---

### Task 3: 实现 `.clangd` 渲染模块

**Files:**
- Create: `src/lsp/mod.rs`
- Create: `src/lsp/clangd_config.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: 写 `.clangd` 渲染测试和实现**

创建 `src/lsp/mod.rs`：

```rust
pub mod clangd_config;
pub mod server;
```

创建 `src/lsp/clangd_config.rs`：

```rust
use crate::paths::clangd_include_arg;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClangdConfigInput {
    pub msvc_include: String,
    pub sdk_includes: Vec<String>,
}

pub fn render_clangd_config(input: &ClangdConfigInput) -> String {
    let mut output = String::new();
    output.push_str("# 由 Zed MSVC C++ Assistant 自动生成。\n");
    output.push_str("# 如果需要自定义 clangd 行为，请编辑本文件；插件 V0.1 不会覆盖已有 .clangd。\n");
    output.push_str("CompileFlags:\n");
    output.push_str("  DriverMode: cl\n");
    output.push_str("  Add:\n");
    output.push_str(&format!("    - {}\n", clangd_include_arg(&input.msvc_include)));

    if input.sdk_includes.is_empty() {
        output.push_str("    # Windows SDK include 未自动探测到；如有需要，请手动添加 /I...\n");
        output.push_str("    # - /IC:/Program Files (x86)/Windows Kits/10/Include/<version>/ucrt\n");
        output.push_str("    # - /IC:/Program Files (x86)/Windows Kits/10/Include/<version>/um\n");
        output.push_str("    # - /IC:/Program Files (x86)/Windows Kits/10/Include/<version>/shared\n");
    } else {
        for include in &input.sdk_includes {
            output.push_str(&format!("    - {}\n", clangd_include_arg(include)));
        }
    }

    output.push_str("Diagnostics:\n");
    output.push_str("  Suppress: ['pp_file_not_found']\n");
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_msvc_and_sdk_include_paths() {
        let rendered = render_clangd_config(&ClangdConfigInput {
            msvc_include: r"C:\VS\VC\Tools\MSVC\14.40.33807\include".to_string(),
            sdk_includes: vec![
                r"C:\Windows Kits\10\Include\10.0.22621.0\ucrt".to_string(),
                r"C:\Windows Kits\10\Include\10.0.22621.0\um".to_string(),
                r"C:\Windows Kits\10\Include\10.0.22621.0\shared".to_string(),
            ],
        });

        assert!(rendered.contains("DriverMode: cl"));
        assert!(rendered.contains("- /IC:/VS/VC/Tools/MSVC/14.40.33807/include"));
        assert!(rendered.contains("- /IC:/Windows Kits/10/Include/10.0.22621.0/ucrt"));
        assert!(rendered.contains("- /IC:/Windows Kits/10/Include/10.0.22621.0/um"));
        assert!(rendered.contains("- /IC:/Windows Kits/10/Include/10.0.22621.0/shared"));
        assert!(!rendered.contains("Windows SDK include 未自动探测到"));
    }

    #[test]
    fn renders_manual_sdk_comments_when_sdk_is_missing() {
        let rendered = render_clangd_config(&ClangdConfigInput {
            msvc_include: r"C:\VS\VC\Tools\MSVC\14.40.33807\include".to_string(),
            sdk_includes: Vec::new(),
        });

        assert!(rendered.contains("- /IC:/VS/VC/Tools/MSVC/14.40.33807/include"));
        assert!(rendered.contains("Windows SDK include 未自动探测到"));
        assert!(rendered.contains("# - /IC:/Program Files (x86)/Windows Kits/10/Include/<version>/ucrt"));
    }
}
```

- [ ] **Step 2: 创建 server 模块占位**

创建 `src/lsp/server.rs`：

```rust
use zed_extension_api as zed;

pub fn clangd_args() -> Vec<String> {
    vec!["--header-insertion=never".to_string()]
}

pub fn build_clangd_command(command: String, env: Vec<(String, String)>) -> zed::Command {
    zed::Command {
        command,
        args: clangd_args(),
        env,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clangd_args_disable_header_insertion() {
        assert_eq!(clangd_args(), vec!["--header-insertion=never"]);
    }
}
```

- [ ] **Step 3: 在入口模块声明 LSP 模块**

将 `src/lib.rs` 改为：

```rust
use zed_extension_api as zed;

mod cmake;
mod debug;
mod error;
mod lsp;
mod paths;

#[derive(Default)]
struct MsvcToolkitExtension;

impl zed::Extension for MsvcToolkitExtension {}

zed::register_extension!(MsvcToolkitExtension);
```

- [ ] **Step 4: 运行 LSP 单元测试**

Run:

```powershell
cargo test lsp
```

Expected:

```text
test result: ok. 3 passed
```

- [ ] **Step 5: 提交 `.clangd` 渲染模块**

Run:

```powershell
git add src/lib.rs src/lsp
git commit -m "feat: render MSVC clangd config"
```

---

### Task 4: 实现 MSVC 和 Windows SDK 选择逻辑

**Files:**
- Create: `src/environment/mod.rs`
- Create: `src/environment/vswhere.rs`
- Create: `src/environment/msvc.rs`
- Create: `src/environment/windows_sdk.rs`
- Create: `src/environment/tools.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: 创建环境模块入口和数据结构**

创建 `src/environment/mod.rs`：

```rust
pub mod msvc;
pub mod tools;
pub mod vswhere;
pub mod windows_sdk;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsvcEnvironment {
    pub visual_studio_root: String,
    pub msvc_include: String,
    pub sdk_includes: Vec<String>,
}
```

- [ ] **Step 2: 写 `vswhere` 常量和输出解析**

创建 `src/environment/vswhere.rs`：

```rust
pub const VSWHERE_PATH: &str =
    r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe";

pub fn parse_installation_path(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_first_non_empty_installation_path() {
        let parsed = parse_installation_path("\r\nC:\\Program Files\\Microsoft Visual Studio\\2022\\Community\r\n");

        assert_eq!(
            parsed,
            Some("C:\\Program Files\\Microsoft Visual Studio\\2022\\Community".to_string())
        );
    }
}
```

- [ ] **Step 3: 写 MSVC include 选择逻辑**

创建 `src/environment/msvc.rs`：

```rust
use crate::paths::highest_version_dir;

pub fn select_msvc_include<'a>(versions: impl IntoIterator<Item = &'a str>, vs_root: &str) -> Option<String> {
    highest_version_dir(versions).map(|version| {
        format!(
            r"{vs_root}\VC\Tools\MSVC\{version}\include",
            vs_root = vs_root.trim_end_matches('\\'),
            version = version
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_highest_msvc_include_path() {
        let include = select_msvc_include(
            ["14.38.33130", "14.40.33807", "14.9.99999"],
            r"C:\Program Files\Microsoft Visual Studio\2022\Community",
        );

        assert_eq!(
            include,
            Some(r"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.40.33807\include".to_string())
        );
    }
}
```

- [ ] **Step 4: 写 Windows SDK include 选择逻辑**

创建 `src/environment/windows_sdk.rs`：

```rust
use crate::paths::highest_version_dir;

const SDK_INCLUDE_KINDS: [&str; 3] = ["ucrt", "um", "shared"];

pub fn select_windows_sdk_includes<'a>(
    versions: impl IntoIterator<Item = &'a str>,
    kits_include_root: &str,
) -> Vec<String> {
    let Some(version) = highest_version_dir(versions) else {
        return Vec::new();
    };

    SDK_INCLUDE_KINDS
        .iter()
        .map(|kind| {
            format!(
                r"{root}\{version}\{kind}",
                root = kits_include_root.trim_end_matches('\\'),
                version = version,
                kind = kind
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_highest_sdk_include_group() {
        let includes = select_windows_sdk_includes(
            ["10.0.19041.0", "10.0.22621.0"],
            r"C:\Program Files (x86)\Windows Kits\10\Include",
        );

        assert_eq!(
            includes,
            vec![
                r"C:\Program Files (x86)\Windows Kits\10\Include\10.0.22621.0\ucrt",
                r"C:\Program Files (x86)\Windows Kits\10\Include\10.0.22621.0\um",
                r"C:\Program Files (x86)\Windows Kits\10\Include\10.0.22621.0\shared",
            ]
        );
    }

    #[test]
    fn returns_empty_includes_when_sdk_versions_are_missing() {
        let includes = select_windows_sdk_includes([], r"C:\Program Files (x86)\Windows Kits\10\Include");

        assert!(includes.is_empty());
    }
}
```

- [ ] **Step 5: 写工具探测封装**

创建 `src/environment/tools.rs`：

```rust
use crate::error::{ToolkitError, ToolkitResult};

pub fn require_clangd(clangd_path: Option<String>) -> ToolkitResult<String> {
    clangd_path.ok_or(ToolkitError::MissingClangd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_existing_clangd_path() {
        let path = require_clangd(Some(r"C:\LLVM\bin\clangd.exe".to_string()));

        assert_eq!(path, Ok(r"C:\LLVM\bin\clangd.exe".to_string()));
    }

    #[test]
    fn reports_missing_clangd() {
        let error = require_clangd(None).unwrap_err();

        assert_eq!(error, ToolkitError::MissingClangd);
    }
}
```

- [ ] **Step 6: 在入口模块声明 environment**

将 `src/lib.rs` 改为：

```rust
use zed_extension_api as zed;

mod cmake;
mod debug;
mod environment;
mod error;
mod lsp;
mod paths;

#[derive(Default)]
struct MsvcToolkitExtension;

impl zed::Extension for MsvcToolkitExtension {}

zed::register_extension!(MsvcToolkitExtension);
```

- [ ] **Step 7: 运行环境模块测试**

Run:

```powershell
cargo test environment
```

Expected:

```text
test result: ok. 7 passed
```

- [ ] **Step 8: 提交环境选择逻辑**

Run:

```powershell
git add src/lib.rs src/environment
git commit -m "feat: add MSVC environment selectors"
```

---

### Task 5: 接入 Zed language_server_command

**Files:**
- Modify: `src/lib.rs`
- Modify: `src/lsp/server.rs`

- [ ] **Step 1: 扩展 server 模块，集中处理 language server ID 和 clangd 命令**

将 `src/lsp/server.rs` 替换为：

```rust
use crate::error::{ToolkitError, ToolkitResult};
use crate::environment::tools::require_clangd;
use zed_extension_api as zed;

pub const LANGUAGE_SERVER_ID: &str = "msvc-cpp-clangd";

pub fn clangd_args() -> Vec<String> {
    vec!["--header-insertion=never".to_string()]
}

pub fn validate_language_server_id(id: &str) -> ToolkitResult<()> {
    if id == LANGUAGE_SERVER_ID {
        Ok(())
    } else {
        Err(ToolkitError::UnsupportedLanguageServer(id.to_string()))
    }
}

pub fn build_clangd_command(command: String, env: Vec<(String, String)>) -> zed::Command {
    zed::Command {
        command,
        args: clangd_args(),
        env,
    }
}

pub fn command_from_worktree(worktree: &zed::Worktree) -> ToolkitResult<zed::Command> {
    let clangd = require_clangd(worktree.which("clangd"))?;
    Ok(build_clangd_command(clangd, worktree.shell_env()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clangd_args_disable_header_insertion() {
        assert_eq!(clangd_args(), vec!["--header-insertion=never"]);
    }

    #[test]
    fn accepts_expected_language_server_id() {
        assert_eq!(validate_language_server_id("msvc-cpp-clangd"), Ok(()));
    }

    #[test]
    fn rejects_unexpected_language_server_id() {
        let error = validate_language_server_id("other-lsp").unwrap_err();

        assert_eq!(
            error,
            ToolkitError::UnsupportedLanguageServer("other-lsp".to_string())
        );
    }
}
```

- [ ] **Step 2: 在扩展入口中实现 `language_server_command`**

将 `src/lib.rs` 替换为：

```rust
use zed_extension_api as zed;

mod cmake;
mod debug;
mod environment;
mod error;
mod lsp;
mod paths;

#[derive(Default)]
struct MsvcToolkitExtension;

impl zed::Extension for MsvcToolkitExtension {
    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        lsp::server::validate_language_server_id(language_server_id.as_ref())
            .map_err(|error| error.user_message())?;

        lsp::server::command_from_worktree(worktree).map_err(|error| error.user_message())
    }
}

zed::register_extension!(MsvcToolkitExtension);
```

- [ ] **Step 3: 运行 LSP server 测试**

Run:

```powershell
cargo test lsp::server
```

Expected:

```text
test result: ok. 3 passed
```

- [ ] **Step 4: 运行完整编译检查**

Run:

```powershell
cargo check
```

Expected:

```text
Finished `dev` profile
```

如果 `LanguageServerId` 没有 `as_ref()`，把该行改为当前 API 支持的字符串转换方式：

```rust
lsp::server::validate_language_server_id(language_server_id.as_str())
    .map_err(|error| error.user_message())?;
```

然后重新运行 `cargo check`。

- [ ] **Step 5: 提交 language server 接入**

Run:

```powershell
git add src/lib.rs src/lsp/server.rs
git commit -m "feat: launch clangd for MSVC projects"
```

---

### Task 6: 实现 V0.1 的 `.clangd` 生成决策层

**Files:**
- Create: `src/lsp/workspace_config.rs`
- Modify: `src/lsp/mod.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: 写 `.clangd` 生成决策模块**

创建 `src/lsp/workspace_config.rs`：

```rust
use crate::lsp::clangd_config::{render_clangd_config, ClangdConfigInput};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClangdFileDecision {
    Create { path: String, contents: String },
    PreserveExisting { path: String },
}

pub fn decide_clangd_file(
    root_path: &str,
    existing_contents: Option<String>,
    input: &ClangdConfigInput,
) -> ClangdFileDecision {
    let path = format!("{}/.clangd", root_path.replace('\\', "/").trim_end_matches('/'));

    if existing_contents.is_some() {
        ClangdFileDecision::PreserveExisting { path }
    } else {
        ClangdFileDecision::Create {
            path,
            contents: render_clangd_config(input),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> ClangdConfigInput {
        ClangdConfigInput {
            msvc_include: r"C:\VS\VC\Tools\MSVC\14.40.33807\include".to_string(),
            sdk_includes: Vec::new(),
        }
    }

    #[test]
    fn creates_config_when_file_is_missing() {
        let decision = decide_clangd_file(r"C:\repo", None, &input());

        match decision {
            ClangdFileDecision::Create { path, contents } => {
                assert_eq!(path, "C:/repo/.clangd");
                assert!(contents.contains("DriverMode: cl"));
            }
            ClangdFileDecision::PreserveExisting { .. } => panic!("expected create decision"),
        }
    }

    #[test]
    fn preserves_existing_config() {
        let decision = decide_clangd_file(r"C:\repo", Some("CompileFlags: {}".to_string()), &input());

        assert_eq!(
            decision,
            ClangdFileDecision::PreserveExisting {
                path: "C:/repo/.clangd".to_string()
            }
        );
    }
}
```

- [ ] **Step 2: 导出 workspace_config 模块**

将 `src/lsp/mod.rs` 改为：

```rust
pub mod clangd_config;
pub mod server;
pub mod workspace_config;
```

- [ ] **Step 3: 在 `src/lib.rs` 中添加保守读取钩子**

将 `language_server_command` 中的实现改为以下形态。此步骤只读取 `.clangd` 并计算决策，不写文件；如果当前 Zed API 后续确认支持写入，再在后续任务补写入适配。

```rust
impl zed::Extension for MsvcToolkitExtension {
    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        lsp::server::validate_language_server_id(language_server_id.as_ref())
            .map_err(|error| error.user_message())?;

        let _existing_clangd = worktree.read_text_file(".clangd").ok();

        lsp::server::command_from_worktree(worktree).map_err(|error| error.user_message())
    }
}
```

如果 `LanguageServerId` 转字符串接口已在 Task 5 中调整过，本步骤保持同样写法，不来回切换。

- [ ] **Step 4: 运行 `.clangd` 决策测试**

Run:

```powershell
cargo test workspace_config
```

Expected:

```text
test result: ok. 2 passed
```

- [ ] **Step 5: 运行完整测试**

Run:

```powershell
cargo test
```

Expected:

```text
test result: ok.
```

- [ ] **Step 6: 提交 `.clangd` 生成策略**

Run:

```powershell
git add src/lib.rs src/lsp/mod.rs src/lsp/workspace_config.rs
git commit -m "feat: add conservative clangd config policy"
```

---

### Task 7: V0.1 收尾验证与说明

**Files:**
- Modify: `docs/plugin-requirements.md`
- Create: `docs/v0.1-usage.md`

- [ ] **Step 1: 添加 V0.1 使用说明**

创建 `docs/v0.1-usage.md`：

```markdown
# V0.1 使用说明

## 适用范围

V0.1 只覆盖 Windows + Visual Studio 2022+ + MSVC 项目的 clangd 智能感知路径。

V0.1 不实现：

- CMake configure/build 命令
- `compile_commands.json` 自动协同
- DAP 调试
- `vsdbg.exe` 下载
- 已有 `.clangd` 的 YAML 合并

## 依赖

- Windows 10/11
- Visual Studio 2022 或更新版本
- “使用 C++ 的桌面开发”工作负载
- MSVC v143+ build tools
- Windows SDK
- PATH 中可找到 `clangd.exe`

## `.clangd` 策略

V0.1 采用保守策略：

- 工作区没有 `.clangd` 时，扩展准备生成 MSVC 兼容配置。
- 工作区已有 `.clangd` 时，扩展不覆盖用户配置。
- Windows SDK 探测失败时，生成内容会包含手动补充 SDK include 的注释。

## clangd 参数

V0.1 启动 clangd 时会加入：

```text
--header-insertion=never
```
```

- [ ] **Step 2: 在需求文档中加入 V0.1 状态链接**

在 `docs/plugin-requirements.md` 末尾追加：

```markdown
---

## V0.1 实施状态

V0.1 的设计与实施计划已拆分到：

- `docs/superpowers/specs/2026-05-20-zed-msvc-toolkit-design.md`
- `docs/superpowers/plans/2026-05-20-v0-1-msvc-clangd.md`
- `docs/v0.1-usage.md`
```

- [ ] **Step 3: 运行最终验证**

Run:

```powershell
cargo test
cargo check
git status --short
```

Expected:

```text
test result: ok.
Finished `dev` profile
```

`git status --short` 只应显示本任务修改的文档文件，直到下一步提交。

- [ ] **Step 4: 提交文档收尾**

Run:

```powershell
git add docs/plugin-requirements.md docs/v0.1-usage.md
git commit -m "docs: describe V0.1 usage"
```

---

## 自审结果

- 规格覆盖：阶段 0 和阶段 1 均有任务覆盖；阶段 2-5 只保留模块边界，不在 V0.1 实现。
- 占位扫描：计划中没有 `TBD` 或未定义的“以后补”步骤；V0.1 不写入工作区文件的 API 风险已在 Task 6 明确降级为读取和决策层。
- 类型一致性：`ToolkitError`、`ClangdConfigInput`、`ClangdFileDecision`、`LANGUAGE_SERVER_ID` 在首次使用前均有定义。
- 风险说明：`LanguageServerId` 字符串转换可能因 Zed API 版本不同需要调整，Task 5 给出唯一调整点和重新验证命令。
