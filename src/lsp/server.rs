use crate::environment::discover_msvc_environment;
use crate::environment::tools::{ZedCommandRunner, require_clangd};
use crate::error::{ToolkitError, ToolkitResult};
use crate::lsp::clangd_config::ClangdConfigInput;
use crate::lsp::workspace_config::{ClangdFileDecision, decide_clangd_file};
use zed_extension_api as zed;

pub const LANGUAGE_SERVER_ID: &str = "msvc-cpp-clangd";

pub fn clangd_args() -> Vec<String> {
    vec!["--header-insertion=never".to_string()]
}

pub fn validate_language_server_id(id: &str) -> ToolkitResult<()> {
    if id == LANGUAGE_SERVER_ID {
        Ok(())
    } else {
        Err(ToolkitError::UnsupportedLanguageServer(id.to_string()))
    }
}

pub fn build_clangd_command(command: String, env: Vec<(String, String)>) -> zed::Command {
    zed::Command {
        command,
        args: clangd_args(),
        env,
    }
}

pub fn command_from_worktree(worktree: &zed::Worktree) -> ToolkitResult<zed::Command> {
    let clangd = require_clangd(worktree.which("clangd"))?;
    Ok(build_clangd_command(clangd, worktree.shell_env()))
}

pub fn prepare_workspace_config(
    root_path: &str,
    existing_clangd: Option<String>,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<()> {
    if existing_clangd.is_some() {
        return Ok(());
    }

    let environment = discover_msvc_environment(runner)?;
    let input = ClangdConfigInput {
        msvc_include: environment.msvc_include,
        sdk_includes: environment.sdk_includes,
    };

    match decide_clangd_file(root_path, None, &input) {
        ClangdFileDecision::Create { contents, .. } => {
            Err(ToolkitError::MissingWorkspaceConfig(contents))
        }
        ClangdFileDecision::PreserveExisting { .. } => Ok(()),
    }
}

pub fn prepare_workspace_config_from_worktree(worktree: &zed::Worktree) -> ToolkitResult<()> {
    prepare_workspace_config(
        &worktree.root_path(),
        worktree.read_text_file(".clangd").ok(),
        &ZedCommandRunner,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::tools::CommandOutput;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    #[test]
    fn clangd_args_disable_header_insertion() {
        assert_eq!(clangd_args(), vec!["--header-insertion=never"]);
    }

    #[test]
    fn accepts_expected_language_server_id() {
        assert_eq!(validate_language_server_id("msvc-cpp-clangd"), Ok(()));
    }

    #[test]
    fn rejects_unexpected_language_server_id() {
        let error = validate_language_server_id("other-lsp").unwrap_err();

        assert_eq!(
            error,
            ToolkitError::UnsupportedLanguageServer("other-lsp".to_string())
        );
    }

    #[test]
    fn skips_environment_discovery_when_clangd_config_exists() {
        struct PanickingRunner;

        impl crate::environment::tools::CommandRunner for PanickingRunner {
            fn run_command(
                &self,
                _command: &str,
                _args: &[String],
            ) -> ToolkitResult<crate::environment::tools::CommandOutput> {
                panic!("runner should not be used when .clangd exists");
            }
        }

        assert_eq!(
            prepare_workspace_config(
                "C:/repo",
                Some("CompileFlags: {}".to_string()),
                &PanickingRunner
            ),
            Ok(())
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

    impl crate::environment::tools::CommandRunner for QueueRunner {
        fn run_command(&self, _command: &str, _args: &[String]) -> ToolkitResult<CommandOutput> {
            self.outputs
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| ToolkitError::IoMessage("unexpected command".to_string()))
        }
    }

    #[test]
    fn reports_generated_config_when_clangd_config_is_missing() {
        let runner = QueueRunner::new([
            CommandOutput {
                status: Some(0),
                stdout: "C:\\VS\\2022\\Community\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "14.38.33130\n14.40.33807\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "10.0.19041.0\n10.0.22621.0\n".to_string(),
                stderr: String::new(),
            },
        ]);

        let error = prepare_workspace_config("C:/repo", None, &runner).unwrap_err();

        match error {
            ToolkitError::MissingWorkspaceConfig(contents) => {
                assert!(contents.contains("DriverMode: cl"));
                assert!(
                    contents.contains("/IC:/VS/2022/Community/VC/Tools/MSVC/14.40.33807/include")
                );
                assert!(contents.contains(
                    "/IC:/Program Files (x86)/Windows Kits/10/Include/10.0.22621.0/ucrt"
                ));
            }
            other => panic!("expected MissingWorkspaceConfig, got {other:?}"),
        }
    }
}
