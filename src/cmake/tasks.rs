//! Zed tasks file generation.
//!
//! V0.4 implements .zed/tasks.json generation, bypassing API limitations to support CMake commands.

use crate::error::{ToolkitError, ToolkitResult};
use serde_json::json;

/// CMake build target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmakeTarget {
    /// CMake/Ninja target name.
    pub name: String,
    /// Target output file, relative to build directory.
    pub output: Option<String>,
    /// Whether this is an executable target.
    pub executable: bool,
}

/// Task configuration options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskOptions {
    /// Build directory (relative to workspace root)
    pub build_dir: String,
    /// Build type
    pub build_type: String,
    /// Visual Studio Developer Command Prompt script path.
    pub vs_dev_cmd: Option<String>,
    /// List of targets after CMake generation.
    pub targets: Vec<CmakeTarget>,
}

/// Generates Zed tasks file content.
///
/// The returned JSON contains CMake configure, build, and run tasks,
/// using Zed's ZED_WORKTREE_ROOT environment variable to reference the workspace root.
pub fn generate_tasks_json(options: &TaskOptions) -> ToolkitResult<String> {
    let mut tasks = vec![
        powershell_task(
            format!("CMake: Configure ({})", options.build_type),
            configure_script(options),
        ),
        powershell_task(
            format!("CMake: Build ({})", options.build_type),
            build_script(options, None),
        ),
    ];

    for target in &options.targets {
        tasks.push(powershell_task(
            format!("CMake: Build Target: {}", target.name),
            build_script(options, Some(&target.name)),
        ));
    }

    for target in options.targets.iter().filter(|target| target.executable) {
        if let Some(output) = &target.output {
            tasks.push(powershell_task(
                format!("CMake: Run: {}", target.name),
                run_script(options, output),
            ));
        }
    }

    serde_json::to_string_pretty(&tasks).map_err(|error| ToolkitError::IoMessage(error.to_string()))
}

/// Returns build directory name in CLion style.
fn build_dir_for_type(build_type: &str) -> String {
    let suffix = match build_type {
        "Debug" => "debug",
        "Release" => "release",
        "RelWithDebInfo" => "relwithdebinfo",
        "MinSizeRel" => "minsizerel",
        _ => &build_type.to_lowercase(),
    };
    format!("cmake-build-{}", suffix)
}

/// Default task configuration (Debug build).
impl Default for TaskOptions {
    fn default() -> Self {
        let build_type = "Debug";
        Self {
            build_dir: build_dir_for_type(build_type),
            build_type: build_type.to_string(),
            vs_dev_cmd: None,
            targets: Vec::new(),
        }
    }
}

fn powershell_task(label: String, script: String) -> serde_json::Value {
    json!({
        "label": label,
        "command": "powershell",
        "args": [
            "-NoProfile",
            "-Command",
            script
        ],
        "env": {},
        "cwd": "$ZED_WORKTREE_ROOT"
    })
}

fn configure_script(options: &TaskOptions) -> String {
    let cmake_command = format!(
        "cmake -S \"%ZED_WORKTREE_ROOT%\" -B \"%ZED_WORKTREE_ROOT%\\{}\" -G Ninja -DCMAKE_C_COMPILER=cl -DCMAKE_CXX_COMPILER=cl -DCMAKE_BUILD_TYPE={} -DCMAKE_EXPORT_COMPILE_COMMANDS=ON",
        options.build_dir, options.build_type
    );
    developer_environment_script(options, &cmake_command)
}

fn build_script(options: &TaskOptions, target: Option<&str>) -> String {
    let mut cmake_command = format!(
        "cmake --build \"%ZED_WORKTREE_ROOT%\\{}\" --config {}",
        options.build_dir, options.build_type
    );

    if let Some(target) = target {
        cmake_command.push_str(" --target ");
        cmake_command.push_str(&cmd_quote(target));
    }

    developer_environment_script(options, &cmake_command)
}

fn run_script(options: &TaskOptions, output: &str) -> String {
    let output = output.replace('/', "\\");
    format!(
        "& \"$ZED_WORKTREE_ROOT\\{}\\{}\"",
        options.build_dir.replace('\'', "''"),
        output.replace('"', "`\"")
    )
}

fn developer_environment_script(options: &TaskOptions, command: &str) -> String {
    let cmd_line = if let Some(vs_dev_cmd) = &options.vs_dev_cmd {
        format!(
            "call {} -arch=x64 -host_arch=x64 && {command}",
            cmd_quote(vs_dev_cmd)
        )
    } else {
        command.to_string()
    };

    format!("& cmd.exe /S /C {}", powershell_single_quote(&cmd_line))
}

