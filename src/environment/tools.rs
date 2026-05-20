use crate::error::{ToolkitError, ToolkitResult};

pub fn require_clangd(clangd_path: Option<String>) -> ToolkitResult<String> {
    clangd_path.ok_or(ToolkitError::MissingClangd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_existing_clangd_path() {
        let path = require_clangd(Some(r"C:\LLVM\bin\clangd.exe".to_string()));

        assert_eq!(path, Ok(r"C:\LLVM\bin\clangd.exe".to_string()));
    }

    #[test]
    fn reports_missing_clangd() {
        let error = require_clangd(None).unwrap_err();

        assert_eq!(error, ToolkitError::MissingClangd);
    }
}
