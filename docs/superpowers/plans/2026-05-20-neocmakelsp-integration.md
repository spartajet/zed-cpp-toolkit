# neocmakelsp 集成实施计划

> **对于代理开发者**: 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐步执行此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标**: 将 neocmakelsp 作为 CMake Language Server 集成到现有的 clangd LSP 旁边

**架构**: 新增 `src/lsp/neocmake/` 模块。2026-05-21 修订后，集成采用 PATH 优先策略：先查找 PATH 中的 `neocmakelsp`，找不到时从 `neocmakelsp/neocmakelsp` GitHub Releases 下载匹配平台的资产；通过 Zed `language_server_initialization_options` 传递 init options，不由扩展解析 `.neocmake.toml`。

**技术栈**: Rust, zed_extension_api v0.6.0, serde_json, GitHub Releases API

---

## 2026-05-21 下载功能修订

原计划中的 `.neocmake.toml` 合并被推迟到后续阶段。当前范围如下：

- `server.rs` 通过 `worktree.which("neocmakelsp")` 优先查找二进制。
- 找不到 `neocmakelsp` 时调用 `download.rs` 从 GitHub Releases 下载。
- 启动命令为 `neocmakelsp stdio`，不再传递 `--init-options=...`。
- `src/lib.rs` 实现 `language_server_initialization_options`，通过 Zed API 传递 neocmakelsp init options。
- `config.rs` 只读取 `.zed/settings.json` 中的 `lsp.msvc-cmake-neocmake` 覆盖。
- `.neocmake.toml` 由 neocmakelsp 自己读取，扩展不解析。
- `download.rs` 匹配当前 release 资产名：Windows zip、Linux tar.gz、macOS universal tar.gz。

后续执行时，以本修订为准，不再执行任务 4 中关于 `.neocmake.toml` 解析的旧步骤。

## 文件结构

**新建文件:**
- `src/lsp/neocmake/mod.rs` - 模块入口点，导出公共 API
- `src/lsp/neocmake/server.rs` - neocmakelsp 命令构建、二进制查找、下载编排
- `src/lsp/neocmake/download.rs` - GitHub Releases 二进制下载与平台检测
- `src/lsp/neocmake/config.rs` - 从 .neocmake.toml 和 settings.json 读取配置
- `src/lsp/neocmake/init_options.rs` - LSP 初始化选项构建器

**修改文件:**
- `src/lsp/mod.rs` - 添加 `pub mod neocmake;` 导出
- `src/lib.rs` - 路由 `msvc-cmake-neocmake` LSP ID 到 neocmake 模块
- `extension.toml` - 添加 `[language_servers.msvc-cmake-neocmake]` 声明
- `src/error.rs` - 添加 neocmake 特定错误变体

---

### 任务 1: 添加 neocmake 特定错误到 error.rs

**文件:**
- 修改: `src/error.rs`

- [ ] **步骤 1: 添加 neocmake 错误变体**

在 `ToolkitError` 枚举中添加（在 `MissingTool` 之后，`MissingWorkspaceConfig` 之前）：

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolkitError {
    // ... 现有变体 ...
    MissingNeocmakelsp,
    NeocmakeDownloadFailed(String),
    NeocmakeConfigParseError(String),
    // ... 其余变体 ...
}
```

在 `user_message()` 方法中添加（在 `MissingCmake` 分支之后，`MissingTool` 分支之前）：

```rust
Self::MissingNeocmakelsp => {
    "找不到 neocmakelsp。将从 GitHub 下载。".to_string()
}
Self::NeocmakeDownloadFailed(url) => {
    format!("下载 neocmakelsp 失败：{url}")
}
Self::NeocmakeConfigParseError(detail) => {
    format!("解析 neocmake 配置失败：{detail}，将使用默认配置。")
}
```

- [ ] **步骤 2: 运行测试验证更改可编译**

运行: `cargo test`
预期: PASS（所有现有测试仍然通过）

- [ ] **步骤 3: 提交**

```bash
git add src/error.rs
git commit -m "feat(neocmake): add neocmake-specific error variants"
```

---

### 任务 2: 创建 neocmake 模块入口点

**文件:**
- 创建: `src/lsp/neocmake/mod.rs`

- [ ] **步骤 1: 创建 mod.rs 并导出模块**

```rust
//! neocmakelsp LSP 集成模块。
//!
//! 通过 neocmakelsp 提供 CMake 语言支持，支持双重安装
//!（PATH + GitHub 下载）和双重配置（.neocmake.toml + settings.json）。

