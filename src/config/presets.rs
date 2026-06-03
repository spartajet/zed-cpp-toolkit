use crate::config::schema::{
    BuildConfig, BuildDirStyle, ClangdConfig, ToolchainConfig, UserConfig,
};

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

fn cmake_preset(
    preset: &str,
    toolchain_name: &str,
    cc: &str,
    cxx: &str,
    clangd_compiler: &str,
) -> UserConfig {
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
            query_driver: Some(query_drivers(toolchain_name)),
            ..ClangdConfig::default()
        },
        ..UserConfig::default()
    }
}

fn make_preset(
    preset: &str,
    toolchain_name: &str,
    cc: &str,
    cxx: &str,
    clangd_compiler: &str,
) -> UserConfig {
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
            query_driver: Some(query_drivers(toolchain_name)),
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
