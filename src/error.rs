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
    MissingTool(String),
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
                format!("不支持的 language server: {id}")
            }
            Self::MissingVswhere => {
                "找不到 vswhere.exe。请确认已安装 Visual Studio Installer 和 Visual Studio 2022+。"
                    .to_string()
            }
            Self::MissingVisualStudio => {
                "找不到 Visual Studio 2022+。请安装 Visual Studio 2022 或更新版本，并包含“使用 C++ 的桌面开发”工作负载。"
                    .to_string()
            }
            Self::MissingMsvcToolset => {
                "找不到 MSVC v143+ toolset。请在 Visual Studio Installer 中安装 MSVC C++ build tools。"
                    .to_string()
            }
            Self::MissingClangd => {
                "找不到 clangd。请安装 LLVM，或将 clangd.exe 加入 PATH。".to_string()
            }
            Self::MissingCmake => {
                "找不到 cmake。请安装 CMake 并将其加入 PATH。".to_string()
            }
            Self::MissingTool(tool) => {
                format!("找不到工具：{tool}。请确认已安装并加入 PATH。")
            }
            Self::MissingWorkspaceConfig(contents) => format!(
                "当前 Zed extension API 不支持从扩展直接写入工作区 .clangd。请在工作区根目录手动创建 .clangd，内容如下：\n\n{contents}"
            ),
            Self::ProcessFailed {
                command,
                status,
                stderr,
            } => {
                format!("执行外部命令失败：{command}，退出码：{status:?}，错误输出：{stderr}")
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
