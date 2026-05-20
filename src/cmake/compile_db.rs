//! CMake 编译数据库探测。
//!
//! V0.2 实现 compile_commands.json 探测。

use std::path::Path;

pub const COMPILE_COMMANDS_JSON: &str = "compile_commands.json";

/// 探测编译数据库路径。
///
/// 搜索顺序：
/// 1. 工作区根目录
/// 2. build/ 子目录
pub fn discover_compile_database(root_path: &str) -> Option<String> {
    let root = Path::new(root_path);

    // 先检查根目录
    let root_db = root.join(COMPILE_COMMANDS_JSON);
    if root_db.exists() {
        return root_db.to_str().map(String::from);
    }

    // 再检查 build/ 子目录
    let build_db = root.join("build").join(COMPILE_COMMANDS_JSON);
    if build_db.exists() {
        return build_db.to_str().map(String::from);
    }

    None
}

/// 探测 CMakeLists.txt 是否存在。
pub fn has_cmake_lists(root_path: &str) -> bool {
    Path::new(root_path).join("CMakeLists.txt").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// 创建一个临时测试目录，并在测试完成后清理。
    fn with_temp_dir<F>(f: F)
    where
        F: FnOnce(&std::path::Path),
    {
        // 使用系统临时目录创建唯一的子目录
        let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_dir = std::env::temp_dir().join(format!(
            "zed-msvc-test-{}-{}",
            std::process::id(),
            test_id
        ));
        fs::create_dir_all(&temp_dir).unwrap();

        // 执行测试
        f(&temp_dir);

        // 清理
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn finds_compile_commands_in_root() {
        with_temp_dir(|root| {
            fs::write(root.join(COMPILE_COMMANDS_JSON), r#"[]"#).unwrap();

            let found = discover_compile_database(root.to_str().unwrap());

            assert_eq!(
                found,
                Some(root.join(COMPILE_COMMANDS_JSON).to_str().unwrap().to_string())
            );
        });
    }

    #[test]
    fn finds_compile_commands_in_build_subdirectory() {
        with_temp_dir(|root| {
            let build_dir = root.join("build");
            fs::create_dir_all(&build_dir).unwrap();
            fs::write(build_dir.join(COMPILE_COMMANDS_JSON), r#"[]"#).unwrap();

            let found = discover_compile_database(root.to_str().unwrap());

            assert_eq!(
                found,
                Some(build_dir.join(COMPILE_COMMANDS_JSON).to_str().unwrap().to_string())
            );
        });
    }

    #[test]
    fn prefers_root_over_build_directory() {
        with_temp_dir(|root| {
            let build_dir = root.join("build");
            fs::create_dir_all(&build_dir).unwrap();
            fs::write(root.join(COMPILE_COMMANDS_JSON), r#"[]"#).unwrap();
            fs::write(build_dir.join(COMPILE_COMMANDS_JSON), r#"[]"#).unwrap();

            let found = discover_compile_database(root.to_str().unwrap());

            // 应该返回根目录的版本
            assert_eq!(
                found,
                Some(root.join(COMPILE_COMMANDS_JSON).to_str().unwrap().to_string())
            );
        });
    }

    #[test]
    fn returns_none_when_compile_commands_missing() {
        with_temp_dir(|root| {
            let found = discover_compile_database(root.to_str().unwrap());
            assert!(found.is_none());
        });
    }

    #[test]
    fn detects_cmake_project() {
        with_temp_dir(|root| {
            fs::write(root.join("CMakeLists.txt"), "cmake_minimum_required(VERSION 3.10)").unwrap();

            assert!(has_cmake_lists(root.to_str().unwrap()));
        });
    }

    #[test]
    fn returns_false_for_non_cmake_project() {
        with_temp_dir(|root| {
            assert!(!has_cmake_lists(root.to_str().unwrap()));
        });
    }
}
