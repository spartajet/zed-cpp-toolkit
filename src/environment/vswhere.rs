use crate::environment::tools::{CommandRunner, ensure_success};
use crate::error::{ToolkitError, ToolkitResult};

pub const VSWHERE_PATH: &str =
    r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe";

pub fn discover_visual_studio(runner: &impl CommandRunner) -> ToolkitResult<String> {
    let args = vec![
        "-latest".to_string(),
        "-version".to_string(),
        "[17.0,)".to_string(),
        "-property".to_string(),
        "installationPath".to_string(),
    ];
    let output = runner
        .run_command(VSWHERE_PATH, &args)
        .map_err(|_| ToolkitError::MissingVswhere)?;
    let stdout = ensure_success(VSWHERE_PATH, output).map_err(|error| match error {
        ToolkitError::ProcessFailed { .. } => ToolkitError::MissingVisualStudio,
        other => other,
    })?;
    parse_installation_path(&stdout).ok_or(ToolkitError::MissingVisualStudio)
}

pub fn parse_installation_path(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_first_non_empty_installation_path() {
        let parsed = parse_installation_path(
            "\r\nC:\\Program Files\\Microsoft Visual Studio\\2022\\Community\r\n",
        );

        assert_eq!(
            parsed,
            Some("C:\\Program Files\\Microsoft Visual Studio\\2022\\Community".to_string())
        );
    }

    struct FakeRunner {
        stdout: String,
        calls: std::cell::RefCell<Vec<(String, Vec<String>)>>,
    }

    impl CommandRunner for FakeRunner {
        fn run_command(
            &self,
            command: &str,
            args: &[String],
        ) -> ToolkitResult<crate::environment::tools::CommandOutput> {
            self.calls
                .borrow_mut()
                .push((command.to_string(), args.to_vec()));
            Ok(crate::environment::tools::CommandOutput {
                status: Some(0),
                stdout: self.stdout.clone(),
                stderr: String::new(),
            })
        }
    }

    #[test]
    fn discovers_visual_studio_from_vswhere_output() {
        let discovered = discover_visual_studio(&FakeRunner {
            stdout: "C:\\VS\\2022\\Community\n".to_string(),
            calls: std::cell::RefCell::new(Vec::new()),
        })
        .unwrap();

        assert_eq!(discovered, "C:\\VS\\2022\\Community");
    }

    #[test]
    fn invokes_vswhere_for_latest_visual_studio_2022_or_newer() {
        let runner = FakeRunner {
            stdout: "C:\\VS\\2022\\Community\n".to_string(),
            calls: std::cell::RefCell::new(Vec::new()),
        };

        let _ = discover_visual_studio(&runner).unwrap();
        let calls = runner.calls.borrow();

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, VSWHERE_PATH);
        assert_eq!(
            calls[0].1,
            vec!["-latest", "-version", "[17.0,)", "-property", "installationPath"]
        );
    }
}
