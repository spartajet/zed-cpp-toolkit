//! neocmakelsp LSP integration module.
//!
//! Provides CMake language support via neocmakelsp, supporting dual installation
//! (PATH + GitHub download) and dual configuration (.neocmake.toml + settings.json).

pub mod config;
pub mod download;
pub mod init_options;
pub mod server;

// Convenience exports (callable via lsp::neocmake::command_from_worktree)
#[allow(dead_code)]
#[allow(unused_imports)]
pub use server::command_from_worktree;
