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

        // 根据 ID 路由到对应的 LSP
        let result = match language_server_id {
            "msvc-cpp-clangd" => {
                if let Err(error_msg) = validate_and_prepare_clangd(worktree, language_server_id_value) {
                    return Err(error_msg);
                }
                lsp::server::command_from_worktree(worktree)
                    .map_err(|e| e.user_message())
            }
            "msvc-cmake-neocmake" => {
                if let Err(error_msg) = validate_and_prepare_neocmake(worktree, language_server_id_value) {
                    return Err(error_msg);
                }
                lsp::neocmake::server::command_from_worktree(worktree)
                    .map_err(|e| e.user_message())
            }
            _ => {
                let error = format!("不支持的 language server: {language_server_id}");
                debug::log_message(&error);
                set_lsp_status(
                    language_server_id_value,
                    zed::LanguageServerInstallationStatus::Failed(error.clone()),
                );
                return Err(error);
            }
        };

        match result {
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
                debug::log_message(&format!("language server command creation failed: {error}"));
                set_lsp_status(
                    language_server_id_value,
                    zed::LanguageServerInstallationStatus::Failed(error.clone()),
                );
                Err(error)
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

fn validate_and_prepare_clangd(
    worktree: &zed::Worktree,
    language_server_id: &zed::LanguageServerId,
) -> Result<(), String> {
    if let Err(error) = lsp::server::validate_language_server_id("msvc-cpp-clangd") {
        debug::log_error("language server id validation failed", &error);
        set_lsp_status(
            language_server_id,
            zed::LanguageServerInstallationStatus::Failed(error.user_message()),
        );
        return Err(error.user_message());
    }
    debug::log_message("language server id validation succeeded");

    set_lsp_status(
        language_server_id,
        zed::LanguageServerInstallationStatus::Downloading,
    );
    if let Err(error) = lsp::server::prepare_workspace_config_from_worktree(worktree) {
        debug::log_error("workspace config preparation failed", &error);
        set_lsp_status(
            language_server_id,
            zed::LanguageServerInstallationStatus::Failed(error.user_message()),
        );
        return Err(error.user_message());
    }
    debug::log_message("workspace config preparation succeeded");

    set_lsp_status(
        language_server_id,
        zed::LanguageServerInstallationStatus::CheckingForUpdate,
    );
    Ok(())
}

fn validate_and_prepare_neocmake(
    _worktree: &zed::Worktree,
    language_server_id: &zed::LanguageServerId,
) -> Result<(), String> {
    if let Err(error) = lsp::neocmake::server::validate_language_server_id("msvc-cmake-neocmake") {
        debug::log_error("neocmake language server id validation failed", &error);
        set_lsp_status(
            language_server_id,
            zed::LanguageServerInstallationStatus::Failed(error.user_message()),
        );
        return Err(error.user_message());
    }
    debug::log_message("neocmake language server id validation succeeded");

    set_lsp_status(
        language_server_id,
        zed::LanguageServerInstallationStatus::CheckingForUpdate,
    );
    Ok(())
}

zed::register_extension!(MsvcToolkitExtension);
