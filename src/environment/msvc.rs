use crate::paths::highest_version_dir;

pub fn select_msvc_include<'a>(
    versions: impl IntoIterator<Item = &'a str>,
    vs_root: &str,
) -> Option<String> {
    highest_version_dir(versions).map(|version| {
        format!(
            r"{vs_root}\VC\Tools\MSVC\{version}\include",
            vs_root = vs_root.trim_end_matches('\\'),
            version = version
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_highest_msvc_include_path() {
        let include = select_msvc_include(
            ["14.38.33130", "14.40.33807", "14.9.99999"],
            r"C:\Program Files\Microsoft Visual Studio\2022\Community",
        );

        assert_eq!(
            include,
            Some(
                r"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.40.33807\include"
                    .to_string()
            )
        );
    }
}
