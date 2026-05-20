//! CMake 集成模块。
//!
//! V0.2 实现 compile_commands.json 探测。

pub mod compile_db;

pub use compile_db::{discover_compile_database, has_cmake_lists};
