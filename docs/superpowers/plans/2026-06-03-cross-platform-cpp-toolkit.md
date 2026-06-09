# 跨平台 C++ Toolkit 实施计划

> **给 agentic workers：** REQUIRED SUB-SKILL：执行本计划时必须使用 `superpowers:subagent-driven-development`（推荐）或 `superpowers:executing-plans`。每个步骤都使用 checkbox（`- [ ]`）语法跟踪。

**目标：** 将当前 Windows/MSVC 专用扩展改造成配置驱动、preset 辅助的跨平台 `cpp-toolkit`，默认使用 `build` 构建目录，并始终生成 `.clangd`。

**架构：** 先建立独立的配置解析与 preset 合并层，再用该层驱动 `.clangd` 渲染和 `.zed/tasks.json` 生成。MSVC 检测逻辑暂时保留在 `src/environment/`，通过配置解析后的 MSVC preset 接入，避免一次性大搬迁。

**技术栈：** Rust 2024、`zed_extension_api`、`serde_json`、新增 `serde` 与 `toml`、Zed extension WASM crate。

---

## 文件结构

### 新建文件

- `src/config/mod.rs`：导出配置模块。
- `src/config/schema.rs`：定义用户配置、有效配置、preset 名称、build 目录风格等类型。
- `src/config/presets.rs`：定义内置 preset 默认配置。
- `src/config/merge.rs`：合并 preset、用户覆盖和默认值，并展开 `{build_dir}`、`{build_type}`。
- `src/config/loader.rs`：从 `zed::Worktree` 读取 `.zed/cpp-toolkit.toml`。
- `src/build/mod.rs`：导出通用 task 生成模块。
- `src/build/shell.rs`：根据平台包装命令字符串。
- `src/build/tasks.rs`：从有效配置生成 `.zed/tasks.json`。

### 修改文件

- `Cargo.toml`：添加 `serde` 和 `toml`。
- `extension.toml`：重命名扩展 ID、名称、描述和 language server ID；capabilities 从 MSVC 专用改为跨平台常用命令。
- `src/lib.rs`：注册新模块；把 clangd language server ID 改为 `cpp-toolkit-clangd`。
- `src/lsp/clangd_config.rs`：从 MSVC 专用输入改为通用输入，同时保留 MSVC include 支持。
- `src/lsp/workspace_config.rs`：适配新的 clangd 配置输入。
- `src/lsp/server.rs`：读取配置、解析 preset、生成 `.clangd` 和 `.zed/tasks.json`；保留 MSVC fallback。
- `README.md`：改成中文为主，说明 `cpp-toolkit` 配置和 preset。
- `docs/superpowers/specs/2026-06-03-cross-platform-cpp-toolkit-design.md`：如果实现过程中发现必要小修，保持中文更新。

### 暂不移动的文件

- `src/environment/msvc.rs`
- `src/environment/vswhere.rs`
- `src/environment/windows_sdk.rs`
- `src/environment/tools.rs`

这些文件在第一版继续作为 MSVC provider 的底层实现使用，避免把跨平台配置和 MSVC 目录搬迁混在同一批改动里。

---

## Task 1: 添加配置依赖和模块骨架

**文件：**
- 修改：`Cargo.toml`
- 修改：`src/lib.rs`
- 新建：`src/config/mod.rs`
- 新建：`src/config/schema.rs`
- 新建：`src/config/presets.rs`
- 新建：`src/config/merge.rs`
- 新建：`src/config/loader.rs`

- [ ] **Step 1: 修改依赖**

在 `Cargo.toml` 的 `[dependencies]` 中加入：

```toml
serde = { version = "1", features = ["derive"] }
toml = "0.8"
```

保留现有依赖：

```toml
serde_json = { version = "1", features = ["preserve_order"] }
zed_extension_api = "0.6.0"
```

- [ ] **Step 2: 注册配置模块**

在 `src/lib.rs` 顶部模块列表中加入：

```rust
mod config;
```

放在 `mod cmake;` 后面即可。

- [ ] **Step 3: 创建 `src/config/mod.rs`**

```rust
pub mod loader;
pub mod merge;
pub mod presets;
pub mod schema;
```

- [ ] **Step 4: 创建配置 schema 初版**

创建 `src/config/schema.rs`：

```rust
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct UserConfig {
    pub preset: Option<String>,
    #[serde(default)]
    pub toolchain: ToolchainConfig,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default)]
    pub run: RunConfig,
    #[serde(default)]
    pub clangd: ClangdConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct ToolchainConfig {
    pub name: Option<String>,
    pub cc: Option<String>,
    pub cxx: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct BuildConfig {
    pub system: Option<String>,
    pub build_dir_style: Option<BuildDirStyle>,
    pub build_dir: Option<String>,
    pub build_type: Option<String>,
    pub configure: Option<String>,
    pub build: Option<String>,
    pub clean: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct RunConfig {
    pub command: Option<String>,
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct ClangdConfig {
    pub command: Option<String>,
    pub compiler: Option<String>,
    pub compile_commands_dir: Option<String>,
    #[serde(default)]
    pub extra_flags: Vec<String>,
    #[serde(default)]
    pub query_driver: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuildDirStyle {
    Build,
    Clion,
    Custom,
}

impl Default for BuildDirStyle {
    fn default() -> Self {
        Self::Build
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveConfig {
    pub preset: String,
    pub toolchain: EffectiveToolchain,
    pub build: EffectiveBuild,
    pub run: EffectiveRun,
    pub clangd: EffectiveClangd,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveToolchain {
    pub name: String,
    pub cc: String,
    pub cxx: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveBuild {
    pub system: String,
    pub build_dir_style: BuildDirStyle,
    pub build_dir: String,
    pub build_type: String,
    pub configure: Option<String>,
    pub build: Option<String>,
    pub clean: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveRun {
    pub command: Option<String>,
    pub cwd: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveClangd {
    pub command: String,
    pub compiler: String,
    pub compile_commands_dir: String,
    pub extra_flags: Vec<String>,
    pub query_driver: Vec<String>,
}
```

