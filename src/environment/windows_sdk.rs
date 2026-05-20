use crate::paths::highest_version_dir;

const SDK_INCLUDE_KINDS: [&str; 3] = ["ucrt", "um", "shared"];

pub fn select_windows_sdk_includes<'a>(
    versions: impl IntoIterator<Item = &'a str>,
    kits_include_root: &str,
) -> Vec<String> {
    let Some(version) = highest_version_dir(versions) else {
        return Vec::new();
    };

    SDK_INCLUDE_KINDS
        .iter()
        .map(|kind| {
            format!(
                r"{root}\{version}\{kind}",
                root = kits_include_root.trim_end_matches('\\'),
                version = version,
                kind = kind
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_highest_sdk_include_group() {
        let includes = select_windows_sdk_includes(
            ["10.0.19041.0", "10.0.22621.0"],
            r"C:\Program Files (x86)\Windows Kits\10\Include",
        );

        assert_eq!(
            includes,
            vec![
                r"C:\Program Files (x86)\Windows Kits\10\Include\10.0.22621.0\ucrt",
                r"C:\Program Files (x86)\Windows Kits\10\Include\10.0.22621.0\um",
                r"C:\Program Files (x86)\Windows Kits\10\Include\10.0.22621.0\shared",
            ]
        );
    }

    #[test]
    fn returns_empty_includes_when_sdk_versions_are_missing() {
        let includes =
            select_windows_sdk_includes([], r"C:\Program Files (x86)\Windows Kits\10\Include");

        assert!(includes.is_empty());
    }
}
