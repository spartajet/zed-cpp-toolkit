use crate::paths::clangd_include_arg;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClangdConfigInput {
    pub msvc_include: String,
    pub sdk_includes: Vec<String>,
    pub compile_database_path: Option<String>,
}

pub fn render_clangd_config(input: &ClangdConfigInput) -> String {
    let mut output = String::new();
    output.push_str("# 由 Zed MSVC C++ Assistant 自动生成。\n");
    output
        .push_str("# 如果需要自定义 clangd 行为，请编辑本文件；插件 V0.2 不会覆盖已有 .clangd。\n");

    // 如果有编译数据库，优先使用
    if let Some(db_path) = &input.compile_database_path {
        output.push_str("# 检测到 compile_commands.json，使用编译数据库。\n");
        output.push_str("CompileFlags:\n");
        output.push_str(&format!(
            "  CompilationDatabase: {}\n",
            db_path.replace('\\', "/")
        ));
        output.push_str("  # 编译数据库包含完整 include 路径，以下仅作为备用。\n");
        output.push_str("  DriverMode: cl\n");
    } else {
        output.push_str("CompileFlags:\n");
        output.push_str("  DriverMode: cl\n");
    }

    output.push_str("  Add:\n");
    output.push_str(&format!(
        "    - {}\n",
        clangd_include_arg(&input.msvc_include)
    ));

    if input.sdk_includes.is_empty() {
        output.push_str("    # Windows SDK include 未自动探测到；如有需要，请手动添加 /I...\n");
        output
            .push_str("    # - /IC:/Program Files (x86)/Windows Kits/10/Include/<version>/ucrt\n");
        output.push_str("    # - /IC:/Program Files (x86)/Windows Kits/10/Include/<version>/um\n");
        output.push_str(
            "    # - /IC:/Program Files (x86)/Windows Kits/10/Include/<version>/shared\n",
        );
    } else {
        for include in &input.sdk_includes {
            output.push_str(&format!("    - {}\n", clangd_include_arg(include)));
        }
    }

    output.push_str("Diagnostics:\n");
    output.push_str("  Suppress: ['pp_file_not_found']\n");
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_msvc_and_sdk_include_paths() {
        let rendered = render_clangd_config(&ClangdConfigInput {
            msvc_include: r"C:\VS\VC\Tools\MSVC\14.40.33807\include".to_string(),
            sdk_includes: vec![
                r"C:\Windows Kits\10\Include\10.0.22621.0\ucrt".to_string(),
                r"C:\Windows Kits\10\Include\10.0.22621.0\um".to_string(),
                r"C:\Windows Kits\10\Include\10.0.22621.0\shared".to_string(),
            ],
            compile_database_path: None,
        });

        assert!(rendered.contains("DriverMode: cl"));
        assert!(rendered.contains("- /IC:/VS/VC/Tools/MSVC/14.40.33807/include"));
        assert!(rendered.contains("- /IC:/Windows Kits/10/Include/10.0.22621.0/ucrt"));
        assert!(rendered.contains("- /IC:/Windows Kits/10/Include/10.0.22621.0/um"));
        assert!(rendered.contains("- /IC:/Windows Kits/10/Include/10.0.22621.0/shared"));
        assert!(!rendered.contains("Windows SDK include 未自动探测到"));
    }

    #[test]
    fn renders_manual_sdk_comments_when_sdk_is_missing() {
        let rendered = render_clangd_config(&ClangdConfigInput {
            msvc_include: r"C:\VS\VC\Tools\MSVC\14.40.33807\include".to_string(),
            sdk_includes: Vec::new(),
            compile_database_path: None,
        });

        assert!(rendered.contains("- /IC:/VS/VC/Tools/MSVC/14.40.33807/include"));
        assert!(rendered.contains("Windows SDK include 未自动探测到"));
        assert!(
            rendered
                .contains("# - /IC:/Program Files (x86)/Windows Kits/10/Include/<version>/ucrt")
        );
    }

    #[test]
    fn renders_compilation_database_when_present() {
        let rendered = render_clangd_config(&ClangdConfigInput {
            msvc_include: r"C:\VS\VC\Tools\MSVC\14.40.33807\include".to_string(),
            sdk_includes: Vec::new(),
            compile_database_path: Some(r"C:\project\build".to_string()),
        });

        assert!(rendered.contains("检测到 compile_commands.json，使用编译数据库"));
        assert!(rendered.contains("CompilationDatabase: C:/project/build"));
        assert!(rendered.contains("编译数据库包含完整 include 路径，以下仅作为备用"));
    }

    #[test]
    fn does_not_render_compilation_database_when_missing() {
        let rendered = render_clangd_config(&ClangdConfigInput {
            msvc_include: r"C:\VS\VC\Tools\MSVC\14.40.33807\include".to_string(),
            sdk_includes: Vec::new(),
            compile_database_path: None,
        });

        assert!(!rendered.contains("CompilationDatabase:"));
        assert!(!rendered.contains("检测到 compile_commands.json"));
    }
}
