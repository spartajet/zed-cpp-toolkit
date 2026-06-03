use crate::build::shell::{ShellKind, wrap_command};
use crate::config::schema::EffectiveConfig;
use crate::error::{ToolkitError, ToolkitResult};
use serde_json::json;

pub fn generate_cpp_tasks_json(
    config: &EffectiveConfig,
    shell: ShellKind,
) -> ToolkitResult<String> {
    let mut tasks = Vec::new();

    if let Some(command) = &config.build.configure {
        tasks.push(task("C++: Configure", command, "$ZED_WORKTREE_ROOT", shell));
    }
    if let Some(command) = &config.build.build {
        tasks.push(task("C++: Build", command, "$ZED_WORKTREE_ROOT", shell));
    }
    if let Some(command) = &config.build.clean {
        tasks.push(task("C++: Clean", command, "$ZED_WORKTREE_ROOT", shell));
    }
    if let Some(command) = &config.run.command {
        tasks.push(task("C++: Run", command, &config.run.cwd, shell));
    }

    serde_json::to_string_pretty(&tasks).map_err(|error| ToolkitError::IoMessage(error.to_string()))
}

fn task(label: &str, command_string: &str, cwd: &str, shell: ShellKind) -> serde_json::Value {
    let (command, args) = wrap_command(shell, command_string);
    json!({
        "label": label,
        "command": command,
        "args": args,
        "env": {},
        "cwd": cwd
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::shell::ShellKind;
    use crate::config::merge::resolve_config;
    use crate::config::schema::UserConfig;

    #[test]
    fn generates_configure_build_and_clean_tasks() {
        let config = resolve_config(Some(UserConfig {
            preset: Some("gcc-cmake-ninja".to_string()),
            ..UserConfig::default()
        }))
        .unwrap();
        let json = generate_cpp_tasks_json(&config, ShellKind::Sh).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.as_array().unwrap().len(), 3);
        assert_eq!(parsed[0]["label"], "C++: Configure");
        assert_eq!(parsed[1]["label"], "C++: Build");
        assert_eq!(parsed[2]["label"], "C++: Clean");
        assert_eq!(parsed[1]["command"], "sh");
        assert_eq!(parsed[1]["args"][0], "-lc");
        assert_eq!(parsed[1]["args"][1], "cmake --build build");
    }
}
