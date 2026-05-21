//! neocmakelsp server command builder.

use crate::debug::log_message;
use crate::error::{ToolkitError, ToolkitResult};
use crate::lsp::neocmake::download::get_or_download_binary;
use zed_extension_api as zed;

pub const LANGUAGE_SERVER_ID: &str = "msvc-cmake-neocmake";

/// Validates neocmake language server ID.
pub fn validate_language_server_id(id: &str) -> ToolkitResult<()> {
    if id == LANGUAGE_SERVER_ID {
        Ok(())
    } else {
        Err(ToolkitError::UnsupportedLanguageServer(id.to_string()))
    }
}

/// Builds neocmakelsp command.
pub fn command_from_worktree(
    worktree: &zed::Worktree,
    language_server_id: &zed::LanguageServerId,
) -> ToolkitResult<zed::Command> {
    log_message("building neocmakelsp command");

    let binary_path = get_or_download_binary(worktree, language_server_id)?;
    log_message(&format!("neocmakelsp binary: {binary_path}"));

    Ok(build_neocmakelsp_command(binary_path))
}

fn build_neocmakelsp_command(binary_path: String) -> zed::Command {
    zed::Command {
        command: binary_path,
        args: vec!["stdio".to_string()],
        env: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_neocmake_language_server_id() {
        assert_eq!(validate_language_server_id("msvc-cmake-neocmake"), Ok(()));
    }

    #[test]
    fn rejects_unexpected_language_server_id() {
        let error = validate_language_server_id("other-lsp").unwrap_err();
        assert!(matches!(error, ToolkitError::UnsupportedLanguageServer(_)));
    }

    #[test]
    fn builds_stdio_command_without_cli_init_options() {
        let command = build_neocmakelsp_command("C:\\tools\\neocmakelsp.exe".to_string());

        assert_eq!(command.command, "C:\\tools\\neocmakelsp.exe");
        assert_eq!(command.args, vec!["stdio"]);
        assert!(command.env.is_empty());
    }
}
