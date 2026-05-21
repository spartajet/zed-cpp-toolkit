use crate::error::{ToolkitError, ToolkitResult};
use zed_extension_api as zed;

/// Generic tool discovery function.
///
/// Checks if a tool path is valid (non-empty string).
#[allow(dead_code)]
pub fn require_tool(tool_path: Option<String>) -> ToolkitResult<String> {
    tool_path.ok_or_else(|| ToolkitError::MissingTool(String::from("unknown")))
}

pub fn require_clangd(clangd_path: Option<String>) -> ToolkitResult<String> {
    clangd_path.ok_or(ToolkitError::MissingClangd)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait CommandRunner {
    fn run_command(&self, command: &str, args: &[String]) -> ToolkitResult<CommandOutput>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ZedCommandRunner;

impl CommandRunner for ZedCommandRunner {
    fn run_command(&self, command: &str, args: &[String]) -> ToolkitResult<CommandOutput> {
        crate::debug::log_message(&format!("running external command: {command} {args:?}"));
        let mut command = zed::Command {
            command: command.to_string(),
            args: args.to_vec(),
            env: Vec::new(),
        };
        let output = command.output().map_err(|message| {
            crate::debug::log_message(&format!("external command spawn failed: {message}"));
            ToolkitError::IoMessage(message)
        })?;
        crate::debug::log_message(&format!(
            "external command finished: status={:?}, stdout_bytes={}, stderr_bytes={}",
            output.status,
            output.stdout.len(),
            output.stderr.len()
        ));
        Ok(CommandOutput {
            status: output.status,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

pub fn ensure_success(command: &str, output: CommandOutput) -> ToolkitResult<String> {
    if output.status == Some(0) {
        crate::debug::log_message(&format!("{command} succeeded"));
        Ok(output.stdout)
    } else {
        crate::debug::log_message(&format!(
            "{command} failed: status={:?}, stderr={}",
            output.status,
            output.stderr.trim()
        ));
        Err(ToolkitError::ProcessFailed {
            command: command.to_string(),
            status: output.status,
            stderr: output.stderr,
        })
    }
}

pub fn powershell_list_directory_names(
    runner: &impl CommandRunner,
    path: &str,
) -> ToolkitResult<Vec<String>> {
    let escaped = path.replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference='Stop'; Get-ChildItem -LiteralPath '{escaped}' -Directory | Select-Object -ExpandProperty Name"
    );
    let args = vec!["-NoProfile".to_string(), "-Command".to_string(), script];
    let stdout = ensure_success("powershell", runner.run_command("powershell", &args)?)?;
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_existing_clangd_path() {
        let path = require_clangd(Some(r"C:\LLVM\bin\clangd.exe".to_string()));

        assert_eq!(path, Ok(r"C:\LLVM\bin\clangd.exe".to_string()));
    }

    #[test]
    fn reports_missing_clangd() {
        let error = require_clangd(None).unwrap_err();

        assert_eq!(error, ToolkitError::MissingClangd);
    }

    struct FakeRunner {
        output: CommandOutput,
        calls: std::cell::RefCell<Vec<(String, Vec<String>)>>,
    }

    impl CommandRunner for FakeRunner {
        fn run_command(&self, command: &str, args: &[String]) -> ToolkitResult<CommandOutput> {
            self.calls
                .borrow_mut()
                .push((command.to_string(), args.to_vec()));
            Ok(self.output.clone())
        }
    }

    #[test]
    fn parses_directory_names_from_powershell_output() {
        let runner = FakeRunner {
            output: CommandOutput {
                status: Some(0),
                stdout: "14.38.33130\r\n14.40.33807\r\n".to_string(),
                stderr: String::new(),
            },
            calls: std::cell::RefCell::new(Vec::new()),
        };

        let names = powershell_list_directory_names(&runner, r"C:\MSVC").unwrap();

        assert_eq!(names, vec!["14.38.33130", "14.40.33807"]);
    }

    #[test]
    fn invokes_powershell_with_literal_path_directory_listing() {
        let runner = FakeRunner {
            output: CommandOutput {
                status: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
            calls: std::cell::RefCell::new(Vec::new()),
        };

        let _ = powershell_list_directory_names(&runner, r"C:\Program Files\SDK").unwrap();
        let calls = runner.calls.borrow();

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "powershell");
        assert_eq!(calls[0].1[0], "-NoProfile");
        assert_eq!(calls[0].1[1], "-Command");
        assert!(
            calls[0].1[2]
                .contains("Get-ChildItem -LiteralPath 'C:\\Program Files\\SDK' -Directory")
        );
    }
}