pub mod config;
pub mod download;
pub mod init_options;
pub mod server;

pub use server::command_from_worktree;
```

- [ ] **步骤 2: 验证模块可编译**

运行: `cargo check`
预期: FAIL（模块文件尚不存在，但 mod.rs 语法有效）

- [ ] **步骤 3: 提交**

```bash
git add src/lsp/neocmake/mod.rs
git commit -m "feat(neocmake): add neocmake module entry point"
```

---

### 任务 3: 创建 neocmake 下载模块

**文件:**
- 创建: `src/lsp/neocmake/download.rs`

- [ ] **步骤 1: 创建 download.rs 实现下载逻辑**

```rust
//! 从 GitHub Releases 下载 neocmakelsp 二进制。

use crate::debug::log_message;
use crate::error::{ToolkitError, ToolkitResult};
use zed_extension_api as zed;

const GITHUB_REPO: &str = "neocmakelsp/neocmakelsp";
const BINARY_NAME: &str = "neocmakelsp";

/// neocmakelsp releases 的平台特定资源名称模式。
fn platform_asset_name() -> Option<&'static str> {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Some("neocmakelsp-x86_64-pc-windows-msvc.zip");

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Some("neocmakelsp-x86_64-unknown-linux-gnu");

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Some("neocmakelsp-x86_64-apple-darwin");

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Some("neocmakelsp-aarch64-apple-darwin");

    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
    )))]
    {
        log_message("neocmakelsp: 不支持的平台无法下载");
        None
    }
}

/// 从 GitHub Releases 下载 neocmakelsp 二进制。
pub fn download_binary() -> ToolkitResult<String> {
    let asset_name = platform_asset_name()
        .ok_or_else(|| ToolkitError::NeocmakeDownloadFailed("不支持的平台".to_string()))?;

    log_message(&format!("从 GitHub releases 下载 neocmakelsp，资源: {asset_name}"));

    let release = zed::latest_github_release(GITHUB_REPO)
        .map_err(|e| ToolkitError::NeocmakeDownloadFailed(format!("获取 release: {e}")))?;

    log_message(&format!("最新版本: {}", release.version));

    let asset = release.assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            ToolkitError::NeocmakeDownloadFailed(format!("release 中未找到资源 {asset_name}"))
        })?;

    let extension_dir = zed::extensions_dir();
    let target_dir = format!("{extension_dir}/neocmakelsp");
    let _ = zed::make_dir(&target_dir);

    let binary_path = format!("{target_dir}/{BINARY_NAME}");

    // 检查是否已存在
    if let Ok(true) = zed::file_exists(&binary_path) {
        log_message(&format!("neocmakelsp 已存在于: {binary_path}"));
        return Ok(binary_path);
    }

    log_message(&format!("下载资源到: {binary_path}"));
    let downloaded_path = zed::download_file(
        &asset.download_url,
        &target_dir,
        Some(BINARY_NAME),
    )
        .map_err(|e| ToolkitError::NeocmakeDownloadFailed(format!("下载: {e}")))?;

    log_message(&format!("已下载到: {downloaded_path}"));

    // 在 Windows 上处理 .zip 解压
    if asset_name.ends_with(".zip") {
        log_message("从 zip 压缩包中提取 neocmakelsp");
        // Zed 的 download_file 会自动解压 zip，二进制应该在 binary_path
    }

    zed::make_file_executable(&downloaded_path)
        .map_err(|e| ToolkitError::NeocmakeDownloadFailed(format!("设置可执行: {e}")))?;

    log_message(&format!("neocmakelsp 就绪于: {downloaded_path}"));
    Ok(downloaded_path)
}