- [ ] **Step 5: 创建空 preset/merge/loader 骨架**

`src/config/presets.rs`：

```rust
use crate::config::schema::UserConfig;

pub fn preset_config(name: &str) -> Option<UserConfig> {
    match name {
        "msvc-cmake-ninja" => Some(UserConfig::default()),
        "gcc-cmake-ninja" => Some(UserConfig::default()),
        "clang-cmake-ninja" => Some(UserConfig::default()),
        "gcc-make" => Some(UserConfig::default()),
        "clang-make" => Some(UserConfig::default()),
        "custom" => Some(UserConfig::default()),
        _ => None,
    }
}
```

`src/config/merge.rs`：

```rust
use crate::config::schema::{BuildDirStyle, EffectiveBuild, EffectiveClangd, EffectiveConfig, EffectiveRun, EffectiveToolchain, UserConfig};
use crate::error::{ToolkitError, ToolkitResult};

pub fn default_preset_for_current_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "msvc-cmake-ninja"
    } else if cfg!(target_os = "macos") {
        "clang-cmake-ninja"
    } else {
        "gcc-cmake-ninja"
    }
}

pub fn resolve_config(user: Option<UserConfig>) -> ToolkitResult<EffectiveConfig> {
    let preset = user
        .as_ref()
        .and_then(|config| config.preset.clone())
        .unwrap_or_else(|| default_preset_for_current_platform().to_string());

    Ok(EffectiveConfig {
        preset,
        toolchain: EffectiveToolchain {
            name: "custom".to_string(),
            cc: "cc".to_string(),
            cxx: "c++".to_string(),
        },
        build: EffectiveBuild {
            system: "custom".to_string(),
            build_dir_style: BuildDirStyle::Build,
            build_dir: "build".to_string(),
            build_type: "Debug".to_string(),
            configure: None,
            build: None,
            clean: None,
        },
        run: EffectiveRun {
            command: None,
            cwd: "$ZED_WORKTREE_ROOT".to_string(),
        },
        clangd: EffectiveClangd {
            command: "clangd".to_string(),
            compiler: "c++".to_string(),
            compile_commands_dir: "build".to_string(),
            extra_flags: Vec::new(),
            query_driver: Vec::new(),
        },
    })
}

pub fn parse_user_config(contents: &str) -> ToolkitResult<UserConfig> {
    toml::from_str(contents).map_err(|error| ToolkitError::IoMessage(format!("解析 .zed/cpp-toolkit.toml 失败：{error}")))
}
```

`src/config/loader.rs`：

```rust
use crate::config::merge::{parse_user_config, resolve_config};
use crate::config::schema::EffectiveConfig;
use crate::error::ToolkitResult;
use zed_extension_api as zed;

pub fn load_effective_config(worktree: &zed::Worktree) -> ToolkitResult<EffectiveConfig> {
    let user = match worktree.read_text_file(".zed/cpp-toolkit.toml") {
        Ok(contents) => Some(parse_user_config(&contents)?),
        Err(_) => None,
    };
    resolve_config(user)
}
```

- [ ] **Step 6: 运行检查**

运行：

```bash
cargo test --target x86_64-pc-windows-msvc
```

预期： 编译通过；现有测试可能全部 PASS。如果本机没有 MSVC host target，则运行：

```bash
cargo test
```

预期： PASS。

- [ ] **Step 7: 提交**

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/config
git commit -m "Add cpp toolkit config skeleton"
```

---

## Task 2: 实现 preset、build 目录风格和模板展开

**文件：**
- 修改：`src/config/presets.rs`
- 修改：`src/config/merge.rs`
- 修改：`src/config/schema.rs`

- [ ] **Step 1: 写 build 目录风格测试**

在 `src/config/merge.rs` 末尾添加：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{BuildConfig, BuildDirStyle, UserConfig};

    #[test]
    fn defaults_to_build_directory() {
        let config = resolve_config(Some(UserConfig {
            preset: Some("gcc-cmake-ninja".to_string()),
            ..UserConfig::default()
        }))
        .unwrap();

        assert_eq!(config.build.build_dir, "build");
        assert_eq!(config.clangd.compile_commands_dir, "build");
        assert!(config.build.configure.unwrap().contains("-B build"));
    }

    #[test]
    fn clion_style_is_only_used_when_explicit() {
        let config = resolve_config(Some(UserConfig {
            preset: Some("gcc-cmake-ninja".to_string()),
            build: BuildConfig {
                build_dir_style: Some(BuildDirStyle::Clion),
                build_type: Some("Release".to_string()),
                ..BuildConfig::default()
            },
            ..UserConfig::default()
        }))
        .unwrap();

        assert_eq!(config.build.build_dir, "cmake-build-release");
        assert_eq!(config.clangd.compile_commands_dir, "cmake-build-release");
        assert!(config.build.configure.unwrap().contains("-B cmake-build-release"));
    }

    #[test]
    fn explicit_build_dir_wins_over_style() {
        let config = resolve_config(Some(UserConfig {
            preset: Some("gcc-cmake-ninja".to_string()),
            build: BuildConfig {
                build_dir_style: Some(BuildDirStyle::Clion),
                build_dir: Some("out/debug".to_string()),
                ..BuildConfig::default()
            },
            ..UserConfig::default()
        }))
        .unwrap();

        assert_eq!(config.build.build_dir, "out/debug");
        assert_eq!(config.clangd.compile_commands_dir, "out/debug");
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

运行：

```bash
cargo test config::merge::tests -- --nocapture
```

预期： 至少 `defaults_to_build_directory` 失败，因为 preset 还未真正填充 configure 命令。

- [ ] **Step 3: 实现 preset 默认值**

用以下内容替换 `src/config/presets.rs`：

```rust
use crate::config::schema::{BuildConfig, BuildDirStyle, ClangdConfig, ToolchainConfig, UserConfig};

