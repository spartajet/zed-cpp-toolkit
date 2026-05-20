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
        debug::log_message("extension instance created");
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        let language_server_id_value = language_server_id;
        let language_server_id = language_server_id.as_ref();
        let root_path = worktree.root_path();
        debug::log_message(&format!(
            "language_server_command called: id={language_server_id}, root={root_path}"
        ));

        set_lsp_status(
            language_server_id_value,
            zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        if let Err(error) = lsp::server::validate_language_server_id(language_server_id) {
            debug::log_error("language server id validation failed", &error);
            set_lsp_status(
                language_server_id_value,
                zed::LanguageServerInstallationStatus::Failed(error.user_message()),
            );
            return Err(error.user_message());
        }
        debug::log_message("language server id validation succeeded");

        set_lsp_status(
            language_server_id_value,
            zed::LanguageServerInstallationStatus::Downloading,
        );
        if let Err(error) = lsp::server::prepare_workspace_config_from_worktree(worktree) {
            debug::log_error("workspace config preparation failed", &error);
            set_lsp_status(
                language_server_id_value,
                zed::LanguageServerInstallationStatus::Failed(error.user_message()),
            );
            return Err(error.user_message());
        }
        debug::log_message("workspace config preparation succeeded");

        set_lsp_status(
            language_server_id_value,
            zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        match lsp::server::command_from_worktree(worktree) {
            Ok(command) => {
                debug::log_message(&format!(
                    "language server command ready: command={}, args={:?}, env_count={}",
                    command.command,
                    command.args,
                    command.env.len()
                ));
                set_lsp_status(
                    language_server_id_value,
                    zed::LanguageServerInstallationStatus::None,
                );
                Ok(command)
            }
            Err(error) => {
                debug::log_error("language server command creation failed", &error);
                set_lsp_status(
                    language_server_id_value,
                    zed::LanguageServerInstallationStatus::Failed(error.user_message()),
                );
                Err(error.user_message())
            }
        }
    }
}

fn set_lsp_status(
    language_server_id: &zed::LanguageServerId,
    status: zed::LanguageServerInstallationStatus,
) {
    zed::set_language_server_installation_status(language_server_id, &status);
}

zed::register_extension!(MsvcToolkitExtension);
