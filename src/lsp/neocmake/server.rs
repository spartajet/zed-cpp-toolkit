//! neocmakelsp server 命令构建器。

use crate::debug::log_message;
use crate::error::{ToolkitError, ToolkitResult};
use zed_extension_api as zed;

pub const LANGUAGE_SERVER_ID: &str = "msvc-cmake-neocmake";
const BINARY_NAME: &str = "neocmakelsp";

/// 验证 neocmake language server ID。
pub fn validate_language_server_id(id: &str) -> ToolkitResult<()> {
    if id == LANGUAGE_SERVER_ID {
        Ok(())
    } else {
        Err(ToolkitError::UnsupportedLanguageServer(id.to_string()))
    }
}

/// 构建 neocmakelsp 命令。
pub fn command_from_worktree(worktree: &zed::Worktree) -> ToolkitResult<zed::Command> {
    log_message("构建 neocmakelsp 命令");

    let binary_path = require_neocmakelsp(worktree)?;
    log_message(&format!("neocmakelsp 二进制: {binary_path}"));

    Ok(build_neocmakelsp_command(binary_path))
}

fn require_neocmakelsp(worktree: &zed::Worktree) -> ToolkitResult<String> {
    worktree
        .which(BINARY_NAME)
        .ok_or(ToolkitError::MissingNeocmakelsp)
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