pub fn preset_config(name: &str) -> Option<UserConfig> {
    match name {
        "msvc-cmake-ninja" => Some(cmake_preset(name, "msvc", "cl", "cl", "clang-cl")),
        "gcc-cmake-ninja" => Some(cmake_preset(name, "gcc", "gcc", "g++", "g++")),
        "clang-cmake-ninja" => Some(cmake_preset(name, "clang", "clang", "clang++", "clang++")),
        "gcc-make" => Some(make_preset(name, "gcc", "gcc", "g++", "g++")),
        "clang-make" => Some(make_preset(name, "clang", "clang", "clang++", "clang++")),
        "custom" => Some(UserConfig {
            preset: Some("custom".to_string()),
            ..UserConfig::default()
        }),
        _ => None,
    }
}

fn cmake_preset(preset: &str, toolchain_name: &str, cc: &str, cxx: &str, clangd_compiler: &str) -> UserConfig {
    UserConfig {
        preset: Some(preset.to_string()),
        toolchain: ToolchainConfig {
            name: Some(toolchain_name.to_string()),
            cc: Some(cc.to_string()),
            cxx: Some(cxx.to_string()),
        },
        build: BuildConfig {
            system: Some("cmake".to_string()),
            build_dir_style: Some(BuildDirStyle::Build),
            build_type: Some("Debug".to_string()),
            configure: Some("cmake -S . -B {build_dir} -G Ninja -DCMAKE_BUILD_TYPE={build_type} -DCMAKE_EXPORT_COMPILE_COMMANDS=ON".to_string()),
            build: Some("cmake --build {build_dir}".to_string()),
            clean: Some("cmake --build {build_dir} --target clean".to_string()),
            ..BuildConfig::default()
        },
        clangd: ClangdConfig {
            command: Some("clangd".to_string()),
            compiler: Some(clangd_compiler.to_string()),
            compile_commands_dir: Some("{build_dir}".to_string()),
            query_driver: query_drivers(toolchain_name),
            ..ClangdConfig::default()
        },
        ..UserConfig::default()
    }
}

fn make_preset(preset: &str, toolchain_name: &str, cc: &str, cxx: &str, clangd_compiler: &str) -> UserConfig {
    UserConfig {
        preset: Some(preset.to_string()),
        toolchain: ToolchainConfig {
            name: Some(toolchain_name.to_string()),
            cc: Some(cc.to_string()),
            cxx: Some(cxx.to_string()),
        },
        build: BuildConfig {
            system: Some("make".to_string()),
            build: Some("make -j".to_string()),
            clean: Some("make clean".to_string()),
            ..BuildConfig::default()
        },
        clangd: ClangdConfig {
            command: Some("clangd".to_string()),
            compiler: Some(clangd_compiler.to_string()),
            compile_commands_dir: Some(".".to_string()),
            query_driver: query_drivers(toolchain_name),
            ..ClangdConfig::default()
        },
        ..UserConfig::default()
    }
}

fn query_drivers(toolchain_name: &str) -> Vec<String> {
    match toolchain_name {
        "gcc" => vec!["gcc".to_string(), "g++".to_string()],
        "clang" => vec!["clang".to_string(), "clang++".to_string()],
        _ => Vec::new(),
    }
}
```

- [ ] **Step 4: 实现合并与模板展开**

用以下内容替换 `src/config/merge.rs` 中 `resolve_config` 和相关 helper：

```rust
use crate::config::presets::preset_config;
use crate::config::schema::{BuildConfig, BuildDirStyle, ClangdConfig, EffectiveBuild, EffectiveClangd, EffectiveConfig, EffectiveRun, EffectiveToolchain, RunConfig, ToolchainConfig, UserConfig};
use crate::error::{ToolkitError, ToolkitResult};

pub fn default_preset_for_current_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "msvc-cmake-ninja"
    } else if cfg!(target_os = "macos") {
        "clang-cmake-ninja"
    } else {
        "gcc-cmake-ninja"
    }
}

