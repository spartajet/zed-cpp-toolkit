//! neocmakelsp binary discovery.
//!
//! Searches for neocmakelsp in PATH. Does not auto-download.

use crate::debug::log_message;
use crate::error::{ToolkitError, ToolkitResult};
use zed_extension_api as zed;

const BINARY_NAME: &str = "neocmakelsp";

/// Finds neocmakelsp in PATH.
pub fn find_binary(worktree: &zed::Worktree) -> ToolkitResult<String> {
    if let Some(path) = worktree.which(BINARY_NAME) {
        log_message(&format!("found neocmakelsp in PATH: {path}"));
        return Ok(path);
    }

    log_message("neocmakelsp not found in PATH");
    Err(ToolkitError::MissingNeocmakelsp)
}
