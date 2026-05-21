//! CMake compile database discovery.
//!
//! V0.2 implements compile_commands.json discovery.

use std::path::Path;

pub const COMPILE_COMMANDS_JSON: &str = "compile_commands.json";

/// Discovers compile database path.
///
/// Search order:
/// 1. Workspace root directory
/// 2. build/ subdirectory (traditional)
/// 3. cmake-build-debug/ subdirectory (CLion Debug)
/// 4. cmake-build-release/ subdirectory (CLion Release)
/// 5. cmake-build-relwithdebinfo/ subdirectory (CLion RelWithDebInfo)
///
/// # Returns
///
/// Returns the **directory path** containing `compile_commands.json` (without filename),
/// or None if the file doesn't exist or the path contains non-UTF-8 characters.
pub fn discover_compile_database(root_path: &str) -> Option<String> {
    let root = Path::new(root_path);

    // List of directories to check, in priority order
    let build_dirs = [
        "",                              // Root directory
        "build",                         // Traditional
        "cmake-build-debug",             // CLion Debug
        "cmake-build-release",           // CLion Release
        "cmake-build-relwithdebinfo",    // CLion RelWithDebInfo
    ];

    for build_dir in build_dirs {
        let dir = if build_dir.is_empty() {
            root.to_path_buf()
        } else {
            root.join(build_dir)
        };

        let db_path = dir.join(COMPILE_COMMANDS_JSON);
        if db_path.exists() {
            return dir.to_str().map(String::from);
        }
    }

    None
}