pub fn resolve_config(user: Option<UserConfig>) -> ToolkitResult<EffectiveConfig> {
    let preset_name = user
        .as_ref()
        .and_then(|config| config.preset.clone())
        .unwrap_or_else(|| default_preset_for_current_platform().to_string());
    let preset = preset_config(&preset_name)
        .ok_or_else(|| ToolkitError::IoMessage(format!("未知 cpp-toolkit preset：{preset_name}")))?;
    let merged = merge_user_config(preset, user.unwrap_or_default());

    let build_type = merged.build.build_type.clone().unwrap_or_else(|| "Debug".to_string());
    let build_dir_style = merged.build.build_dir_style.unwrap_or_default();
    let build_dir = merged
        .build
        .build_dir
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| infer_build_dir(build_dir_style, &build_type));

    let toolchain_name = merged.toolchain.name.clone().unwrap_or_else(|| "custom".to_string());
    let cc = merged.toolchain.cc.clone().unwrap_or_else(|| "cc".to_string());
    let cxx = merged.toolchain.cxx.clone().unwrap_or_else(|| "c++".to_string());
    let clangd_compiler = merged.clangd.compiler.clone().unwrap_or_else(|| cxx.clone());
    let compile_commands_dir = expand_template(
        &merged
            .clangd
            .compile_commands_dir
            .clone()
            .unwrap_or_else(|| build_dir.clone()),
        &build_dir,
        &build_type,
    );

    Ok(EffectiveConfig {
        preset: preset_name,
        toolchain: EffectiveToolchain {
            name: toolchain_name,
            cc,
            cxx,
        },
        build: EffectiveBuild {
            system: merged.build.system.unwrap_or_else(|| "custom".to_string()),
            build_dir_style,
            build_dir: build_dir.clone(),
            build_type: build_type.clone(),
            configure: expand_optional(merged.build.configure, &build_dir, &build_type),
            build: expand_optional(merged.build.build, &build_dir, &build_type),
            clean: expand_optional(merged.build.clean, &build_dir, &build_type),
        },
        run: EffectiveRun {
            command: expand_optional(merged.run.command, &build_dir, &build_type),
            cwd: merged.run.cwd.unwrap_or_else(|| "$ZED_WORKTREE_ROOT".to_string()),
        },
        clangd: EffectiveClangd {
            command: merged.clangd.command.unwrap_or_else(|| "clangd".to_string()),
            compiler: clangd_compiler,
            compile_commands_dir,
            extra_flags: merged.clangd.extra_flags,
            query_driver: merged.clangd.query_driver,
        },
    })
}

pub fn parse_user_config(contents: &str) -> ToolkitResult<UserConfig> {
    toml::from_str(contents).map_err(|error| ToolkitError::IoMessage(format!("解析 .zed/cpp-toolkit.toml 失败：{error}")))
}

fn merge_user_config(mut base: UserConfig, user: UserConfig) -> UserConfig {
    base.preset = user.preset.or(base.preset);
    base.toolchain = merge_toolchain(base.toolchain, user.toolchain);
    base.build = merge_build(base.build, user.build);
    base.run = merge_run(base.run, user.run);
    base.clangd = merge_clangd(base.clangd, user.clangd);
    base
}

fn merge_toolchain(mut base: ToolchainConfig, user: ToolchainConfig) -> ToolchainConfig {
    base.name = user.name.or(base.name);
    base.cc = user.cc.or(base.cc);
    base.cxx = user.cxx.or(base.cxx);
    base
}

fn merge_build(mut base: BuildConfig, user: BuildConfig) -> BuildConfig {
    base.system = user.system.or(base.system);
    base.build_dir_style = user.build_dir_style.or(base.build_dir_style);
    base.build_dir = user.build_dir.or(base.build_dir);
    base.build_type = user.build_type.or(base.build_type);
    base.configure = user.configure.or(base.configure);
    base.build = user.build.or(base.build);
    base.clean = user.clean.or(base.clean);
    base
}

fn merge_run(mut base: RunConfig, user: RunConfig) -> RunConfig {
    base.command = user.command.or(base.command);
    base.cwd = user.cwd.or(base.cwd);
    base
}

fn merge_clangd(mut base: ClangdConfig, user: ClangdConfig) -> ClangdConfig {
    base.command = user.command.or(base.command);
    base.compiler = user.compiler.or(base.compiler);
    base.compile_commands_dir = user.compile_commands_dir.or(base.compile_commands_dir);
    if !user.extra_flags.is_empty() {
        base.extra_flags = user.extra_flags;
    }
    if !user.query_driver.is_empty() {
        base.query_driver = user.query_driver;
    }
    base
}

fn infer_build_dir(style: BuildDirStyle, build_type: &str) -> String {
    match style {
        BuildDirStyle::Build => "build".to_string(),
        BuildDirStyle::Clion => format!("cmake-build-{}", cmake_build_type_suffix(build_type)),
        BuildDirStyle::Custom => "build".to_string(),
    }
}

fn cmake_build_type_suffix(build_type: &str) -> String {
    match build_type {
        "Debug" => "debug".to_string(),
        "Release" => "release".to_string(),
        "RelWithDebInfo" => "relwithdebinfo".to_string(),
        "MinSizeRel" => "minsizerel".to_string(),
        other => other.to_lowercase(),
    }
}

fn expand_optional(value: Option<String>, build_dir: &str, build_type: &str) -> Option<String> {
    value.map(|value| expand_template(&value, build_dir, build_type))
}

fn expand_template(value: &str, build_dir: &str, build_type: &str) -> String {
    value
        .replace("{build_dir}", build_dir)
        .replace("{build_type}", build_type)
}
```

保留 Step 1 添加的 `#[cfg(test)] mod tests`。

- [ ] **Step 5: 运行配置测试**

运行：

```bash
cargo test config::merge::tests -- --nocapture
```

预期： PASS。

- [ ] **Step 6: 提交**

```bash
git add src/config
git commit -m "Resolve cpp toolkit presets"
```

---

## Task 3: 泛化 `.clangd` 渲染

**文件：**
- 修改：`src/lsp/clangd_config.rs`
- 修改：`src/lsp/workspace_config.rs`

- [ ] **Step 1: 添加通用 clangd 渲染测试**

在 `src/lsp/clangd_config.rs` 的测试模块中添加：

