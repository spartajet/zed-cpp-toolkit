//! Zed 任务文件生成。
//!
//! V0.4 实现 .zed/tasks.json 生成，绕过 API 限制支持 CMake 命令。

use crate::error::ToolkitResult;
use serde_json::json;

/// 任务配置选项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskOptions {
    /// 构建目录（相对于工作区根目录）
    pub build_dir: String,
    /// 构建类型
    pub build_type: String,
}

/// 生成 Zed 任务文件内容。
///
/// 返回的 JSON 包含 CMake configure 和 build 任务，
/// 使用 $ZED_WORKTREE_ROOT 变量引用工作区根目录。
pub fn generate_tasks_json(options: &TaskOptions) -> ToolkitResult<String> {
    let build_dir_with_var = format!("$ZED_WORKTREE_ROOT/{}", options.build_dir);

    let tasks = json!(
        [
            {
                "label": format!("CMake: Configure ({})", options.build_type),
                "command": "cmake",
                "args": [
                    "-S",
                    "$ZED_WORKTREE_ROOT",
                    "-B",
                    &build_dir_with_var,
                    "-DCMAKE_BUILD_TYPE=".to_string() + &options.build_type
                ],
                "env": {}
            },
            {
                "label": format!("CMake: Build ({})", options.build_type),
                "command": "cmake",
                "args": [
                    "--build",
                    &build_dir_with_var,
                    "--config",
                    &options.build_type
                ],
                "env": {}
            }
        ]
    );

    Ok(tasks.to_string())
}

/// 默认任务配置（Debug 构建）。
impl Default for TaskOptions {
    fn default() -> Self {
        Self {
            build_dir: "build".to_string(),
            build_type: "Debug".to_string(),
        }
    }
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
    fn tasks_use_workspace_root_variable() {
        let options = TaskOptions::default();
        let json = generate_tasks_json(&options).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Configure 任务使用 $ZED_WORKTREE_ROOT
        let configure_args = parsed[0]["args"].as_array().unwrap();
        assert!(configure_args.iter().any(|arg| {
            arg.as_str()
                .map(|s| s.contains("$ZED_WORKTREE_ROOT"))
                .unwrap_or(false)
        }));

        // Build 任务使用 $ZED_WORKTREE_ROOT
        let build_args = parsed[1]["args"].as_array().unwrap();
        assert!(build_args.iter().any(|arg| {
            arg.as_str()
                .map(|s| s.contains("$ZED_WORKTREE_ROOT"))
                .unwrap_or(false)
        }));
    }

    #[test]
    fn custom_build_dir_and_type() {
        let options = TaskOptions {
            build_dir: "cmake-build".to_string(),
            build_type: "Release".to_string(),
        };
        let json = generate_tasks_json(&options).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed[0]["label"], "CMake: Configure (Release)");
        assert_eq!(parsed[1]["label"], "CMake: Build (Release)");

        // 验证构建目录
        let build_arg = parsed[0]["args"]
            .as_array()
            .unwrap()
            .iter()
            .find(|arg| {
                arg.as_str()
                    .map(|s| s.contains("cmake-build"))
                    .unwrap_or(false)
            })
            .unwrap();
        assert_eq!(build_arg, "$ZED_WORKTREE_ROOT/cmake-build");
    }
}
