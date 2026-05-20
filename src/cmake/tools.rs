//! CMake 工具探测与命令构建。
//!
//! V0.3 实现 cmake/ninja 探测和命令生成。

use crate::environment::tools::CommandRunner;
use crate::error::{ToolkitError, ToolkitResult};

/// CMake 可执行文件名。
pub const CMAKE_EXE: &str = "cmake";

/// Ninja 可执行文件名。
pub const NINJA_EXE: &str = "ninja";

/// CMake 配置选项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmakeConfigureOptions {
    /// 源目录（工作区根目录）
    pub source_dir: String,
    /// 构建目录
    pub build_dir: String,
    /// 生成器
    pub generator: CmakeGenerator,
    /// 构建类型
    pub build_type: CmakeBuildType,
}

/// CMake 生成器类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmakeGenerator {
    Ninja,
    VisualStudio2022,
}

/// CMake 构建类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmakeBuildType {
    Debug,
    Release,
    RelWithDebInfo,
}

impl CmakeGenerator {
    /// 返回生成器的命令行参数。
    pub fn as_arg(&self) -> &str {
        match self {
            Self::Ninja => "-G Ninja",
            Self::VisualStudio2022 => "-G \"Visual Studio 17 2022\"",
        }
    }
}

impl CmakeBuildType {
    /// 返回构建类型的 CMake 变量。
    pub fn as_cmake_var(&self) -> &str {
        match self {
            Self::Debug => "Debug",
            Self::Release => "Release",
            Self::RelWithDebInfo => "RelWithDebInfo",
        }
    }

    /// 返回 --build 使用的配置参数。
    pub fn as_build_arg(&self) -> &str {
        match self {
            Self::Debug => "Debug",
            Self::Release => "Release",
            Self::RelWithDebInfo => "RelWithDebInfo",
        }
    }
}

/// 探测系统中的 CMake。
pub fn discover_cmake(runner: &impl CommandRunner) -> ToolkitResult<String> {
    runner
        .run_command(CMAKE_EXE, &["--version".to_string()])
        .map(|_| CMAKE_EXE.to_string())
        .map_err(|_| ToolkitError::MissingCmake)
}

/// 探测系统中的 Ninja。
pub fn discover_ninja(runner: &impl CommandRunner) -> Option<String> {
    runner
        .run_command(NINJA_EXE, &["--version".to_string()])
        .ok()
        .map(|_| NINJA_EXE.to_string())
}

/// 根据环境选择合适的生成器。
///
/// 优先使用 Ninja，回退到 Visual Studio 2022。
pub fn select_generator(runner: &impl CommandRunner) -> CmakeGenerator {
    if discover_ninja(runner).is_some() {
        CmakeGenerator::Ninja
    } else {
        CmakeGenerator::VisualStudio2022
    }
}

/// 构建 CMake configure 命令。
pub fn build_configure_command(
    _cmake_path: &str,
    options: &CmakeConfigureOptions,
) -> Vec<String> {
    let args = vec![
        "-B".to_string(),
        options.build_dir.clone(),
        format!("-G {}", options.generator.as_arg().trim()),
        format!(
            "-DCMAKE_BUILD_TYPE={}",
            options.build_type.as_cmake_var()
        ),
    ];

    args
}

/// 构建 CMake build 命令。
pub fn build_build_command(
    _cmake_path: &str,
    build_dir: &str,
    build_type: CmakeBuildType,
) -> Vec<String> {
    vec![
        "--build".to_string(),
        build_dir.to_string(),
        "--config".to_string(),
        build_type.as_build_arg().to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ninja_generator_produces_correct_arg() {
        assert_eq!(CmakeGenerator::Ninja.as_arg(), "-G Ninja");
    }

    #[test]
    fn visual_studio_generator_produces_correct_arg() {
        assert_eq!(
            CmakeGenerator::VisualStudio2022.as_arg(),
            "-G \"Visual Studio 17 2022\""
        );
    }

    #[test]
    fn build_type_produces_correct_cmake_var() {
        assert_eq!(CmakeBuildType::Debug.as_cmake_var(), "Debug");
        assert_eq!(CmakeBuildType::Release.as_cmake_var(), "Release");
        assert_eq!(CmakeBuildType::RelWithDebInfo.as_cmake_var(), "RelWithDebInfo");
    }

    #[test]
    fn build_type_produces_correct_build_arg() {
        assert_eq!(CmakeBuildType::Debug.as_build_arg(), "Debug");
        assert_eq!(CmakeBuildType::Release.as_build_arg(), "Release");
    }

    #[test]
    fn configure_command_for_ninja() {
        let options = CmakeConfigureOptions {
            source_dir: r"C:\project".to_string(),
            build_dir: "build".to_string(),
            generator: CmakeGenerator::Ninja,
            build_type: CmakeBuildType::Debug,
        };

        let args = build_configure_command("cmake", &options);

        assert!(args.contains(&"-B".to_string()));
        assert!(args.contains(&"build".to_string()));
        assert!(args.iter().any(|a| a.contains("Ninja")));
        assert!(args.contains(&"-DCMAKE_BUILD_TYPE=Debug".to_string()));
    }

    #[test]
    fn configure_command_for_visual_studio() {
        let options = CmakeConfigureOptions {
            source_dir: r"C:\project".to_string(),
            build_dir: "build".to_string(),
            generator: CmakeGenerator::VisualStudio2022,
            build_type: CmakeBuildType::Debug,
        };

        let args = build_configure_command("cmake", &options);

        assert!(args.iter().any(|a| a.contains("Visual Studio 17 2022")));
    }

    #[test]
    fn build_command_includes_config() {
        let args = build_build_command("cmake", "build", CmakeBuildType::Debug);

        assert!(args.contains(&"--build".to_string()));
        assert!(args.contains(&"build".to_string()));
        assert!(args.contains(&"--config".to_string()));
        assert!(args.contains(&"Debug".to_string()));
    }
}
