pub const VSWHERE_PATH: &str =
    r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe";

pub fn parse_installation_path(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_first_non_empty_installation_path() {
        let parsed = parse_installation_path(
            "\r\nC:\\Program Files\\Microsoft Visual Studio\\2022\\Community\r\n",
        );

        assert_eq!(
            parsed,
            Some("C:\\Program Files\\Microsoft Visual Studio\\2022\\Community".to_string())
        );
    }
}
