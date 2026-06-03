use crate::build::shell::{ShellKind, wrap_command};
use crate::cmake::tasks::CmakeTarget;
use crate::config::schema::EffectiveConfig;
use crate::error::{ToolkitError, ToolkitResult};
use serde_json::json;

pub fn generate_cpp_tasks_json(
    config: &EffectiveConfig,
    shell: ShellKind,
    cmake_targets: &[CmakeTarget],
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

    // Auto-discover executable targets from cmake when no explicit run command is configured
    if config.run.command.is_none() {
        let build_dir = &config.build.build_dir;
        for target in cmake_targets.iter().filter(|t| t.executable) {
            if let Some(output) = &target.output {
                let run_command = run_command_for_target(build_dir, output, shell);
                tasks.push(task(
                    &format!("C++: Run: {}", target.name),
                    &run_command,
                    "$ZED_WORKTREE_ROOT",
                    shell,
                ));
            }
        }
    }

    serde_json::to_string_pretty(&tasks).map_err(|error| ToolkitError::IoMessage(error.to_string()))
}

fn run_command_for_target(build_dir: &str, output: &str, shell: ShellKind) -> String {
    let output = output.replace('/', "\\");
    match shell {
        ShellKind::Powershell => {
            format!("Start-Process -FilePath \"$ZED_WORKTREE_ROOT\\{}\\{}\"", build_dir, output)
        }
        ShellKind::Sh => {
            format!("\"$ZED_WORKTREE_ROOT/{}/{}\"", build_dir, output.replace('\\', "/"))
        }
    }
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
        let json = generate_cpp_tasks_json(&config, ShellKind::Sh, &[]).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.as_array().unwrap().len(), 3);
        assert_eq!(parsed[0]["label"], "C++: Configure");
        assert_eq!(parsed[1]["label"], "C++: Build");
        assert_eq!(parsed[2]["label"], "C++: Clean");
        assert_eq!(parsed[1]["command"], "sh");
        assert_eq!(parsed[1]["args"][0], "-lc");
        assert_eq!(parsed[1]["args"][1], "cmake --build build");
    }

    #[test]
    fn auto_discovers_run_tasks_from_cmake_targets() {
        let config = resolve_config(Some(UserConfig {
            preset: Some("gcc-cmake-ninja".to_string()),
            ..UserConfig::default()
        }))
        .unwrap();
        let targets = vec![CmakeTarget {
            name: "myapp".to_string(),
            output: Some("myapp.exe".to_string()),
            executable: true,
        }];
        let json = generate_cpp_tasks_json(&config, ShellKind::Sh, &targets).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.as_array().unwrap().len(), 4);
        assert_eq!(parsed[3]["label"], "C++: Run: myapp");
        assert!(parsed[3]["args"][1].as_str().unwrap().contains("build/myapp.exe"));
    }

    #[test]
    fn explicit_run_command_overrides_auto_discovery() {
        let config = resolve_config(Some(UserConfig {
            preset: Some("gcc-cmake-ninja".to_string()),
            run: crate::config::schema::RunConfig {
                command: Some("./build/my-custom-app".to_string()),
                cwd: None,
            },
            ..UserConfig::default()
        }))
        .unwrap();
        let targets = vec![CmakeTarget {
            name: "myapp".to_string(),
            output: Some("myapp.exe".to_string()),
            executable: true,
        }];
        let json = generate_cpp_tasks_json(&config, ShellKind::Sh, &targets).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Only one run task (the explicit one), not auto-discovered
        assert_eq!(parsed.as_array().unwrap().len(), 4);
        assert_eq!(parsed[3]["label"], "C++: Run");
        assert!(parsed[3]["args"][1].as_str().unwrap().contains("my-custom-app"));
    }

    #[test]
    fn skips_non_executable_targets() {
        let config = resolve_config(Some(UserConfig {
            preset: Some("gcc-cmake-ninja".to_string()),
            ..UserConfig::default()
        }))
        .unwrap();
        let targets = vec![
            CmakeTarget {
                name: "mylib".to_string(),
                output: Some("libmylib.a".to_string()),
                executable: false,
            },
            CmakeTarget {
                name: "myapp".to_string(),
                output: Some("myapp".to_string()),
                executable: true,
            },
        ];
        let json = generate_cpp_tasks_json(&config, ShellKind::Sh, &targets).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.as_array().unwrap().len(), 4);
        assert_eq!(parsed[3]["label"], "C++: Run: myapp");
    }
}