```rust
#[test]
fn renders_generic_clangd_config() {
    let rendered = render_clangd_config(&ClangdConfigInput {
        compiler: "g++".to_string(),
        compile_commands_dir: "build".to_string(),
        extra_flags: vec!["-std=c++20".to_string(), "-Iinclude".to_string()],
        msvc_include: None,
        sdk_includes: Vec::new(),
    });

    assert!(rendered.contains("# Auto-generated by Zed C++ Toolkit."));
    assert!(rendered.contains("CompilationDatabase: build"));
    assert!(rendered.contains("Compiler: g++"));
    assert!(rendered.contains("    - -std=c++20"));
    assert!(rendered.contains("    - -Iinclude"));
    assert!(!rendered.contains("Windows SDK include not auto-detected"));
}
```

- [ ] **Step 2: 运行测试确认失败**

运行：

```bash
cargo test lsp::clangd_config::tests::renders_generic_clangd_config -- --nocapture
```

预期： FAIL，因为 `ClangdConfigInput` 还没有通用字段。

- [ ] **Step 3: 修改 `ClangdConfigInput` 和渲染函数**

将 `src/lsp/clangd_config.rs` 中的 `ClangdConfigInput` 改为：

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClangdConfigInput {
    pub compiler: String,
    pub compile_commands_dir: String,
    pub extra_flags: Vec<String>,
    pub msvc_include: Option<String>,
    pub sdk_includes: Vec<String>,
}
```

将 `render_clangd_config` 改成：

```rust
pub fn render_clangd_config(input: &ClangdConfigInput) -> String {
    let mut output = String::new();
    output.push_str("# Auto-generated by Zed C++ Toolkit.\n");
    output.push_str("# Edit this file to customize clangd behavior.\n");
    output.push_str("CompileFlags:\n");
    output.push_str(&format!(
        "  CompilationDatabase: {}\n",
        format_yaml_value(&input.compile_commands_dir)
    ));
    output.push_str(&format!("  Compiler: {}\n", format_yaml_value(&input.compiler)));
    output.push_str("  Add:\n");

    for flag in &input.extra_flags {
        output.push_str(&format!("    - {}\n", format_yaml_value(flag)));
    }

    if let Some(msvc_include) = &input.msvc_include {
        output.push_str(&format!("    - {}\n", clangd_include_arg(msvc_include)));
        if input.sdk_includes.is_empty() {
            output.push_str("    # Windows SDK include not auto-detected; manually add /I... if needed\n");
            output.push_str("    # - /IC:/Program Files (x86)/Windows Kits/10/Include/<version>/ucrt\n");
            output.push_str("    # - /IC:/Program Files (x86)/Windows Kits/10/Include/<version>/um\n");
            output.push_str("    # - /IC:/Program Files (x86)/Windows Kits/10/Include/<version>/shared\n");
        } else {
            for include in &input.sdk_includes {
                output.push_str(&format!("    - {}\n", clangd_include_arg(include)));
            }
        }
    }

    output.push_str("Diagnostics:\n");
    output.push_str("  Suppress: ['pp_file_not_found']\n");
    output
}
```

- [ ] **Step 4: 更新旧测试输入**

把现有测试里的输入从：

```rust
ClangdConfigInput {
    msvc_include: r"C:\VS\VC\Tools\MSVC\14.40.33807\include".to_string(),
    sdk_includes: Vec::new(),
    compile_database_path: None,
}
```

改为：

```rust
ClangdConfigInput {
    compiler: "clang-cl".to_string(),
    compile_commands_dir: "build".to_string(),
    extra_flags: Vec::new(),
    msvc_include: Some(r"C:\VS\VC\Tools\MSVC\14.40.33807\include".to_string()),
    sdk_includes: Vec::new(),
}
```

旧测试中关于 `compile_database_path: Some(...)` 的断言改成检查 `compile_commands_dir`：

```rust
assert!(rendered.contains("CompilationDatabase: C:/project/build"));
assert!(rendered.contains("Compiler: clang-cl"));
```

删除或改写 `does_not_render_compilation_database_when_missing`，因为新设计始终生成 `CompilationDatabase`。

- [ ] **Step 5: 更新 `src/lsp/workspace_config.rs` 测试输入**

把 helper `input()` 改成：

```rust
fn input() -> ClangdConfigInput {
    ClangdConfigInput {
        compiler: "clang-cl".to_string(),
        compile_commands_dir: "build".to_string(),
        extra_flags: Vec::new(),
        msvc_include: Some(r"C:\VS\VC\Tools\MSVC\14.40.33807\include".to_string()),
        sdk_includes: Vec::new(),
    }
}
```

把 `creates_config_when_file_is_missing` 中关于 `Compiler: clang-cl` 的断言保留。

- [ ] **Step 6: 运行 clangd 配置测试**

运行：

```bash
cargo test lsp::clangd_config::tests lsp::workspace_config::tests -- --nocapture
```

预期： PASS。

- [ ] **Step 7: 提交**

```bash
git add src/lsp/clangd_config.rs src/lsp/workspace_config.rs
git commit -m "Generalize clangd config rendering"
```

---

## Task 4: 添加通用 Zed task 生成器

**文件：**
- 修改：`src/lib.rs`
- 新建：`src/build/mod.rs`
- 新建：`src/build/shell.rs`
- 新建：`src/build/tasks.rs`

- [ ] **Step 1: 注册 build 模块**

在 `src/lib.rs` 模块列表中加入：

```rust
mod build;
```

- [ ] **Step 2: 创建 `src/build/mod.rs`**

```rust
pub mod shell;
pub mod tasks;
```

- [ ] **Step 3: 创建 shell 包装测试和实现**

创建 `src/build/shell.rs`：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Powershell,
    Sh,
}

pub fn default_shell_for_current_platform() -> ShellKind {
    if cfg!(target_os = "windows") {
        ShellKind::Powershell
    } else {
        ShellKind::Sh
    }
}

pub fn wrap_command(shell: ShellKind, command: &str) -> (String, Vec<String>) {
    match shell {
        ShellKind::Powershell => (
            "powershell".to_string(),
            vec!["-NoProfile".to_string(), "-Command".to_string(), command.to_string()],
        ),
        ShellKind::Sh => ("sh".to_string(), vec!["-lc".to_string(), command.to_string()]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_powershell_command() {
        let (command, args) = wrap_command(ShellKind::Powershell, "cmake --build build");
        assert_eq!(command, "powershell");
        assert_eq!(args, vec!["-NoProfile", "-Command", "cmake --build build"]);
    }

    #[test]
    fn wraps_sh_command() {
        let (command, args) = wrap_command(ShellKind::Sh, "cmake --build build");
        assert_eq!(command, "sh");
        assert_eq!(args, vec!["-lc", "cmake --build build"]);
    }
}
```

