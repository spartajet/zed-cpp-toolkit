use std::fmt;

pub type ToolkitResult<T> = Result<T, ToolkitError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolkitError {
    UnsupportedLanguageServer(String),
    MissingVswhere,
    MissingVisualStudio,
    MissingMsvcToolset,
    MissingClangd,
    MissingCmake,
    MissingNeocmakelsp,
    #[allow(dead_code)]
    NeocmakeConfigParseError(String),
    // 预留错误：通用工具和配置
    #[allow(dead_code)]
    MissingTool(String),
    #[allow(dead_code)]
    MissingWorkspaceConfig(String),
    ProcessFailed {
        command: String,
        status: Option<i32>,
        stderr: String,
    },
    IoMessage(String),
}

impl ToolkitError {
    pub fn user_message(&self) -> String {
        match self {
            Self::UnsupportedLanguageServer(id) => {
                format!("Unsupported language server: {id}")
            }
            Self::MissingVswhere => {
                "vswhere.exe not found. Please ensure Visual Studio Installer and Visual Studio 2022+ are installed."
                    .to_string()
            }
            Self::MissingVisualStudio => {
                "Visual Studio 2022+ not found. Please install Visual Studio 2022 or later with the 'Desktop development with C++' workload."
                    .to_string()
            }
            Self::MissingMsvcToolset => {
                "MSVC v143+ toolset not found. Please install MSVC C++ build tools in Visual Studio Installer."
                    .to_string()
            }
            Self::MissingClangd => {
                "clangd not found. Please install LLVM or add clangd.exe to PATH.".to_string()
            }
            Self::MissingCmake => {
                "cmake not found. Please install CMake and add it to PATH.".to_string()
            }
            Self::MissingNeocmakelsp => {
                "neocmakelsp not found. Please install neocmakelsp and add it to PATH. See: https://github.com/Neuttions/neocmakelsp".to_string()
            }
            Self::NeocmakeConfigParseError(detail) => {
                format!("Failed to parse neocmake config: {detail}, using default configuration.")
            }
            Self::MissingTool(tool) => {
                format!("Tool not found: {tool}. Please ensure it is installed and in PATH.")
            }
            Self::MissingWorkspaceConfig(contents) => format!(
                "Current Zed extension API does not support writing workspace .clangd from extension. Please manually create .clangd in workspace root with:\n\n{contents}"
            ),
            Self::ProcessFailed {
                command,
                status,
                stderr,
            } => {
                format!("External command failed: {command}, exit code: {status:?}, stderr: {stderr}")
            }
            Self::IoMessage(message) => message.clone(),
        }
    }
}

impl fmt::Display for ToolkitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.user_message())
    }
}
