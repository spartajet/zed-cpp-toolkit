use crate::build::shell::{ShellKind, wrap_command};
use crate::cmake::tasks::CmakeTarget;
use crate::config::schema::EffectiveConfig;
use crate::error::{ToolkitError, ToolkitResult};
use serde_json::json;

const TASK_PROFILES: [TaskProfile; 2] = [
    TaskProfile {
        build_type: "Debug",
        suffix: "debug",
    },
    TaskProfile {
        build_type: "Release",
        suffix: "release",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TaskProfile {
    build_type: &'static str,
    suffix: &'static str,
}

struct ProfiledBuild {
    build_dir: String,
    build_type: &'static str,
    configure: Option<String>,
    build: Option<String>,
    clean: Option<String>,
    run: Option<String>,
}

pub fn generate_cpp_tasks_json(
    config: &EffectiveConfig,
    shell: ShellKind,
    cmake_targets: &[CmakeTarget],
) -> ToolkitResult<String> {
    let mut tasks = Vec::new();

    for profile in TASK_PROFILES {
        let profiled = profiled_build(config, profile);

        if let Some(command) = &profiled.configure {
            tasks.push(task(
                &profile_label("C++: Configure", profile.build_type),
                command,
                "$ZED_WORKTREE_ROOT",
                shell,
            ));
        }
        if let Some(command) = &profiled.build {
            tasks.push(task(
                &profile_label("C++: Build", profile.build_type),
                command,
                "$ZED_WORKTREE_ROOT",
                shell,
            ));
        }
        if let Some(command) = &profiled.clean {
            tasks.push(task(
                &profile_label("C++: Clean", profile.build_type),
                command,
                "$ZED_WORKTREE_ROOT",
                shell,
            ));
        }

        if config.build.system == "cmake" {
            for target in cmake_targets {
                let build_command = build_command_for_target(&profiled, &target.name);
                tasks.push(task(
                    &profile_target_label("C++: Build Target", profile.build_type, &target.name),
                    &build_command,
                    "$ZED_WORKTREE_ROOT",
                    shell,
                ));
            }
        }

        if let Some(command) = &profiled.run {
            tasks.push(task(
                &profile_label("C++: Run", profile.build_type),
                command,
                &config.run.cwd,
                shell,
            ));
        }

        // Auto-discover executable targets from cmake when no explicit run command is configured
        if profiled.run.is_none() {
            for target in cmake_targets.iter().filter(|t| t.executable) {
                if let Some(output) = &target.output {
                    let run_command = run_command_for_target(&profiled.build_dir, output, shell);
                    tasks.push(task(
                        &profile_target_label("C++: Run", profile.build_type, &target.name),
                        &run_command,
                        "$ZED_WORKTREE_ROOT",
                        shell,
                    ));
                }
            }
        }
    }

    serde_json::to_string_pretty(&tasks).map_err(|error| ToolkitError::IoMessage(error.to_string()))
}

fn profiled_build(config: &EffectiveConfig, profile: TaskProfile) -> ProfiledBuild {
    let build_dir = profile_build_dir(config, profile);
    let build_type = profile.build_type;

    ProfiledBuild {
        configure: profile_command(
            config
                .build
                .configure_template
                .as_deref()
                .or(config.build.configure.as_deref()),
            &build_dir,
            build_type,
        ),
        build: profile_command(
            config
                .build
                .build_template
                .as_deref()
                .or(config.build.build.as_deref()),
            &build_dir,
            build_type,
        ),
        clean: profile_command(
            config
                .build
                .clean_template
                .as_deref()
                .or(config.build.clean.as_deref()),
            &build_dir,
            build_type,
        ),
        run: profile_command(
            config
                .run
                .command_template
                .as_deref()
                .or(config.run.command.as_deref()),
            &build_dir,
            build_type,
        ),
        build_dir,
        build_type,
    }
}

fn profile_command(command: Option<&str>, build_dir: &str, build_type: &str) -> Option<String> {
    command.map(|command| {
        command
            .replace("{build_dir}", build_dir)
            .replace("{build_type}", build_type)
    })
}

fn profile_build_dir(config: &EffectiveConfig, profile: TaskProfile) -> String {
    let base_dir = config
        .build
        .build_dir_template
        .as_deref()
        .unwrap_or(&config.build.build_dir);
    let trimmed = base_dir.trim_end_matches(['/', '\\']);
    let lower = trimmed.to_lowercase();
    let suffix = profile.suffix;

    if lower.ends_with(&format!("-{suffix}"))
        || lower.ends_with(&format!("/{suffix}"))
        || lower.ends_with(&format!("\\{suffix}"))
    {
        trimmed.to_string()
    } else if suffix == "debug"
        && (lower.ends_with("-release")
            || lower.ends_with("/release")
            || lower.ends_with("\\release"))
    {
        replace_last_path_segment(trimmed, "debug")
    } else if suffix == "release"
        && (lower.ends_with("-debug") || lower.ends_with("/debug") || lower.ends_with("\\debug"))
    {
        replace_last_path_segment(trimmed, "release")
    } else {
        format!("{trimmed}/{suffix}")
    }
}

fn replace_last_path_segment(path: &str, suffix: &str) -> String {
    if let Some(prefix) = path
        .strip_suffix("-debug")
        .or_else(|| path.strip_suffix("-release"))
    {
        return format!("{prefix}-{suffix}");
    }
    if let Some(prefix) = path
        .strip_suffix("/debug")
        .or_else(|| path.strip_suffix("/release"))
    {
        return format!("{prefix}/{suffix}");
    }
    if let Some(prefix) = path
        .strip_suffix("\\debug")
        .or_else(|| path.strip_suffix("\\release"))
    {
        return format!("{prefix}\\{suffix}");
    }
    path.to_string()
}

fn profile_label(base: &str, build_type: &str) -> String {
    format!("{base} ({build_type})")
}

fn profile_target_label(base: &str, build_type: &str, target: &str) -> String {
    format!("{base} ({build_type}): {target}")
}

fn build_command_for_target(profiled: &ProfiledBuild, target: &str) -> String {
    let base_command = profiled
        .build
        .clone()
        .unwrap_or_else(|| format!("cmake --build {}", profiled.build_dir));
    let target_arg = target_argument(target);
    let config_arg = format!(" --config {}", profiled.build_type);
    if let Some((prefix, suffix)) = base_command.rsplit_once("cmake --build ") {
        let config_arg = if suffix.contains(" --config ") {
            ""
        } else {
            config_arg.as_str()
        };
        return format!("{prefix}cmake --build {suffix}{config_arg} --target {target_arg}");
    }
    if base_command.contains(" --config ") {
        format!("{base_command} --target {target_arg}")
    } else {
        format!("{base_command}{config_arg} --target {target_arg}")
    }
}

fn target_argument(target: &str) -> String {
    if target_needs_shell_quotes(target) {
        format!("\"{}\"", target.replace('"', "\"\""))
    } else {
        target.to_string()
    }
}

fn target_needs_shell_quotes(target: &str) -> bool {
    target.chars().any(|character| {
        character.is_whitespace()
            || matches!(character, '"' | '\'' | '&' | '|' | '<' | '>' | '(' | ')')
    })
}

fn run_command_for_target(build_dir: &str, output: &str, shell: ShellKind) -> String {
    let output = output.replace('/', "\\");
    match shell {
        ShellKind::Powershell => {
            format!(
                "Start-Process -FilePath \"$ZED_WORKTREE_ROOT\\{}\\{}\"",
                build_dir, output
            )
        }
        ShellKind::Sh => {
            format!(
                "\"$ZED_WORKTREE_ROOT/{}/{}\"",
                build_dir,
                output.replace('\\', "/")
            )
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

        assert_eq!(parsed.as_array().unwrap().len(), 6);
        assert_eq!(parsed[0]["label"], "C++: Configure (Debug)");
        assert_eq!(parsed[1]["label"], "C++: Build (Debug)");
        assert_eq!(parsed[2]["label"], "C++: Clean (Debug)");
        assert_eq!(parsed[3]["label"], "C++: Configure (Release)");
        assert_eq!(parsed[4]["label"], "C++: Build (Release)");
        assert_eq!(parsed[5]["label"], "C++: Clean (Release)");
        assert_eq!(parsed[1]["command"], "cmake --build build/debug");
        assert_eq!(parsed[4]["command"], "cmake --build build/release");
        assert!(parsed[1]["args"].as_array().unwrap().is_empty());
    }

    #[test]
    fn generates_debug_and_release_task_profiles_for_cmake_targets() {
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

        assert!(task_labels(&parsed).contains(&"C++: Configure (Debug)"));
        assert!(task_labels(&parsed).contains(&"C++: Configure (Release)"));
        assert!(task_labels(&parsed).contains(&"C++: Build (Debug)"));
        assert!(task_labels(&parsed).contains(&"C++: Build (Release)"));
        assert!(task_labels(&parsed).contains(&"C++: Run (Debug): myapp"));
        assert!(task_labels(&parsed).contains(&"C++: Run (Release): myapp"));

        let debug_configure = task_with_label(&parsed, "C++: Configure (Debug)");
        assert!(
            debug_configure["command"]
                .as_str()
                .unwrap()
                .contains("-B build/debug")
        );
        assert!(
            debug_configure["command"]
                .as_str()
                .unwrap()
                .contains("-DCMAKE_BUILD_TYPE=Debug")
        );

        let release_configure = task_with_label(&parsed, "C++: Configure (Release)");
        assert!(
            release_configure["command"]
                .as_str()
                .unwrap()
                .contains("-B build/release")
        );
        assert!(
            release_configure["command"]
                .as_str()
                .unwrap()
                .contains("-DCMAKE_BUILD_TYPE=Release")
        );

        let release_run = task_with_label(&parsed, "C++: Run (Release): myapp");
        assert!(
            release_run["command"]
                .as_str()
                .unwrap()
                .contains("build/release/myapp.exe")
        );
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

        assert_eq!(parsed.as_array().unwrap().len(), 10);
        let run_task = task_with_label(&parsed, "C++: Run (Debug): myapp");
        assert!(
            run_task["command"]
                .as_str()
                .unwrap()
                .contains("build/debug/myapp.exe")
        );
    }

    #[test]
    fn creates_build_tasks_for_cmake_library_and_executable_targets() {
        let config = resolve_config(Some(UserConfig {
            preset: Some("gcc-cmake-ninja".to_string()),
            ..UserConfig::default()
        }))
        .unwrap();
        let targets = vec![
            CmakeTarget {
                name: "QEnhancedCustomPlot".to_string(),
                output: Some("QEnhancedCustomPlot.lib".to_string()),
                executable: false,
            },
            CmakeTarget {
                name: "demo_realtime".to_string(),
                output: Some("demos/demo_realtime/demo_realtime.exe".to_string()),
                executable: true,
            },
        ];
        let json = generate_cpp_tasks_json(&config, ShellKind::Powershell, &targets).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        let labels = parsed
            .as_array()
            .unwrap()
            .iter()
            .map(|task| task["label"].as_str().unwrap())
            .collect::<Vec<_>>();

        assert!(labels.contains(&"C++: Build Target (Debug): QEnhancedCustomPlot"));
        assert!(labels.contains(&"C++: Build Target (Release): QEnhancedCustomPlot"));
        assert!(labels.contains(&"C++: Build Target (Debug): demo_realtime"));
        assert!(labels.contains(&"C++: Run (Debug): demo_realtime"));
        assert!(labels.contains(&"C++: Run (Release): demo_realtime"));
        assert!(!labels.contains(&"C++: Run (Debug): QEnhancedCustomPlot"));
        assert!(!labels.contains(&"C++: Run (Release): QEnhancedCustomPlot"));
        assert!(parsed.as_array().unwrap().iter().any(|task| {
            task["label"] == "C++: Build Target (Debug): QEnhancedCustomPlot"
                && task["args"][2].as_str().unwrap().contains(
                    "cmake --build build/debug --config Debug --target QEnhancedCustomPlot",
                )
        }));
    }

    #[test]
    fn keeps_cmake_build_target_tasks_when_run_command_is_explicit() {
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
            name: "QEnhancedCustomPlot".to_string(),
            output: Some("QEnhancedCustomPlot.lib".to_string()),
            executable: false,
        }];
        let json = generate_cpp_tasks_json(&config, ShellKind::Sh, &targets).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(task_labels(&parsed).contains(&"C++: Build Target (Debug): QEnhancedCustomPlot"));
        assert!(task_labels(&parsed).contains(&"C++: Build Target (Release): QEnhancedCustomPlot"));
        assert_eq!(run_task_count(&parsed), 2);
        assert!(task_labels(&parsed).contains(&"C++: Run (Debug)"));
        assert!(task_labels(&parsed).contains(&"C++: Run (Release)"));
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

        // Only explicit run tasks are generated for each profile, not auto-discovered run targets.
        assert_eq!(parsed.as_array().unwrap().len(), 10);
        assert_eq!(run_task_count(&parsed), 2);
        assert_eq!(run_task_target_count(&parsed), 0);
        let run_task = task_with_label(&parsed, "C++: Run (Debug)");
        assert!(
            run_task["command"]
                .as_str()
                .unwrap()
                .contains("my-custom-app")
        );
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

        assert_eq!(parsed.as_array().unwrap().len(), 12);
        assert!(task_labels(&parsed).contains(&"C++: Build Target (Debug): mylib"));
        assert!(task_labels(&parsed).contains(&"C++: Build Target (Debug): myapp"));
        assert!(task_labels(&parsed).contains(&"C++: Run (Debug): myapp"));
        assert!(!task_labels(&parsed).contains(&"C++: Run (Debug): mylib"));
    }

    fn task_labels(parsed: &serde_json::Value) -> Vec<&str> {
        parsed
            .as_array()
            .unwrap()
            .iter()
            .map(|task| task["label"].as_str().unwrap())
            .collect()
    }

    fn task_with_label<'a>(parsed: &'a serde_json::Value, label: &str) -> &'a serde_json::Value {
        parsed
            .as_array()
            .unwrap()
            .iter()
            .find(|task| task["label"] == label)
            .unwrap()
    }

    fn run_task_count(parsed: &serde_json::Value) -> usize {
        task_labels(parsed)
            .iter()
            .filter(|label| label.starts_with("C++: Run"))
            .count()
    }

    fn run_task_target_count(parsed: &serde_json::Value) -> usize {
        task_labels(parsed)
            .iter()
            .filter(|label| label.starts_with("C++: Run (") && label.contains("): "))
            .count()
    }
}