- [ ] **Step 4: 创建通用 task 生成测试和实现**

创建 `src/build/tasks.rs`：

```rust
use crate::build::shell::{ShellKind, wrap_command};
use crate::config::schema::EffectiveConfig;
use crate::error::{ToolkitError, ToolkitResult};
use serde_json::json;

pub fn generate_cpp_tasks_json(config: &EffectiveConfig, shell: ShellKind) -> ToolkitResult<String> {
    let mut tasks = Vec::new();

    if let Some(command) = &config.build.configure {
        tasks.push(task("C++: Configure", command, "$ZED_WORKTREE_ROOT", shell));
    }
    if let Some(command) = &config.build.build {
        tasks.push(task("C++: Build", command, "$ZED_WORKTREE_ROOT", shell));
    }
    if let Some(command) = &config.build.clean {
        tasks.push(task("C++: Clean", command, "$ZED_WORKTREE_ROOT", shell));
    }
    if let Some(command) = &config.run.command {
        tasks.push(task("C++: Run", command, &config.run.cwd, shell));
    }

    serde_json::to_string_pretty(&tasks).map_err(|error| ToolkitError::IoMessage(error.to_string()))
}

fn task(label: &str, command_string: &str, cwd: &str, shell: ShellKind) -> serde_json::Value {
    let (command, args) = wrap_command(shell, command_string);
    json!({
        "label": label,
        "command": command,
        "args": args,
        "env": {},
        "cwd": cwd
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::shell::ShellKind;
    use crate::config::merge::resolve_config;
    use crate::config::schema::UserConfig;

    #[test]
    fn generates_configure_build_and_clean_tasks() {
        let config = resolve_config(Some(UserConfig {
            preset: Some("gcc-cmake-ninja".to_string()),
            ..UserConfig::default()
        }))
        .unwrap();
        let json = generate_cpp_tasks_json(&config, ShellKind::Sh).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.as_array().unwrap().len(), 3);
        assert_eq!(parsed[0]["label"], "C++: Configure");
        assert_eq!(parsed[1]["label"], "C++: Build");
        assert_eq!(parsed[2]["label"], "C++: Clean");
        assert_eq!(parsed[1]["command"], "sh");
        assert_eq!(parsed[1]["args"][0], "-lc");
        assert_eq!(parsed[1]["args"][1], "cmake --build build");
    }
}
```

- [ ] **Step 5: 运行 build 模块测试**

运行：

```bash
cargo test build:: -- --nocapture
```

预期： PASS。

- [ ] **Step 6: 提交**

```bash
git add src/lib.rs src/build
git commit -m "Add generic cpp task generation"
```

---

## Task 5: 让 clangd 命令使用配置中的 command 和 query_driver

**文件：**
- 修改：`src/lsp/server.rs`
- 修改：`src/lib.rs`
- 修改：`extension.toml`

- [ ] **Step 1: 更新 language server ID 测试期望**

在 `src/lsp/server.rs` 中把：

```rust
pub const LANGUAGE_SERVER_ID: &str = "msvc-cpp-clangd";
```

改成：

```rust
pub const LANGUAGE_SERVER_ID: &str = "cpp-toolkit-clangd";
```

将测试 `accepts_expected_language_server_id` 改成：

```rust
#[test]
fn accepts_expected_language_server_id() {
    assert_eq!(validate_language_server_id("cpp-toolkit-clangd"), Ok(()));
}
```

- [ ] **Step 2: 添加 query_driver 参数测试**

在 `src/lsp/server.rs` 测试模块中添加：

```rust
#[test]
fn build_clangd_command_includes_query_driver() {
    let command = build_clangd_command(
        "clangd".to_string(),
        Vec::new(),
        vec!["gcc".to_string(), "g++".to_string()],
    );

    assert!(command.args.contains(&"--query-driver=gcc,g++".to_string()));
}
```

- [ ] **Step 3: 修改 `build_clangd_command` 签名和实现**

把函数改成：

```rust
pub fn build_clangd_command(
    command: String,
    env: Vec<(String, String)>,
    query_driver: Vec<String>,
) -> zed::Command {
    let mut args = clangd_args();
    if !query_driver.is_empty() {
        args.push(format!("--query-driver={}", query_driver.join(",")));
    }
    zed::Command { command, args, env }
}
```

把调用处从：

```rust
Ok(build_clangd_command(clangd, worktree.shell_env()))
```

改成：

```rust
Ok(build_clangd_command(clangd, worktree.shell_env(), Vec::new()))
```

后续 Task 6 再把真实配置接入。

- [ ] **Step 4: 更新 `src/lib.rs` 中匹配的 language server ID**

把：

```rust
"msvc-cpp-clangd" => {
```

改成：

```rust
"cpp-toolkit-clangd" => {
```

