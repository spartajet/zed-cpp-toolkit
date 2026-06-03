//! neocmakelsp LSP integration module.
//!
//! Provides CMake language support via neocmakelsp discovered from PATH.
//! Initialization options are read from Zed settings.json.

pub mod config;
pub mod download;
pub mod init_options;
pub mod server;

// Convenience exports (callable via lsp::neocmake::command_from_worktree)
#[allow(dead_code)]
#[allow(unused_imports)]
pub use server::command_from_worktree;
