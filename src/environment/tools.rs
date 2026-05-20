use crate::error::{ToolkitError, ToolkitResult};
use zed_extension_api as zed;

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
        let mut command = zed::Command {
            command: command.to_string(),
            args: args.to_vec(),
            env: Vec::new(),
        };
        let output = command.output().map_err(ToolkitError::IoMessage)?;
        Ok(CommandOutput {
            status: output.status,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

pub fn ensure_success(command: &str, output: CommandOutput) -> ToolkitResult<String> {
    if output.status == Some(0) {
        Ok(output.stdout)
    } else {
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
    }

    impl CommandRunner for FakeRunner {
        fn run_command(&self, _command: &str, _args: &[String]) -> ToolkitResult<CommandOutput> {
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
        };

        let names = powershell_list_directory_names(&runner, r"C:\MSVC").unwrap();

        assert_eq!(names, vec!["14.38.33130", "14.40.33807"]);
    }
}