把 `validate_and_prepare_clangd` 中的：

```rust
lsp::server::validate_language_server_id("msvc-cpp-clangd")
```

改成：

```rust
lsp::server::validate_language_server_id("cpp-toolkit-clangd")
```

- [ ] **Step 5: 更新 `extension.toml` 元数据**

把文件开头改成：

```toml
id = "cpp-toolkit"
name = "Zed C++ Toolkit"
description = "Cross-platform C/C++ toolkit for Zed with configurable toolchains, build commands, tasks, and clangd support."
version = "0.6.0"
schema_version = 1
authors = ["Dr.Guo"]
repository = "https://github.com/spartajet/zed-msvc-toolkit.git"
```

把 language server 段改成：

```toml
[language_servers.cpp-toolkit-clangd]
name = "clangd"
languages = ["C", "C++"]
```

保留 CMake LSP 段，后续可以再改名。

- [ ] **Step 6: 运行测试**

运行：

```bash
cargo test lsp::server::tests::accepts_expected_language_server_id lsp::server::tests::build_clangd_command_includes_query_driver -- --nocapture
```

预期： PASS。

- [ ] **Step 7: 提交**

```bash
git add src/lsp/server.rs src/lib.rs extension.toml
git commit -m "Rename clangd server for cpp toolkit"
```

---

## Task 6: 接入配置驱动的 `.clangd` 和 task 生成

**文件：**
- 修改：`src/lsp/server.rs`
- 修改：`src/lsp/workspace_config.rs`

- [ ] **Step 1: 添加配置驱动 workspace 准备函数测试**

在 `src/lsp/server.rs` 测试模块中添加一个不依赖 MSVC 检测的测试：

```rust
#[test]
fn writes_generic_clangd_and_tasks_from_effective_config() {
    use crate::config::merge::resolve_config;
    use crate::config::schema::UserConfig;

    let config = resolve_config(Some(UserConfig {
        preset: Some("gcc-cmake-ninja".to_string()),
        ..UserConfig::default()
    }))
    .unwrap();
    let runner = QueueRunner::new([
        CommandOutput {
            status: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        },
        CommandOutput {
            status: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        },
    ]);

    let result = write_workspace_files_from_config("C:/repo", None, &config, &runner);

    assert_eq!(result, Ok(()));
    assert!(runner.calls.borrow().iter().any(|(command, args)| {
        command == "powershell"
            && args.iter().any(|arg| {
                arg.contains("C:/repo\\.clangd")
                    && arg.contains("CompilationDatabase: build")
                    && arg.contains("Compiler: g++")
            })
    }));
    assert!(runner.calls.borrow().iter().any(|(command, args)| {
        command == "powershell"
            && args.iter().any(|arg| {
                arg.contains("C:/repo\\.zed\\tasks.json")
                    && arg.contains("C++: Build")
                    && arg.contains("cmake --build build")
            })
    }));
}
```

- [ ] **Step 2: 运行测试确认失败**

运行：

```bash
cargo test lsp::server::tests::writes_generic_clangd_and_tasks_from_effective_config -- --nocapture
```

预期： FAIL，因为 `write_workspace_files_from_config` 尚未实现。

- [ ] **Step 3: 实现配置驱动写入函数**

在 `src/lsp/server.rs` 中加入 imports：

```rust
use crate::build::shell::default_shell_for_current_platform;
use crate::build::tasks::generate_cpp_tasks_json;
use crate::config::schema::EffectiveConfig;
```

添加函数：

```rust
fn write_workspace_files_from_config(
    root_path: &str,
    existing_clangd: Option<String>,
    config: &EffectiveConfig,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<()> {
    if let Some(contents) = &existing_clangd {
        if !contents.contains("# Auto-generated by Zed C++ Toolkit.")
            && !contents.contains("# Auto-generated by Zed MSVC C++ Assistant.")
        {
            log_message("skipping generated .clangd because workspace already provides one");
            return Ok(());
        }
    }

    let input = ClangdConfigInput {
        compiler: config.clangd.compiler.clone(),
        compile_commands_dir: config.clangd.compile_commands_dir.clone(),
        extra_flags: config.clangd.extra_flags.clone(),
        msvc_include: None,
        sdk_includes: Vec::new(),
    };
    let clangd_contents = crate::lsp::clangd_config::render_clangd_config(&input);
    write_clangd_file(root_path, &clangd_contents, runner)?;

    let tasks = generate_cpp_tasks_json(config, default_shell_for_current_platform())?;
    write_tasks_file(root_path, &tasks, runner)?;
    Ok(())
}
```

- [ ] **Step 4: 接入 `prepare_workspace_config_from_worktree`**

把 `prepare_workspace_config_from_worktree` 改成：

```rust
pub fn prepare_workspace_config_from_worktree(worktree: &zed::Worktree) -> ToolkitResult<()> {
    let clangd_contents = worktree.read_text_file(".clangd").ok();
    log_message(&format!(
        "worktree .clangd read result: exists={}",
        clangd_contents.is_some()
    ));
    let config = crate::config::loader::load_effective_config(worktree)?;
    prepare_workspace_config_with_effective_config(&worktree.root_path(), clangd_contents, &config, &ZedCommandRunner)
}

pub fn prepare_workspace_config_with_effective_config(
    root_path: &str,
    existing_clangd: Option<String>,
    config: &EffectiveConfig,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<()> {
    if config.toolchain.name == "msvc" {
        prepare_msvc_workspace_config(root_path, existing_clangd, config, runner)
    } else {
        write_workspace_files_from_config(root_path, existing_clangd, config, runner)
    }
}
```

把原来的 `prepare_workspace_config` 内容保留但重命名为：

