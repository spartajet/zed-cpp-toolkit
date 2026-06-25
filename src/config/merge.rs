use crate::config::presets::preset_config;
use crate::config::schema::{
    BuildConfig, BuildDirStyle, ClangdConfig, EffectiveBuild, EffectiveClangd, EffectiveConfig,
    EffectiveRun, EffectiveToolchain, RunConfig, ToolchainConfig, UserConfig,
};
use crate::error::{ToolkitError, ToolkitResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostPlatform {
    Windows,
    Macos,
    Linux,
}

impl HostPlatform {
    pub fn from_root_path(root_path: &str) -> Self {
        if is_windows_path(root_path) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Linux
        }
    }
}

fn is_windows_path(path: &str) -> bool {
    path.contains('\\') || path.as_bytes().get(1).is_some_and(|byte| *byte == b':')
}

pub fn default_preset_for_host_platform(platform: HostPlatform) -> &'static str {
    match platform {
        HostPlatform::Windows => "msvc-cmake-ninja",
        HostPlatform::Macos => "clang-cmake-ninja",
        HostPlatform::Linux => "gcc-cmake-ninja",
    }
}

#[cfg(test)]
pub fn resolve_config(user: Option<UserConfig>) -> ToolkitResult<EffectiveConfig> {
    resolve_config_for_host(user, HostPlatform::from_root_path(""))
}

pub fn resolve_config_for_root_path(
    user: Option<UserConfig>,
    root_path: &str,
) -> ToolkitResult<EffectiveConfig> {
    resolve_config_for_host(user, HostPlatform::from_root_path(root_path))
}

pub fn resolve_config_for_host(
    user: Option<UserConfig>,
    host_platform: HostPlatform,
) -> ToolkitResult<EffectiveConfig> {
    let preset_name = user
        .as_ref()
        .and_then(|config| config.preset.clone())
        .unwrap_or_else(|| default_preset_for_host_platform(host_platform).to_string());
    let preset = preset_config(&preset_name).ok_or_else(|| {
        ToolkitError::IoMessage(format!("未知 cpp-toolkit preset：{preset_name}"))
    })?;
    let merged = merge_user_config(preset, user.unwrap_or_default());

    let build_type = merged
        .build
        .build_type
        .clone()
        .unwrap_or_else(|| "Debug".to_string());
    let build_dir_style = merged.build.build_dir_style.unwrap_or_default();
    let explicit_build_dir = merged
        .build
        .build_dir
        .clone()
        .filter(|value| !value.trim().is_empty());
    if build_dir_style == BuildDirStyle::Custom && explicit_build_dir.is_none() {
        return Err(ToolkitError::IoMessage(
            "build_dir_style = \"custom\" requires build_dir".to_string(),
        ));
    }
    let build_dir = explicit_build_dir
        .clone()
        .unwrap_or_else(|| infer_build_dir(build_dir_style, &build_type));

    let toolchain_name = merged
        .toolchain
        .name
        .clone()
        .unwrap_or_else(|| "custom".to_string());
    let cc = merged
        .toolchain
        .cc
        .clone()
        .unwrap_or_else(|| "cc".to_string());
    let cxx = merged
        .toolchain
        .cxx
        .clone()
        .unwrap_or_else(|| "c++".to_string());
    let clangd_compiler = merged
        .clangd
        .compiler
        .clone()
        .unwrap_or_else(|| cxx.clone());
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
            build_dir_template: explicit_build_dir,
            build_type: build_type.clone(),
            configure: expand_optional(merged.build.configure.clone(), &build_dir, &build_type),
            configure_template: merged.build.configure,
            build: expand_optional(merged.build.build.clone(), &build_dir, &build_type),
            build_template: merged.build.build,
            clean: expand_optional(merged.build.clean.clone(), &build_dir, &build_type),
            clean_template: merged.build.clean,
        },
        run: EffectiveRun {
            command: expand_optional(merged.run.command.clone(), &build_dir, &build_type),
            command_template: merged.run.command,
            cwd: merged
                .run
                .cwd
                .unwrap_or_else(|| "$ZED_WORKTREE_ROOT".to_string()),
        },
        clangd: EffectiveClangd {
            command: merged
                .clangd
                .command
                .unwrap_or_else(|| "clangd".to_string()),
            compiler: clangd_compiler,
            compile_commands_dir,
            extra_flags: merged.clangd.extra_flags.unwrap_or_default(),
            query_driver: merged.clangd.query_driver.unwrap_or_default(),
        },
    })
}

pub fn parse_user_config(contents: &str) -> ToolkitResult<UserConfig> {
    toml::from_str(contents).map_err(|error| {
        ToolkitError::IoMessage(format!("解析 .zed/cpp-toolkit.toml 失败：{error}"))
    })
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
    base.extra_flags = user.extra_flags.or(base.extra_flags);
    base.query_driver = user.query_driver.or(base.query_driver);
    base
}

fn infer_build_dir(style: BuildDirStyle, build_type: &str) -> String {
    match style {
        BuildDirStyle::Build => "build".to_string(),
        BuildDirStyle::Clion => format!("cmake-build-{}", cmake_build_type_suffix(build_type)),
        BuildDirStyle::Custom => unreachable!("custom build_dir_style is validated earlier"),
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
    fn windows_host_path_uses_msvc_default_preset() {
        let config = resolve_config_for_root_path(None, r"C:\repo").unwrap();

        assert_eq!(config.preset, "msvc-cmake-ninja");
        assert_eq!(config.toolchain.name, "msvc");
        assert_eq!(config.clangd.compiler, "clang-cl");
    }

    #[test]
    fn msvc_cmake_preset_pins_cmake_compilers_to_cl() {
        let config = resolve_config_for_host(None, HostPlatform::Windows).unwrap();
        let configure = config.build.configure.unwrap();

        assert!(configure.contains("-DCMAKE_C_COMPILER=cl"));
        assert!(configure.contains("-DCMAKE_CXX_COMPILER=cl"));
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
        assert!(
            config
                .build
                .configure
                .unwrap()
                .contains("-B cmake-build-release")
        );
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

    #[test]
    fn custom_build_dir_style_requires_explicit_build_dir() {
        let error = resolve_config(Some(UserConfig {
            preset: Some("custom".to_string()),
            build: BuildConfig {
                build_dir_style: Some(BuildDirStyle::Custom),
                ..BuildConfig::default()
            },
            ..UserConfig::default()
        }))
        .unwrap_err();

        assert!(
            error
                .user_message()
                .contains("build_dir_style = \"custom\" requires build_dir")
        );
    }

    #[test]
    fn empty_query_driver_overrides_preset_default() {
        let user = parse_user_config(
            r#"
preset = "gcc-cmake-ninja"

[clangd]
query_driver = []
"#,
        )
        .unwrap();

        let config = resolve_config(Some(user)).unwrap();

        assert!(config.clangd.query_driver.is_empty());
    }

    #[test]
    fn empty_extra_flags_overrides_preset_default() {
        let preset = UserConfig {
            preset: Some("custom".to_string()),
            clangd: ClangdConfig {
                extra_flags: Some(vec!["-std=c++20".to_string()]),
                ..ClangdConfig::default()
            },
            ..UserConfig::default()
        };
        let user = UserConfig {
            clangd: ClangdConfig {
                extra_flags: Some(Vec::new()),
                ..ClangdConfig::default()
            },
            ..UserConfig::default()
        };

        let merged = merge_user_config(preset, user);

        assert_eq!(merged.clangd.extra_flags, Some(Vec::new()));
    }
}