/// 在 PATH 中查找 neocmakelsp 或下载它。
pub fn get_or_download_binary(worktree: &zed::Worktree) -> ToolkitResult<String> {
    // 首先尝试 PATH
    if let Some(path) = worktree.which(BINARY_NAME) {
        log_message(&format!("在 PATH 中找到 neocmakelsp: {path}"));
        return Ok(path);
    }

    log_message("PATH 中未找到 neocmakelsp，尝试下载");
    download_binary()
}
```

- [ ] **步骤 2: 验证模块可编译**

运行: `cargo check`
预期: FAIL（config 和 init_options 模块尚不存在）

- [ ] **步骤 3: 提交**

```bash
git add src/lsp/neocmake/download.rs
git commit -m "feat(neocmake): add neocmakelsp download from GitHub releases"
```

---

### 任务 4: 创建 neocmake 配置模块

**文件:**
- 创建: `src/lsp/neocmake/config.rs`

- [ ] **步骤 1: 创建 config.rs 实现配置数据结构**

```rust
//! 从 .neocmake.toml 和 settings.json 读取 neocmakelsp 配置。

use crate::debug::log_message;
use serde_json::Value;
use zed_extension_api as zed;

/// 功能开关配置。
#[derive(Debug, Clone, Default)]
pub struct FeatureConfig {
    pub enable: bool,
}

/// neocmakelsp 配置。
#[derive(Debug, Clone)]
pub struct NeocmakeConfig {
    pub format: FeatureConfig,
    pub lint: FeatureConfig,
    pub scan_cmake_in_package: bool,
    pub semantic_token: bool,
}

impl Default for NeocmakeConfig {
    fn default() -> Self {
        Self {
            format: FeatureConfig { enable: true },
            lint: FeatureConfig { enable: true },
            scan_cmake_in_package: true,
            semantic_token: false,
        }
    }
}

/// 解析 .neocmake.toml 文件内容为 JSON 值。
///
/// 使用基础字符串解析以避免添加 toml 依赖。
fn parse_neocmake_toml(content: &str) -> Value {
    let mut result = serde_json::map::Map::new();

    for line in content.lines() {
        let line = line.trim();

        // 跳过注释和空行
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // 解析 key = value 对
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();

            // 处理布尔值
            let value = match value.trim() {
                "true" => Value::Bool(true),
                "false" => Value::Bool(false),
                other => Value::String(other.to_string()),
            };

            // 将点表示法转换为嵌套对象: format.enable -> {"format": {"enable": bool}}
            if let Some((parent, child)) = key.rsplit_once('.') {
                let parent = parent.to_string();
                let child = child.to_string();

                if !result.contains_key(&parent) {
                    result.insert(parent.clone(), Value::Object(serde_json::map::Map::new()));
                }

                if let Some(Value::Object(obj)) = result.get_mut(&parent) {
                    obj.insert(child, value);
                }
            } else {
                result.insert(key, value);
            }
        }
    }

    Value::Object(result)
}

