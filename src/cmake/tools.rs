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
    /// 返回生成器的名称（不包括 -G 前缀）。
    pub fn generator_name(&self) -> &str {
        match self {
            Self::Ninja => "Ninja",
            Self::VisualStudio2022 => "Visual Studio 17 2022",
        }
    }

    /// 返回生成器的完整命令行参数列表。
    ///
    /// 返回一个 Vec，每个元素是一个独立的命令行参数。
    /// 例如：vec!["-G", "Ninja"]
    pub fn as_args(&self) -> Vec<String> {
        match self {
            Self::Ninja => vec!["-G".to_string(), "Ninja".to_string()],
            Self::VisualStudio2022 => vec![
                "-G".to_string(),
                "Visual Studio 17 2022".to_string(),
            ],
        }
    }
}

impl CmakeBuildType {
    /// 返回构建类型的 CMake 变量值。
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
///
/// 通过执行 `cmake --version` 验证 CMake 是否可用。
///
/// # 返回值
///
/// 返回 `Ok("cmake")` 表示工具在系统 PATH 中可用。
/// 返回 `Err(ToolkitError::MissingCmake)` 表示工具不存在或无法执行。
///
/// # 注意
///
/// 此函数返回可执行文件名称而非完整路径。
/// 调用者应确保此名称在 PATH 环境变量中可用。
pub fn discover_cmake(runner: &impl CommandRunner) -> ToolkitResult<String> {
    runner
        .run_command(CMAKE_EXE, &["--version".to_string()])
        .map(|_| CMAKE_EXE.to_string())
        .map_err(|_| ToolkitError::MissingCmake)
}

/// 探测系统中的 Ninja。
///
/// 通过执行 `ninja --version` 验证 Ninja 是否可用。
///
/// # 返回值
///
/// 返回 `Some("ninja")` 表示工具在系统 PATH 中可用。
/// 返回 `None` 表示工具不存在或无法执行。
///
/// # 注意
///
/// 此函数返回可执行文件名称而非完整路径。
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

/// 构建 CMake configure 命令参数列表。
///
/// 返回的参数列表可以直接传递给 CMake 可执行文件。
pub fn build_configure_command(options: &CmakeConfigureOptions) -> Vec<String> {
    let mut args = vec![
        "-S".to_string(),
        options.source_dir.clone(),
        "-B".to_string(),
        options.build_dir.clone(),
    ];
    args.extend(options.generator.as_args());
    args.push(format!(
        "-DCMAKE_BUILD_TYPE={}",
        options.build_type.as_cmake_var()
    ));
    args
}

/// 构建 CMake build 命令参数列表。
///
/// 返回的参数列表可以直接传递给 CMake 可执行文件。
pub fn build_build_command(build_dir: &str, build_type: CmakeBuildType) -> Vec<String> {
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
    fn ninja_generator_produces_correct_args() {
        assert_eq!(CmakeGenerator::Ninja.as_args(), vec!["-G", "Ninja"]);
    }

    #[test]
    fn visual_studio_generator_produces_correct_args() {
        assert_eq!(
            CmakeGenerator::VisualStudio2022.as_args(),
            vec!["-G", "Visual Studio 17 2022"]
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
        assert_eq!(CmakeBuildType::RelWithDebInfo.as_build_arg(), "RelWithDebInfo");
    }

    #[test]
    fn configure_command_for_ninja() {
        let options = CmakeConfigureOptions {
            source_dir: r"C:\project".to_string(),
            build_dir: "build".to_string(),
            generator: CmakeGenerator::Ninja,
            build_type: CmakeBuildType::Debug,
        };

        let args = build_configure_command(&options);

        assert!(args.contains(&"-S".to_string()));
        assert!(args.contains(&"C:\\project".to_string()));
        assert!(args.contains(&"-B".to_string()));
        assert!(args.contains(&"build".to_string()));
        assert!(args.contains(&"-G".to_string()));
        assert!(args.contains(&"Ninja".to_string()));
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

        let args = build_configure_command(&options);

        assert!(args.contains(&"-G".to_string()));
        assert!(args.contains(&"Visual Studio 17 2022".to_string()));
    }

    #[test]
    fn build_command_includes_config() {
        let args = build_build_command("build", CmakeBuildType::Debug);

        assert!(args.contains(&"--build".to_string()));
        assert!(args.contains(&"build".to_string()));
        assert!(args.contains(&"--config".to_string()));
        assert!(args.contains(&"Debug".to_string()));
    }

    #[test]
    fn configure_command_arguments_are_separate() {
        let options = CmakeConfigureOptions {
            source_dir: r"C:\project".to_string(),
            build_dir: "build".to_string(),
            generator: CmakeGenerator::VisualStudio2022,
            build_type: CmakeBuildType::Debug,
        };

        let args = build_configure_command(&options);

        // 验证 -S 和源目录是相邻参数
        let s_index = args.iter().position(|a| a == "-S").unwrap();
        assert_eq!(args[s_index + 1], r"C:\project");

        // 验证 -G 和生成器名称是相邻参数
        let g_index = args.iter().position(|a| a == "-G").unwrap();
        assert_eq!(args[g_index + 1], "Visual Studio 17 2022");
    }

    #[test]
    fn source_dir_with_spaces_is_separate_argument() {
        let options = CmakeConfigureOptions {
            source_dir: r"C:\My Project\src".to_string(),
            build_dir: "build".to_string(),
            generator: CmakeGenerator::Ninja,
            build_type: CmakeBuildType::Debug,
        };

        let args = build_configure_command(&options);

        let s_index = args.iter().position(|a| a == "-S").unwrap();
        assert_eq!(args[s_index + 1], r"C:\My Project\src");
    }
}
