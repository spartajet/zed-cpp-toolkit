#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Powershell,
    Sh,
}

pub fn default_shell_for_current_platform() -> ShellKind {
    if cfg!(target_os = "windows") {
        ShellKind::Powershell
    } else {
        ShellKind::Sh
    }
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
        ShellKind::Sh => (
            "sh".to_string(),
            vec!["-lc".to_string(), command.to_string()],
        ),
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
        assert_eq!(command, "sh");
        assert_eq!(args, vec!["-lc", "cmake --build build"]);
    }
}