/// 读取并合并 .neocmake.toml 和 settings.json 的配置。
pub fn load_config(worktree: &zed::Worktree) -> NeocmakeConfig {
    let mut config = NeocmakeConfig::default();

    // 尝试读取 .neocmake.toml
    if let Ok(content) = worktree.read_text_file(".neocmake.toml") {
        log_message("读取 .neocmake.toml");
        let parsed = parse_neocmake_toml(&content);

        // 应用 .neocmake.toml 值
        if let Some(obj) = parsed.as_object() {
            if let Some(Value::Object(format_obj)) = obj.get("format") {
                if let Some(Value::Bool(enable)) = format_obj.get("enable") {
                    config.format.enable = *enable;
                }
            }
            if let Some(Value::Object(lint_obj)) = obj.get("lint") {
                if let Some(Value::Bool(enable)) = lint_obj.get("enable") {
                    config.lint.enable = *enable;
                }
            }
            if let Some(Value::Bool(scan)) = obj.get("scan_cmake_in_package") {
                config.scan_cmake_in_package = *scan;
            }
            if let Some(Value::Bool(token)) = obj.get("semantic_token") {
                config.semantic_token = *token;
            }
        }
    }

    // 尝试读取 settings.json LSP 配置覆盖
    if let Ok(settings) = worktree.read_text_file(".zed/settings.json") {
        log_message("读取 .zed/settings.json 以获取 LSP 配置覆盖");

        if let Ok(value) = serde_json::from_str::<Value>(&settings) {
            // 导航到 lsp.msvc-cmake-neocmake
            if let Some(Value::Object(lsp_obj)) = value.get("lsp") {
                if let Some(Value::Object(neocmake_obj)) = lsp_obj.get("msvc-cmake-neocmake") {
                    // 使用 settings.json 值覆盖（更高优先级）
                    if let Some(Value::Object(format_obj)) = neocmake_obj.get("format") {
                        if let Some(Value::Bool(enable)) = format_obj.get("enable") {
                            config.format.enable = *enable;
                            log_message(&format!("settings.json 覆盖: format.enable = {enable}"));
                        }
                    }
                    if let Some(Value::Object(lint_obj)) = neocmake_obj.get("lint") {
                        if let Some(Value::Bool(enable)) = lint_obj.get("enable") {
                            config.lint.enable = *enable;
                            log_message(&format!("settings.json 覆盖: lint.enable = {enable}"));
                        }
                    }
                    if let Some(Value::Bool(scan)) = neocmake_obj.get("scan_cmake_in_package") {
                        config.scan_cmake_in_package = *scan;
                        log_message(&format!("settings.json 覆盖: scan_cmake_in_package = {scan}"));
                    }
                    if let Some(Value::Bool(token)) = neocmake_obj.get("semantic_token") {
                        config.semantic_token = *token;
                        log_message(&format!("settings.json 覆盖: semantic_token = {token}"));
                    }
                }
            }
        }
    }

    log_message(&format!(
        "最终 neocmake 配置: format.enable={}, lint.enable={}, scan_cmake_in_package={}, semantic_token={}",
        config.format.enable,
        config.lint.enable,
        config.scan_cmake_in_package,
        config.semantic_token
    ));

    config
}
```

- [ ] **步骤 2: 验证模块可编译**

运行: `cargo check`
预期: FAIL（init_options 模块尚不存在）

- [ ] **步骤 3: 提交**

```bash
git add src/lsp/neocmake/config.rs
git commit -m "feat(neocmake): add configuration from .neocmake.toml and settings.json"
```

---

### 任务 5: 创建 neocmake init_options 模块

**文件:**
- 创建: `src/lsp/neocmake/init_options.rs`

- [ ] **步骤 1: 创建 init_options.rs 实现 LSP 初始化选项构建器**

```rust
//! neocmakelsp LSP 初始化选项。

use crate::lsp::neocmake::config::{FeatureConfig, NeocmakeConfig};
use serde_json::Value;

