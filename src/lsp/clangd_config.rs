use crate::paths::clangd_include_arg;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClangdConfigInput {
    pub msvc_include: String,
    pub sdk_includes: Vec<String>,
}

pub fn render_clangd_config(input: &ClangdConfigInput) -> String {
    let mut output = String::new();
    output.push_str("# 由 Zed MSVC C++ Assistant 自动生成。\n");
    output
        .push_str("# 如果需要自定义 clangd 行为，请编辑本文件；插件 V0.1 不会覆盖已有 .clangd。\n");
    output.push_str("CompileFlags:\n");
    output.push_str("  DriverMode: cl\n");
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
        });

        assert!(rendered.contains("- /IC:/VS/VC/Tools/MSVC/14.40.33807/include"));
        assert!(rendered.contains("Windows SDK include 未自动探测到"));
        assert!(
            rendered
                .contains("# - /IC:/Program Files (x86)/Windows Kits/10/Include/<version>/ucrt")
        );
    }
}
