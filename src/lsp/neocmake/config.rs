//! 从 Zed settings.json 读取 neocmakelsp 初始化配置。

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

/// 读取 Zed settings.json 覆盖配置。
pub fn load_config(worktree: &zed::Worktree) -> NeocmakeConfig {
    let settings = match worktree.read_text_file(".zed/settings.json") {
        Ok(settings) => {
            log_message("读取 .zed/settings.json 以获取 LSP 配置覆盖");
            Some(settings)
        }
        Err(_) => None,
    };

    config_from_settings_json(settings.as_deref())
}

fn config_from_settings_json(settings: Option<&str>) -> NeocmakeConfig {
    let mut config = NeocmakeConfig::default();

    let Some(settings) = settings else {
        log_final_config(&config);
        return config;
    };

    let Ok(value) = serde_json::from_str::<Value>(settings) else {
        log_message("解析 .zed/settings.json 失败，使用 neocmakelsp 默认初始化配置");
        log_final_config(&config);
        return config;
    };

    let Some(Value::Object(lsp_obj)) = value.get("lsp") else {
        log_final_config(&config);
        return config;
    };

    let Some(Value::Object(neocmake_obj)) = lsp_obj.get("msvc-cmake-neocmake") else {
        log_final_config(&config);
        return config;
    };

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
        log_message(&format!(
            "settings.json 覆盖: scan_cmake_in_package = {scan}"
        ));
    }
    if let Some(Value::Bool(token)) = neocmake_obj.get("semantic_token") {
        config.semantic_token = *token;
        log_message(&format!("settings.json 覆盖: semantic_token = {token}"));
    }

    log_final_config(&config);
    config
}

fn log_final_config(config: &NeocmakeConfig) {
    log_message(&format!(
        "最终 neocmake 配置: format.enable={}, lint.enable={}, scan_cmake_in_package={}, semantic_token={}",
        config.format.enable,
        config.lint.enable,
        config.scan_cmake_in_package,
        config.semantic_token
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_json_overrides_default_init_options() {
        let settings = r#"{
            "lsp": {
                "msvc-cmake-neocmake": {
                    "format": { "enable": false },
                    "lint": { "enable": true },
                    "scan_cmake_in_package": false,
                    "semantic_token": true
                }
            }
        }"#;

        let config = config_from_settings_json(Some(settings));

        assert!(!config.format.enable);
        assert!(config.lint.enable);
        assert!(!config.scan_cmake_in_package);
        assert!(config.semantic_token);
    }

    #[test]
    fn invalid_settings_json_keeps_defaults() {
        let config = config_from_settings_json(Some("{ invalid json"));

        assert!(config.format.enable);
        assert!(config.lint.enable);
        assert!(config.scan_cmake_in_package);
        assert!(!config.semantic_token);
    }
}
