use crate::lsp::clangd_config::{ClangdConfigInput, render_clangd_config};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClangdFileDecision {
    Create { path: String, contents: String },
    PreserveExisting { path: String },
}

pub fn decide_clangd_file(
    root_path: &str,
    existing_contents: Option<String>,
    input: &ClangdConfigInput,
) -> ClangdFileDecision {
    let path = format!(
        "{}/.clangd",
        root_path.replace('\\', "/").trim_end_matches('/')
    );

    if existing_contents.is_some() {
        ClangdFileDecision::PreserveExisting { path }
    } else {
        ClangdFileDecision::Create {
            path,
            contents: render_clangd_config(input),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> ClangdConfigInput {
        ClangdConfigInput {
            compiler: "clang-cl".to_string(),
            compile_commands_dir: "build".to_string(),
            extra_flags: Vec::new(),
            msvc_include: Some(r"C:\VS\VC\Tools\MSVC\14.40.33807\include".to_string()),
            sdk_includes: Vec::new(),
        }
    }

    #[test]
    fn creates_config_when_file_is_missing() {
        let decision = decide_clangd_file(r"C:\repo", None, &input());

        match decision {
            ClangdFileDecision::Create { path, contents } => {
                assert_eq!(path, "C:/repo/.clangd");
                assert!(contents.contains("Compiler: clang-cl"));
            }
            ClangdFileDecision::PreserveExisting { .. } => panic!("expected create decision"),
        }
    }

    #[test]
    fn preserves_existing_config() {
        let decision =
            decide_clangd_file(r"C:\repo", Some("CompileFlags: {}".to_string()), &input());

        assert_eq!(
            decision,
            ClangdFileDecision::PreserveExisting {
                path: "C:/repo/.clangd".to_string()
            }
        );
    }
}
