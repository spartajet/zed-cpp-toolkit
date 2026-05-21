//! Debug adapter integration module boundary.
//!
//! V0.1 does not register or start vsdbg.

/// Appends a diagnostic log line to stderr.
///
/// Logs appear in Zed's LSP logs window (dev: open language server logs).
/// Uses eprintln! to write directly to stderr, avoiding spawning additional PowerShell subprocess.
pub fn log_message(message: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        eprintln!("[zed-msvc-toolkit] {}", message);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = message;
    }
}

pub fn log_error(context: &str, error: &crate::error::ToolkitError) {
    log_message(&format!("{context}: {}", error.user_message()));
}
