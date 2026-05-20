//! Debug adapter 集成模块边界。
//!
//! V0.1 不注册或启动 vsdbg。

/// 追加一行诊断日志到 stderr。
///
/// 日志会显示在 Zed 的 LSP logs 窗体中（dev: open language server logs）。
/// 使用 eprintln! 直接输出到 stderr，避免启动额外的 PowerShell 子进程。
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
