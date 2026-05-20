use zed_extension_api as zed;

pub fn clangd_args() -> Vec<String> {
    vec!["--header-insertion=never".to_string()]
}

pub fn build_clangd_command(command: String, env: Vec<(String, String)>) -> zed::Command {
    zed::Command {
        command,
        args: clangd_args(),
        env,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clangd_args_disable_header_insertion() {
        assert_eq!(clangd_args(), vec!["--header-insertion=never"]);
    }
}
