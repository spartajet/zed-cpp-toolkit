use zed_extension_api as zed;

mod build;
// CMake integration module (reserved feature, currently only used by clangd LSP)
#[allow(unused_imports)]
mod cmake;
mod config;
mod debug;
mod environment;
mod error;
mod lsp;
mod paths;
mod toolchain;

#[derive(Default)]
struct CppToolkitExtension;

impl zed::Extension for CppToolkitExtension {
    fn new() -> Self {
        debug::log_message("extension instance created");

        // Check if Git is available
        match std::process::Command::new("git").arg("--version").output() {
            Ok(output) => {
                let version = String::from_utf8_lossy(&output.stdout);
                debug::log_message(&format!("Git is available: {version}"));
            }
            Err(e) => {
                debug::log_message(&format!(
                    "Git is NOT available: {e}. Grammar download may fail."
                ));
            }
        }

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

        // Route to the appropriate LSP based on ID
        let result = match language_server_id {
            "cpp-toolkit-clangd" => {
                validate_and_prepare_clangd(worktree, language_server_id_value)?;
                lsp::server::command_from_worktree(worktree).map_err(|e| e.user_message())
            }
            "cpp-toolkit-neocmake" => {
                validate_and_prepare_neocmake(worktree, language_server_id_value)?;
                lsp::neocmake::server::command_from_worktree(worktree, language_server_id_value)
                    .map_err(|e| e.user_message())
            }
            _ => {
                let error = format!("Unsupported language server: {language_server_id}");
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

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<Option<serde_json::Value>> {
        match language_server_id.as_ref() {
            "cpp-toolkit-neocmake" => {
                let config = lsp::neocmake::config::load_config(worktree);
                let options = lsp::neocmake::init_options::build_init_options(&config);
                debug::log_message(&format!("neocmakelsp init options: {options}"));
                Ok(Some(options))
            }
            _ => Ok(None),
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
    if let Err(error) = lsp::server::validate_language_server_id("cpp-toolkit-clangd") {
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
    if let Err(error) = lsp::neocmake::server::validate_language_server_id("cpp-toolkit-neocmake") {
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

zed::register_extension!(CppToolkitExtension);

#[cfg(test)]
mod tests {
    #[test]
    fn extension_manifest_is_valid_toml() {
        let manifest = include_str!("../extension.toml");
        let _: toml::Value = toml::from_str(manifest).unwrap();
    }
}
