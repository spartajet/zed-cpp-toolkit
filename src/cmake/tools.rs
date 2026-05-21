//! CMake tool discovery and command building.
//!
//! V0.3 implements cmake/ninja detection and command generation.

use crate::environment::tools::CommandRunner;
use crate::error::{ToolkitError, ToolkitResult};

/// CMake executable name.
pub const CMAKE_EXE: &str = "cmake";

/// Ninja executable name.
pub const NINJA_EXE: &str = "ninja";

/// CMake configuration options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmakeConfigureOptions {
    /// Source directory (workspace root)
    pub source_dir: String,
    /// Build directory
    pub build_dir: String,
    /// Generator
    pub generator: CmakeGenerator,
    /// Build type
    pub build_type: CmakeBuildType,
}

/// CMake generator type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmakeGenerator {
    Ninja,
    VisualStudio2022,
}

/// CMake build type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum CmakeBuildType {
    Debug,
    Release,
    RelWithDebInfo,
}

impl CmakeGenerator {
    /// Returns the generator name (without -G prefix).
    #[allow(dead_code)]
    pub fn generator_name(&self) -> &str {
        match self {
            Self::Ninja => "Ninja",
            Self::VisualStudio2022 => "Visual Studio 17 2022",
        }
    }

    /// Returns the full command-line argument list for the generator.
    ///
    /// Returns a Vec where each element is a separate command-line argument.
    /// Example: vec!["-G", "Ninja"]
    pub fn as_args(&self) -> Vec<String> {
        match self {
            Self::Ninja => vec!["-G".to_string(), "Ninja".to_string()],
            Self::VisualStudio2022 => vec!["-G".to_string(), "Visual Studio 17 2022".to_string()],
        }
    }
}

impl CmakeBuildType {
    /// Returns the CMake variable value for the build type.
    pub fn as_cmake_var(&self) -> &str {
        match self {
            Self::Debug => "Debug",
            Self::Release => "Release",
            Self::RelWithDebInfo => "RelWithDebInfo",
        }
    }

    /// Returns the config argument for --build.
    pub fn as_build_arg(&self) -> &str {
        match self {
            Self::Debug => "Debug",
            Self::Release => "Release",
            Self::RelWithDebInfo => "RelWithDebInfo",
        }
    }
}

/// Discovers CMake in the system.
///
/// Verifies CMake availability by executing `cmake --version`.
///
/// # Returns
///
/// Returns `Ok("cmake")` if the tool is available in system PATH.
/// Returns `Err(ToolkitError::MissingCmake)` if the tool doesn't exist or cannot be executed.
///
/// # Note
///
/// This function returns the executable name, not the full path.
/// Callers should ensure this name is available in the PATH environment variable.
pub fn discover_cmake(runner: &impl CommandRunner) -> ToolkitResult<String> {
    runner
        .run_command(CMAKE_EXE, &["--version".to_string()])
        .map(|_| CMAKE_EXE.to_string())
        .map_err(|_| ToolkitError::MissingCmake)
}

/// Discovers Ninja in the system.
///
/// Verifies Ninja availability by executing `ninja --version`.
///
/// # Returns
///
/// Returns `Some("ninja")` if the tool is available in system PATH.
/// Returns `None` if the tool doesn't exist or cannot be executed.
///
/// # Note
///
/// This function returns the executable name, not the full path.
pub fn discover_ninja(runner: &impl CommandRunner) -> Option<String> {
    runner
        .run_command(NINJA_EXE, &["--version".to_string()])
        .ok()
        .map(|_| NINJA_EXE.to_string())
}

/// Selects an appropriate generator based on the environment.
///
/// Prefers Ninja, falls back to Visual Studio 2022.
pub fn select_generator(runner: &impl CommandRunner) -> CmakeGenerator {
    if discover_ninja(runner).is_some() {
        CmakeGenerator::Ninja
    } else {
        CmakeGenerator::VisualStudio2022
    }
}

/// Builds CMake configure command argument list.
///
/// The returned argument list can be passed directly to the CMake executable.
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

/// Builds CMake build command argument list.
///
/// The returned argument list can be passed directly to the CMake executable.
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
        assert_eq!(
            CmakeBuildType::RelWithDebInfo.as_cmake_var(),
            "RelWithDebInfo"
        );
    }

    #[test]
    fn build_type_produces_correct_build_arg() {
        assert_eq!(CmakeBuildType::Debug.as_build_arg(), "Debug");
        assert_eq!(CmakeBuildType::Release.as_build_arg(), "Release");
        assert_eq!(
            CmakeBuildType::RelWithDebInfo.as_build_arg(),
            "RelWithDebInfo"
        );
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
