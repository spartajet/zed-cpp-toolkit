use crate::config::schema::EffectiveConfig;
use crate::environment::tools::CommandRunner;
use crate::environment::vswhere::discover_visual_studio;
use crate::error::ToolkitResult;

pub fn prepare_task_config(
    config: &EffectiveConfig,
    runner: &impl CommandRunner,
) -> ToolkitResult<EffectiveConfig> {
    let visual_studio_root = discover_visual_studio(runner)?;
    let vs_dev_cmd = join_windows_path(&visual_studio_root, r"Common7\Tools\VsDevCmd.bat");
    let mut prepared = config.clone();

    prepared.build.configure = wrap_optional_command(prepared.build.configure, &vs_dev_cmd);
    prepared.build.build = wrap_optional_command(prepared.build.build, &vs_dev_cmd);
    prepared.build.clean = wrap_optional_command(prepared.build.clean, &vs_dev_cmd);
    prepared.run.command = wrap_optional_command(prepared.run.command, &vs_dev_cmd);

    Ok(prepared)
}

fn wrap_optional_command(command: Option<String>, vs_dev_cmd: &str) -> Option<String> {
    command.map(|command| developer_environment_script(vs_dev_cmd, &command))
}

fn developer_environment_script(vs_dev_cmd: &str, command: &str) -> String {
    let cmd_line = format!(
        "call {} -arch=x64 -host_arch=x64 && {command}",
        cmd_quote(vs_dev_cmd)
    );
    format!("& cmd.exe /S /C {}", powershell_single_quote(&cmd_line))
}

fn join_windows_path(root_path: &str, child: &str) -> String {
    format!(
        "{}\\{}",
        root_path.trim_end_matches(['\\', '/'].as_slice()),
        child.trim_start_matches(['\\', '/'].as_slice())
    )
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
    use crate::config::merge::resolve_config;
    use crate::config::schema::UserConfig;
    use crate::environment::tools::CommandOutput;
    use crate::error::{ToolkitError, ToolkitResult};
    use std::cell::RefCell;
    use std::collections::VecDeque;

    #[test]
    fn prepare_task_config_wraps_msvc_build_commands() {
        let config = resolve_config(Some(UserConfig {
            preset: Some("msvc-cmake-ninja".to_string()),
            ..UserConfig::default()
        }))
        .unwrap();
        let runner = QueueRunner::new([CommandOutput {
            status: Some(0),
            stdout: "C:\\VS\\2022\\Community\n".to_string(),
            stderr: String::new(),
        }]);

        let prepared = prepare_task_config(&config, &runner).unwrap();

        assert!(
            prepared
                .build
                .build
                .as_deref()
                .unwrap()
                .contains("VsDevCmd.bat")
        );
    }

    struct QueueRunner {
        outputs: RefCell<VecDeque<CommandOutput>>,
    }

    impl QueueRunner {
        fn new(outputs: impl IntoIterator<Item = CommandOutput>) -> Self {
            Self {
                outputs: RefCell::new(outputs.into_iter().collect()),
            }
        }
    }

    impl CommandRunner for QueueRunner {
        fn run_command(&self, _command: &str, _args: &[String]) -> ToolkitResult<CommandOutput> {
            self.outputs
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| ToolkitError::IoMessage("unexpected command".to_string()))
        }
    }
}