fn powershell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn cmd_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_tasks_json_creates_valid_json() {
        let options = TaskOptions::default();
        let json = generate_tasks_json(&options).unwrap();

        // 验证是有效 JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
    }

    #[test]
    fn tasks_include_configure_and_build() {
        let options = TaskOptions::default();
        let json = generate_tasks_json(&options).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.as_array().unwrap().len(), 2);
        assert_eq!(parsed[0]["label"], "CMake: Configure (Debug)");
        assert_eq!(parsed[1]["label"], "CMake: Build (Debug)");
    }

    #[test]
    fn tasks_keep_documented_field_order() {
        let options = TaskOptions::default();
        let json = generate_tasks_json(&options).unwrap();

        assert!(json.contains(
            "{\n    \"label\": \"CMake: Configure (Debug)\",\n    \"command\": \"powershell\",\n    \"args\":"
        ));
        assert!(json.contains("\n    \"env\": {},\n    \"cwd\": \"$ZED_WORKTREE_ROOT\""));
    }

    #[test]
    fn tasks_use_workspace_root_variable() {
        let options = TaskOptions::default();
        let json = generate_tasks_json(&options).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Configure 任务使用 ZED_WORKTREE_ROOT
        let configure_args = parsed[0]["args"].as_array().unwrap();
        assert!(configure_args.iter().any(|arg| {
            arg.as_str()
                .map(|s| s.contains("ZED_WORKTREE_ROOT"))
                .unwrap_or(false)
        }));

        // Build 任务使用 ZED_WORKTREE_ROOT
        let build_args = parsed[1]["args"].as_array().unwrap();
        assert!(build_args.iter().any(|arg| {
            arg.as_str()
                .map(|s| s.contains("ZED_WORKTREE_ROOT"))
                .unwrap_or(false)
        }));
    }

    #[test]
    fn custom_build_dir_and_type() {
        let options = TaskOptions {
            build_dir: "my-custom-build".to_string(),
            build_type: "Release".to_string(),
            vs_dev_cmd: None,
            targets: Vec::new(),
        };
        let json = generate_tasks_json(&options).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed[0]["label"], "CMake: Configure (Release)");
        assert_eq!(parsed[1]["label"], "CMake: Build (Release)");

        // 验证自定义构建目录
        let build_arg = parsed[0]["args"]
            .as_array()
            .unwrap()
            .iter()
            .find(|arg| {
                arg.as_str()
                    .map(|s| s.contains("my-custom-build"))
                    .unwrap_or(false)
            })
            .unwrap();
        assert!(
            build_arg
                .as_str()
                .unwrap()
                .contains("%ZED_WORKTREE_ROOT%\\my-custom-build")
        );
    }

    #[test]
    fn default_uses_clion_style_build_dir() {
        let options = TaskOptions::default();
        assert_eq!(options.build_dir, "cmake-build-debug");
        assert_eq!(options.build_type, "Debug");
    }

    #[test]
    fn build_dir_for_type_maps_all_cmake_build_types() {
        assert_eq!(build_dir_for_type("Debug"), "cmake-build-debug");
        assert_eq!(build_dir_for_type("Release"), "cmake-build-release");
        assert_eq!(build_dir_for_type("RelWithDebInfo"), "cmake-build-relwithdebinfo");
        assert_eq!(build_dir_for_type("MinSizeRel"), "cmake-build-minsizerel");
    }

    #[test]
    fn tasks_include_target_build_and_run() {
        let options = TaskOptions {
            targets: vec![CmakeTarget {
                name: "app".to_string(),
                output: Some("app.exe".to_string()),
                executable: true,
            }],
            ..TaskOptions::default()
        };
        let json = generate_tasks_json(&options).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.as_array().unwrap().len(), 4);
        assert_eq!(parsed[2]["label"], "CMake: Build Target: app");
        assert_eq!(parsed[3]["label"], "CMake: Run: app");
        assert!(parsed[3]["args"][2].as_str().unwrap().contains("app.exe"));
        assert!(
            !parsed[3]["args"][2]
                .as_str()
                .unwrap()
                .contains("$ErrorActionPreference")
        );
    }

    #[test]
    fn tasks_use_visual_studio_developer_environment_when_available() {
        let options = TaskOptions {
            vs_dev_cmd: Some("C:\\VS\\Common7\\Tools\\VsDevCmd.bat".to_string()),
            ..TaskOptions::default()
        };
        let json = generate_tasks_json(&options).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(
            parsed[0]["args"][2]
                .as_str()
                .unwrap()
                .contains("VsDevCmd.bat")
        );
        assert!(
            !parsed[0]["args"][2]
                .as_str()
                .unwrap()
                .contains("$ErrorActionPreference")
        );
    }
}
