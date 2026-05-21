//! CMake integration module.
//!
//! V0.3 implements CMake configure/build command support.
//! V0.4 implements .zed/tasks.json generation.
//! V0.5 implements neocmakelsp CMake LSP integration.

pub mod compile_db;
pub mod tasks;
pub mod tools;

pub use compile_db::discover_compile_database;
#[allow(dead_code)]
pub use compile_db::has_cmake_lists;
pub use tasks::{CmakeTarget, TaskOptions, generate_tasks_json};
// Reserved: CMake configure/build commands (for future implementation)
#[allow(dead_code)]
pub use tools::{
    CmakeBuildType, CmakeConfigureOptions, CmakeGenerator, build_build_command,
    build_configure_command, discover_cmake, select_generator,
};