```rust
fn prepare_msvc_workspace_config(
    root_path: &str,
    existing_clangd: Option<String>,
    config: &EffectiveConfig,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<()> {
    // 先复用原 prepare_workspace_config 的逻辑。
    // 后续把 build_dir/build_type 从 config 传入 ensure_compile_database 和 write_generated_tasks。
    prepare_workspace_config(root_path, existing_clangd, runner)
}
```

保留原 `prepare_workspace_config` 作为测试入口，避免一次修改太多现有 MSVC 测试。

- [ ] **Step 5: 让 clangd 启动命令读取配置**

把 `command_from_worktree` 改成：

```rust
pub fn command_from_worktree(worktree: &zed::Worktree) -> ToolkitResult<zed::Command> {
    let config = crate::config::loader::load_effective_config(worktree)?;
    log_message(&format!("looking up clangd via worktree.which({:?})", config.clangd.command));
    let clangd = require_clangd(worktree.which(&config.clangd.command))?;
    log_message(&format!("clangd found: {clangd}"));
    Ok(build_clangd_command(
        clangd,
        worktree.shell_env(),
        config.clangd.query_driver,
    ))
}
```

- [ ] **Step 6: 运行相关测试**

运行：

```bash
cargo test config:: build:: lsp::clangd_config::tests lsp::workspace_config::tests lsp::server::tests -- --nocapture
```

预期： PASS。

- [ ] **Step 7: 提交**

```bash
git add src/lsp/server.rs src/lsp/workspace_config.rs
git commit -m "Use cpp toolkit config for workspace files"
```

---

## Task 7: 更新 README 和安装说明

**文件：**
- 修改：`README.md`
- 修改：`INSTALL.md`

- [ ] **Step 1: 用中文 README 替换旧 MSVC 专用说明**

将 `README.md` 改成以下结构：

```markdown
# Zed C++ Toolkit

跨平台 C/C++ Toolkit for Zed。它通过 `.zed/cpp-toolkit.toml` 选择 preset、编译器、构建命令、运行命令和 clangd 配置，并自动生成：

- `.clangd`
- `.zed/tasks.json`

## 快速开始

最简单的 CMake 项目不需要配置文件。扩展会根据平台选择默认 preset：

| 平台 | 默认 preset |
| --- | --- |
| Windows | `msvc-cmake-ninja` |
| Linux | `gcc-cmake-ninja` |
| macOS | `clang-cmake-ninja` |

默认 CMake 构建目录是 `build`。

## 配置文件

项目配置文件：

```text
.zed/cpp-toolkit.toml
```

示例：

```toml
preset = "gcc-cmake-ninja"

[build]
build_dir_style = "build"
build_type = "Debug"

[clangd]
extra_flags = ["-std=c++20"]
```

## CLion 构建目录风格

只有显式设置时才启用：

```toml
[build]
build_dir_style = "clion"
build_type = "Debug"
```

这会生成：

```text
cmake-build-debug
```

## 自定义命令

```toml
preset = "custom"

[toolchain]
cc = "gcc"
cxx = "g++"

[build]
build = "make -j16"
clean = "make clean"

[run]
command = "./app"

[clangd]
compiler = "g++"
compile_commands_dir = "."
extra_flags = ["-Iinclude", "-std=c++23"]
query_driver = ["gcc", "g++"]
```
```

- [ ] **Step 2: 更新 `INSTALL.md` 中扩展目录名**

把安装路径中的旧目录名替换为：

```text
cpp-toolkit
```

并说明构建产物名称仍由 Cargo package 决定，直到后续重命名 crate。

- [ ] **Step 3: 运行文档相关检查**

运行：

```bash
git --no-pager diff -- README.md INSTALL.md
```

预期： 文档为中文，旧的“Windows-specific MSVC toolkit”定位已删除。

- [ ] **Step 4: 提交**

```bash
git add README.md INSTALL.md
git commit -m "Update docs for cpp toolkit"
```

---

## Task 8: 全量验证和修复

**文件：**
- 可能修改前面任务涉及的文件，但仅限测试暴露出问题时。

- [ ] **Step 1: 格式化**

运行：

```bash
cargo fmt
```

预期： 无错误。

- [ ] **Step 2: 运行测试**

运行：

```bash
cargo test -- --nocapture
```

预期： PASS。

如果失败，优先修复当前任务引入的问题，不要重构无关代码。

- [ ] **Step 3: 构建 WASM 扩展**

运行：

```bash
cargo build --target wasm32-unknown-unknown --release
```

预期： PASS，生成 release WASM。

如果本机未安装 target，先运行：

```bash
rustup target add wasm32-unknown-unknown
```

再重试 build。

- [ ] **Step 4: 查看最终 diff**

运行：

```bash
git --no-pager diff --stat
```

预期： diff 只包含本计划相关文件。

- [ ] **Step 5: 最终提交**

如果 Step 1-4 中产生了修复改动：

```bash
git add .
git commit -m "Stabilize cpp toolkit migration"
```

如果没有新改动，不创建空提交。

---

## 自检结果

- 规格覆盖：计划覆盖配置文件、preset、默认 `build` 目录、显式 `clion` 风格、模板变量、强制 `.clangd`、task 生成、clangd `query_driver`、MSVC 保留策略、文档更新。
- 占位符扫描：没有使用 TBD/TODO/待定式占位步骤；MSVC provider 目录搬迁明确列为暂不移动。
- 类型一致性：`UserConfig`、`EffectiveConfig`、`BuildDirStyle`、`ClangdConfigInput`、`generate_cpp_tasks_json` 在后续任务中使用的字段名与前置任务定义一致。
