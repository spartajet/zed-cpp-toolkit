#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Powershell,
    Sh,
}

#[cfg(test)]
pub fn default_shell_for_current_platform() -> ShellKind {
    if cfg!(target_os = "windows") {
        ShellKind::Powershell
    } else {
        ShellKind::Sh
    }
}

pub fn shell_for_root_path(root_path: &str) -> ShellKind {
    if is_windows_path(root_path) {
        ShellKind::Powershell
    } else {
        ShellKind::Sh
    }
}

fn is_windows_path(path: &str) -> bool {
    path.contains('\\') || path.as_bytes().get(1).is_some_and(|byte| *byte == b':')
}

pub fn wrap_command(shell: ShellKind, command: &str) -> (String, Vec<String>) {
    match shell {
        ShellKind::Powershell => (
            "powershell".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                command.to_string(),
            ],
        ),
        ShellKind::Sh => (command.to_string(), Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_powershell_command() {
        let (command, args) = wrap_command(ShellKind::Powershell, "cmake --build build");
        assert_eq!(command, "powershell");
        assert_eq!(args, vec!["-NoProfile", "-Command", "cmake --build build"]);
    }

    #[test]
    fn wraps_sh_command() {
        let (command, args) = wrap_command(ShellKind::Sh, "cmake --build build");
        assert_eq!(command, "cmake --build build");
        assert!(args.is_empty());
    }

    #[test]
    fn default_shell_matches_current_platform() {
        let expected = if cfg!(target_os = "windows") {
            ShellKind::Powershell
        } else {
            ShellKind::Sh
        };

        assert_eq!(default_shell_for_current_platform(), expected);
    }

    #[test]
    fn infers_shell_from_root_path() {
        assert_eq!(shell_for_root_path(r"C:\repo"), ShellKind::Powershell);
        assert_eq!(shell_for_root_path("C:/repo"), ShellKind::Powershell);
        assert_eq!(shell_for_root_path("/home/me/repo"), ShellKind::Sh);
    }
}
