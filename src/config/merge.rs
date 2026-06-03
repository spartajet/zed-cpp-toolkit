use crate::config::schema::{
    BuildDirStyle, EffectiveBuild, EffectiveClangd, EffectiveConfig, EffectiveRun,
    EffectiveToolchain, UserConfig,
};
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
    toml::from_str(contents).map_err(|error| {
        ToolkitError::IoMessage(format!("解析 .zed/cpp-toolkit.toml 失败：{error}"))
    })
}
