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
    pub extra_flags: Option<Vec<String>>,
    pub query_driver: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum BuildDirStyle {
    #[default]
    Build,
    Clion,
    Custom,
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