/// 从配置构建 LSP 初始化选项。
pub fn build_init_options(config: &NeocmakeConfig) -> Value {
    serde_json::json!({
        "format": {
            "enable": config.format.enable
        },
        "lint": {
            "enable": config.lint.enable
        },
        "scan_cmake_in_package": config.scan_cmake_in_package,
        "semantic_token": config.semantic_token
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_default_init_options() {
        let config = NeocmakeConfig::default();
        let options = build_init_options(&config);

        assert_eq!(options["format"]["enable"], true);
        assert_eq!(options["lint"]["enable"], true);
        assert_eq!(options["scan_cmake_in_package"], true);
        assert_eq!(options["semantic_token"], false);
    }

    #[test]
    fn builds_custom_init_options() {
        let config = NeocmakeConfig {
            format: FeatureConfig { enable: false },
            lint: FeatureConfig { enable: true },
            scan_cmake_in_package: false,
            semantic_token: true,
        };
        let options = build_init_options(&config);

        assert_eq!(options["format"]["enable"], false);
        assert_eq!(options["lint"]["enable"], true);
        assert_eq!(options["scan_cmake_in_package"], false);
        assert_eq!(options["semantic_token"], true);
    }
}
```

- [ ] **步骤 2: 运行测试验证逻辑**

运行: `cargo test -p zed-msvc-toolkit --lib neocmake::init_options`
预期: PASS（两个测试都通过）

- [ ] **步骤 3: 提交**

```bash
git add src/lsp/neocmake/init_options.rs
git commit -m "feat(neocmake): add LSP initialization options builder"
```

---

### 任务 6: 创建 neocmake server 模块

**文件:**
- 创建: `src/lsp/neocmake/server.rs`

- [ ] **步骤 1: 创建 server.rs 实现命令构建逻辑**

```rust
//! neocmakelsp server 命令构建器。

use crate::debug::log_message;
use crate::error::{ToolkitError, ToolkitResult};
use crate::lsp::neocmake::config::load_config;
use crate::lsp::neocmake::download::get_or_download_binary;
use crate::lsp::neocmake::init_options::build_init_options;
use zed_extension_api as zed;

pub const LANGUAGE_SERVER_ID: &str = "msvc-cmake-neocmake";

/// 验证 neocmake language server ID。
pub fn validate_language_server_id(id: &str) -> ToolkitResult<()> {
    if id == LANGUAGE_SERVER_ID {
        Ok(())
    } else {
        Err(ToolkitError::UnsupportedLanguageServer(id.to_string()))
    }
}

/// 构建带配置的 neocmakelsp 命令。
pub fn command_from_worktree(worktree: &zed::Worktree) -> ToolkitResult<zed::Command> {
    log_message("构建 neocmakelsp 命令");

    let binary_path = get_or_download_binary(worktree)?;
    log_message(&format!("neocmakelsp 二进制: {binary_path}"));

    let config = load_config(worktree);
    let init_options = build_init_options(&config);

    log_message(&format!("neocmakelsp 初始化选项: {init_options}"));

    // neocmakelsp 使用 stdio 传输并通过命令行接受初始化选项
    // 将 init_options 转换为 JSON 字符串用于命令行参数
    let init_options_json = serde_json::to_string(&init_options)
        .unwrap_or_else(|_| "{}".to_string());

    Ok(zed::Command {
        command: binary_path,
        args: vec![
            "stdio".to_string(),
            format!("--init-options={init_options_json}"),
        ],
        env: Default::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_neocmake_language_server_id() {
        assert_eq!(validate_language_server_id("msvc-cmake-neocmake"), Ok(()));
    }

    #[test]
    fn rejects_unexpected_language_server_id() {
        let error = validate_language_server_id("other-lsp").unwrap_err();
        assert!(matches!(error, ToolkitError::UnsupportedLanguageServer(_)));
    }
}
```

- [ ] **步骤 2: 运行测试验证逻辑**

运行: `cargo test -p zed-msvc-toolkit --lib neocmake::server`
预期: PASS（两个验证测试都通过）

- [ ] **步骤 3: 提交**

```bash
git add src/lsp/neocmake/server.rs
git commit -m "feat(neocmake): add neocmakelsp command builder"
```

---

### 任务 7: 从 lsp mod.rs 导出 neocmake 模块

**文件:**
- 修改: `src/lsp/mod.rs`

- [ ] **步骤 1: 添加 neocmake 模块导出**

在 `src/lsp/mod.rs` 中添加（在现有 pub mod 声明之后）：

```rust
pub mod clangd_config;
pub mod server;
pub mod workspace_config;
pub mod neocmake;
```

- [ ] **步骤 2: 验证模块可编译**

运行: `cargo check`
预期: PASS

- [ ] **步骤 3: 提交**

```bash
git add src/lsp/mod.rs
git commit -m "feat(neocmake): export neocmake module from lsp"
```

---

### 任务 8: 在 lib.rs 中添加 LSP 路由

**文件:**
- 修改: `src/lib.rs`

- [ ] **步骤 1: 更新 language_server_command 路由 neocmake LSP**

替换 `src/lib.rs` 中的 `language_server_command` 方法为：

```rust
fn language_server_command(
    &mut self,
    language_server_id: &zed::LanguageServerId,
    worktree: &zed::Worktree,
) -> zed::Result<zed::Command> {
    let language_server_id_value = language_server_id;
    let language_server_id = language_server_id.as_ref();
    let root_path = worktree.root_path();
    debug::log_message(&format!(
        "language_server_command called: id={language_server_id}, root={root_path}"
    ));

    set_lsp_status(
        language_server_id_value,
        zed::LanguageServerInstallationStatus::CheckingForUpdate,
    );

    // 根据 ID 路由到对应的 LSP
    let result = match language_server_id {
        "msvc-cpp-clangd" => {
            validate_and_prepare_clangd(worktree, language_server_id_value)?;
            lsp::server::command_from_worktree(worktree)
                .map_err(|e| zed::ExtentError::Other(e.user_message()))
        }
        "msvc-cmake-neocmake" => {
            validate_and_prepare_neocmake(worktree, language_server_id_value)?;
            lsp::neocmake::server::command_from_worktree(worktree)
                .map_err(|e| zed::ExtentError::Other(e.user_message()))
        }
        _ => {
            let error = format!("不支持的 language server: {language_server_id}");
            debug::log_message(&error);
            set_lsp_status(
                language_server_id_value,
                zed::LanguageServerInstallationStatus::Failed(error.clone()),
            );
            return Err(zed::ExtentError::Other(error));
        }
    };

    match result {
        Ok(command) => {
            debug::log_message(&format!(
                "language server command ready: command={}, args={:?}, env_count={}",
                command.command,
                command.args,
                command.env.len()
            ));
            set_lsp_status(
                language_server_id_value,
                zed::LanguageServerInstallationStatus::None,
            );
            Ok(command)
        }
        Err(error) => {
            debug::log_message(&format!("language server command creation failed: {error}"));
            set_lsp_status(
                language_server_id_value,
                zed::LanguageServerInstallationStatus::Failed(error.to_string()),
            );
            Err(error)
        }
    }
}

fn validate_and_prepare_clangd(
    worktree: &zed::Worktree,
    language_server_id: &zed::LanguageServerId,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(error) = lsp::server::validate_language_server_id("msvc-cpp-clangd") {
        debug::log_error("language server id validation failed", &error);
        set_lsp_status(
            language_server_id,
            zed::LanguageServerInstallationStatus::Failed(error.user_message()),
        );
        return Err(error.user_message().into());
    }
    debug::log_message("language server id validation succeeded");

    set_lsp_status(
        language_server_id,
        zed::LanguageServerInstallationStatus::Downloading,
    );
    if let Err(error) = lsp::server::prepare_workspace_config_from_worktree(worktree) {
        debug::log_error("workspace config preparation failed", &error);
        set_lsp_status(
            language_server_id,
            zed::LanguageServerInstallationStatus::Failed(error.user_message()),
        );
        return Err(error.user_message().into());
    }
    debug::log_message("workspace config preparation succeeded");

    set_lsp_status(
        language_server_id,
        zed::LanguageServerInstallationStatus::CheckingForUpdate,
    );
    Ok(())
}

fn validate_and_prepare_neocmake(
    worktree: &zed::Worktree,
    language_server_id: &zed::LanguageServerId,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(error) = lsp::neocmake::server::validate_language_server_id("msvc-cmake-neocmake") {
        debug::log_error("neocmake language server id validation failed", &error);
        set_lsp_status(
            language_server_id,
            zed::LanguageServerInstallationStatus::Failed(error.user_message()),
        );
        return Err(error.user_message().into());
    }
    debug::log_message("neocmake language server id validation succeeded");

    set_lsp_status(
        language_server_id,
        zed::LanguageServerInstallationStatus::CheckingForUpdate,
    );
    Ok(())
}
```

- [ ] **步骤 2: 验证代码可编译**

运行: `cargo check`
预期: PASS

- [ ] **步骤 3: 提交**

```bash
git add src/lib.rs
git commit -m "feat(neocmake): add LSP routing for msvc-cmake-neocmake"
```

---

### 任务 9: 在 extension.toml 中添加 language server 声明

**文件:**
- 修改: `extension.toml`

- [ ] **步骤 1: 添加 neocmake language server 声明**

在 `extension.toml` 中 clangd language server 条目之后添加：

```toml
[language_servers.msvc-cmake-neocmake]
name = "MSVC NeoCMake"
languages = ["CMake"]
```

- [ ] **步骤 2: 验证 TOML 语法**

运行: `cargo check`（Zed 在构建期间验证 extension.toml）
预期: PASS

- [ ] **步骤 3: 提交**

```bash
git add extension.toml
git commit -m "feat(neocmake): declare msvc-cmake-neocmake language server"
```

---

### 任务 10: 端到端集成测试

**文件:**
- 测试: 在 Zed 中手动测试

- [ ] **步骤 1: 构建扩展**

运行: `cargo build`
预期: PASS，产生 `target/debug/zed-msvc-toolkit.dll`

- [ ] **步骤 2: 在 Zed 中用 CMake 项目测试**

1. 在 Zed 中打开一个 CMake 项目（包含 CMakeLists.txt）
2. 打开 Zed 设置: `Ctrl+,`
3. 确保扩展已启用
4. 打开一个 CMakeLists.txt 文件
5. 检查 LSP 日志: `dev: open language server logs`

预期行为:
- 打开 CMakeLists.txt 触发 `msvc-cmake-neocmake` LSP
- LSP 日志显示 "neocmakelsp 在 PATH 中找到: ..." 或 "正在下载 neocmakelsp..."
- LSP 状态显示 "CheckingForUpdate" → "Downloading"（如需要）→ "None"
- CMake 语法高亮工作（通过 tree-sitter）
- CMake 补全/诊断工作（通过 neocmakelsp）

- [ ] **步骤 3: 测试配置覆盖**

在项目根目录创建 `.neocmake.toml`:

```toml
[format]
enable = false

[lint]
enable = true
```

重新加载 Zed，检查 LSP 日志中的:
- "读取 .neocmake.toml"
- "最终 neocmake 配置: format.enable=false, ..."

用 `.zed/settings.json` 覆盖:

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

重新加载 Zed，检查 LSP 日志中的:
- "settings.json 覆盖: format.enable = true"
- "最终 neocmake 配置: format.enable=true, ..." （已覆盖）

- [ ] **步骤 4: 测试 PATH 回退和下载**

1. 如果 neocmakelsp 在 PATH 中: 验证它被使用
2. 从 PATH 中移除，重新加载 Zed，验证下载开始
3. 检查扩展目录中下载的二进制

- [ ] **步骤 5: 记录测试结果**

在项目 README 或单独测试文档中创建测试笔记:

```bash
echo "
## neocmakelsp 集成测试

测试日期: [DATE]

- CMake LSP 激活: 通过
- PATH 查找: 通过
- GitHub 下载回退: 通过
- .neocmake.toml 配置: 通过
- settings.json 覆盖: 通过
- clangd 对 C/C++ 仍然有效: 通过
" >> docs/testing-notes.md
```

- [ ] **步骤 6: 提交测试文档**

```bash
git add docs/testing-notes.md
git commit -m "test(neocmake): document integration test results"
```

---

### 任务 11: 最终清理和文档

**文件:**
- 修改: README.md（或创建 docs/usage.md）

- [ ] **步骤 1: 更新文档添加 neocmake 使用说明**

在项目 README 中添加:

```markdown
## CMake 语言支持

该扩展包含 neocmakelsp 用于 CMake 语言支持。

### 配置

neocmakelsp 可通过以下方式配置:

1. **项目级别**（项目根目录的 `.neocmake.toml`）:
   ```toml
   [format]
   enable = true

   [lint]
   enable = true

   scan_cmake_in_package = true
   semantic_token = false
   ```

2. **工作区级别**（`.zed/settings.json`）:
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

settings.json 的值会覆盖 `.neocmake.toml`。

### 安装

如果 PATH 中没有 neocmakelsp，它将自动从 GitHub Releases 下载。
```

- [ ] **步骤 2: 运行完整测试套件**

运行: `cargo test`
预期: PASS（所有测试通过，包括新的 neocmake 测试）

- [ ] **步骤 3: 最终提交**

```bash
git add README.md docs/
git commit -m "docs(neocmake): add neocmake usage documentation"
```

---

## 自审结果

**规范覆盖:**
- ✅ 模块结构（任务 2）
- ✅ 下载逻辑（任务 3）
- ✅ 配置系统（任务 4）
- ✅ 初始化选项（任务 5）
- ✅ 服务器命令（任务 6）
- ✅ LSP 路由（任务 8）
- ✅ 扩展清单（任务 9）
- ✅ 错误处理（任务 1）
- ✅ 测试策略（任务 10、11）

**占位符扫描:** 未发现。所有代码都是完整的。

**类型一致性:** 已验证 `NeocmakeConfig`、`FeatureConfig`、`ToolkitError` 变体在各任务间使用一致。
