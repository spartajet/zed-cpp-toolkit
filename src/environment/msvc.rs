use crate::environment::tools::{CommandRunner, powershell_list_directory_names};
use crate::error::{ToolkitError, ToolkitResult};
use crate::paths::highest_version_dir;

pub fn select_msvc_include<'a>(
    versions: impl IntoIterator<Item = &'a str>,
    vs_root: &str,
) -> Option<String> {
    highest_version_dir(versions).map(|version| {
        format!(
            r"{vs_root}\VC\Tools\MSVC\{version}\include",
            vs_root = vs_root.trim_end_matches('\\'),
            version = version
        )
    })
}

pub fn discover_msvc_include(runner: &impl CommandRunner, vs_root: &str) -> ToolkitResult<String> {
    let tools_root = format!(
        r"{vs_root}\VC\Tools\MSVC",
        vs_root = vs_root.trim_end_matches('\\')
    );
    let versions = powershell_list_directory_names(runner, &tools_root)
        .map_err(|_| ToolkitError::MissingMsvcToolset)?;
    select_msvc_include(versions.iter().map(String::as_str), vs_root)
        .ok_or(ToolkitError::MissingMsvcToolset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::tools::{CommandOutput, CommandRunner};
    use crate::error::ToolkitError;

    #[test]
    fn selects_highest_msvc_include_path() {
        let include = select_msvc_include(
            ["14.38.33130", "14.40.33807", "14.9.99999"],
            r"C:\Program Files\Microsoft Visual Studio\2022\Community",
        );

        assert_eq!(
            include,
            Some(
                r"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.40.33807\include"
                    .to_string()
            )
        );
    }

    struct FailingRunner;

    impl CommandRunner for FailingRunner {
        fn run_command(
            &self,
            _command: &str,
            _args: &[String],
        ) -> ToolkitResult<CommandOutput> {
            Ok(CommandOutput {
                status: Some(1),
                stdout: String::new(),
                stderr: "missing directory".to_string(),
            })
        }
    }

    #[test]
    fn reports_missing_msvc_toolset_when_toolset_directory_cannot_be_listed() {
        let error = discover_msvc_include(&FailingRunner, r"C:\VS\2022\Community").unwrap_err();

        assert_eq!(error, ToolkitError::MissingMsvcToolset);
    }
}
