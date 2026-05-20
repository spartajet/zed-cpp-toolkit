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
fn parse_neocmake_toml(content: &str) -> Value {
    let mut result = serde_json::map::Map::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            let value = match value.trim() {
                "true" => Value::Bool(true),
                "false" => Value::Bool(false),
                other => Value::String(other.to_string()),
            };

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

/// 读取并合并配置。
pub fn load_config(worktree: &zed::Worktree) -> NeocmakeConfig {
    let mut config = NeocmakeConfig::default();

    // 读取 .neocmake.toml
    if let Ok(content) = worktree.read_text_file(".neocmake.toml") {
        log_message("读取 .neocmake.toml");
        let parsed = parse_neocmake_toml(&content);

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

    // 读取 settings.json 覆盖
    if let Ok(settings) = worktree.read_text_file(".zed/settings.json") {
        log_message("读取 .zed/settings.json 以获取 LSP 配置覆盖");

        if let Ok(value) = serde_json::from_str::<Value>(&settings) {
            if let Some(Value::Object(lsp_obj)) = value.get("lsp") {
                if let Some(Value::Object(neocmake_obj)) = lsp_obj.get("msvc-cmake-neocmake") {
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
