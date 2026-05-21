//! neocmakelsp LSP initialization options.

// FeatureConfig is used in tests
#[allow(unused_imports)]
use crate::lsp::neocmake::config::{FeatureConfig, NeocmakeConfig};
use serde_json::Value;

/// Builds LSP initialization options from configuration.
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
