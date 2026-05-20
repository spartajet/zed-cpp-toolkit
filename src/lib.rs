use zed_extension_api as zed;

mod cmake;
mod debug;
mod environment;
mod error;
mod lsp;
mod paths;

#[derive(Default)]
struct MsvcToolkitExtension;

impl zed::Extension for MsvcToolkitExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        lsp::server::validate_language_server_id(language_server_id.as_ref())
            .map_err(|error| error.user_message())?;

        lsp::server::prepare_workspace_config_from_worktree(worktree)
            .map_err(|error| error.user_message())?;

        lsp::server::command_from_worktree(worktree).map_err(|error| error.user_message())
    }
}

zed::register_extension!(MsvcToolkitExtension);