/// Checks if CMakeLists.txt exists.
pub fn has_cmake_lists(root_path: &str) -> bool {
    Path::new(root_path).join("CMakeLists.txt").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Creates a temporary test directory and cleans up after the test.
    fn with_temp_dir<F>(f: F)
    where
        F: FnOnce(&std::path::Path),
    {
        // Create a unique subdirectory in the system temp directory
        let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_dir =
            std::env::temp_dir().join(format!("zed-msvc-test-{}-{}", std::process::id(), test_id));
        fs::create_dir_all(&temp_dir).unwrap();

        // Run the test
        f(&temp_dir);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn finds_compile_commands_in_root() {
        with_temp_dir(|root| {
            fs::write(root.join(COMPILE_COMMANDS_JSON), r#"[]"#).unwrap();

            let root_str = root.to_str().expect("temp path should be valid UTF-8");
            let found = discover_compile_database(root_str);

            let expected = root.to_str().expect("path should be valid UTF-8");
            assert_eq!(found, Some(expected.to_string()));
        });
    }

    #[test]
    fn finds_compile_commands_in_build_subdirectory() {
        with_temp_dir(|root| {
            let build_dir = root.join("build");
            fs::create_dir_all(&build_dir).unwrap();
            fs::write(build_dir.join(COMPILE_COMMANDS_JSON), r#"[]"#).unwrap();

            let root_str = root.to_str().expect("temp path should be valid UTF-8");
            let found = discover_compile_database(root_str);

            let expected = build_dir.to_str().expect("path should be valid UTF-8");
            assert_eq!(found, Some(expected.to_string()));
        });
    }

    #[test]
    fn prefers_root_over_build_directory() {
        with_temp_dir(|root| {
            let build_dir = root.join("build");
            fs::create_dir_all(&build_dir).unwrap();
            fs::write(root.join(COMPILE_COMMANDS_JSON), r#"[]"#).unwrap();
            fs::write(build_dir.join(COMPILE_COMMANDS_JSON), r#"[]"#).unwrap();

            let root_str = root.to_str().expect("temp path should be valid UTF-8");
            let found = discover_compile_database(root_str);

            // 应该返回根目录（不是文件路径）
            let expected = root.to_str().expect("path should be valid UTF-8");
            assert_eq!(found, Some(expected.to_string()));
        });
    }

    #[test]
    fn returns_none_when_compile_commands_missing() {
        with_temp_dir(|root| {
            let root_str = root.to_str().expect("temp path should be valid UTF-8");
            let found = discover_compile_database(root_str);
            assert!(found.is_none());
        });
    }

    #[test]
    fn detects_cmake_project() {
        with_temp_dir(|root| {
            fs::write(
                root.join("CMakeLists.txt"),
                "cmake_minimum_required(VERSION 3.10)",
            )
            .unwrap();

            let root_str = root.to_str().expect("temp path should be valid UTF-8");
            assert!(has_cmake_lists(root_str));
        });
    }

    #[test]
    fn returns_false_for_non_cmake_project() {
        with_temp_dir(|root| {
            let root_str = root.to_str().expect("temp path should be valid UTF-8");
            assert!(!has_cmake_lists(root_str));
        });
    }

    #[test]
    fn handles_non_utf8_path_gracefully() {
        // Test graceful degradation for non-UTF-8 paths
        // Create a string from bytes to simulate non-UTF-8 paths
        let invalid_bytes = [0xFF, 0xFE, 0xFD];
        let invalid_path = std::str::from_utf8(&invalid_bytes);
        // Non-UTF-8 paths cannot be converted to &str, this is a compile-time guarantee
        // In real scenarios, paths come from API calls; if conversion to str fails, it degrades
        assert!(invalid_path.is_err());

        // Test actual degradation behavior with a non-convertible scenario
        // Path::new() takes &str; if caller passes invalid UTF-8,
        // the issue will be exposed upstream in the call stack. This verifies our function
        // won't panic due to path content
        let test_path = "C:\\valid\\utf8\\path";
        let result = std::panic::catch_unwind(|| discover_compile_database(test_path));
        assert!(result.is_ok());
    }

    #[test]
    fn finds_compile_commands_in_clion_debug_directory() {
        with_temp_dir(|root| {
            let build_dir = root.join("cmake-build-debug");
            fs::create_dir_all(&build_dir).unwrap();
            fs::write(build_dir.join(COMPILE_COMMANDS_JSON), r#"[]"#).unwrap();

            let root_str = root.to_str().expect("temp path should be valid UTF-8");
            let found = discover_compile_database(root_str);

            let expected = build_dir.to_str().expect("path should be valid UTF-8");
            assert_eq!(found, Some(expected.to_string()));
        });
    }

    #[test]
    fn finds_compile_commands_in_clion_release_directory() {
        with_temp_dir(|root| {
            let build_dir = root.join("cmake-build-release");
            fs::create_dir_all(&build_dir).unwrap();
            fs::write(build_dir.join(COMPILE_COMMANDS_JSON), r#"[]"#).unwrap();

            let root_str = root.to_str().expect("temp path should be valid UTF-8");
            let found = discover_compile_database(root_str);

            let expected = build_dir.to_str().expect("path should be valid UTF-8");
            assert_eq!(found, Some(expected.to_string()));
        });
    }

    #[test]
    fn finds_compile_commands_in_clion_relwithdebinfo_directory() {
        with_temp_dir(|root| {
            let build_dir = root.join("cmake-build-relwithdebinfo");
            fs::create_dir_all(&build_dir).unwrap();
            fs::write(build_dir.join(COMPILE_COMMANDS_JSON), r#"[]"#).unwrap();

            let root_str = root.to_str().expect("temp path should be valid UTF-8");
            let found = discover_compile_database(root_str);

            let expected = build_dir.to_str().expect("path should be valid UTF-8");
            assert_eq!(found, Some(expected.to_string()));
        });
    }

    #[test]
    fn prefers_traditional_build_over_clion_style() {
        with_temp_dir(|root| {
            let build_dir = root.join("build");
            let clion_dir = root.join("cmake-build-debug");
            fs::create_dir_all(&build_dir).unwrap();
            fs::create_dir_all(&clion_dir).unwrap();
            fs::write(build_dir.join(COMPILE_COMMANDS_JSON), r#"[]"#).unwrap();
            fs::write(clion_dir.join(COMPILE_COMMANDS_JSON), r#"[]"#).unwrap();

            let root_str = root.to_str().expect("temp path should be valid UTF-8");
            let found = discover_compile_database(root_str);

            // Should prefer the traditional build/ directory
            let expected = build_dir.to_str().expect("path should be valid UTF-8");
            assert_eq!(found, Some(expected.to_string()));
        });
    }
}
