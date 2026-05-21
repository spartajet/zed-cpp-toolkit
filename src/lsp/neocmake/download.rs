//! neocmakelsp binary discovery and download.
//!
//! Uses Zed API to download neocmakelsp from GitHub Releases.

use crate::debug::log_message;
use crate::error::{ToolkitError, ToolkitResult};
use zed_extension_api as zed;

const GITHUB_REPO: &str = "neocmakelsp/neocmakelsp";
const BINARY_NAME: &str = "neocmakelsp";

/// Gets platform-specific asset name.
fn get_asset_name() -> ToolkitResult<String> {
    let (platform, arch) = zed::current_platform();
    asset_name_for_platform(platform, arch)
}

fn asset_name_for_platform(platform: zed::Os, arch: zed::Architecture) -> ToolkitResult<String> {
    match (platform, arch) {
        (zed::Os::Windows, zed::Architecture::X8664) => {
            Ok("neocmakelsp-x86_64-pc-windows-msvc.zip".to_string())
        }
        (zed::Os::Linux, zed::Architecture::X8664) => {
            Ok("neocmakelsp-x86_64-unknown-linux-gnu.tar.gz".to_string())
        }
        (zed::Os::Mac, _) => Ok("neocmakelsp-universal-apple-darwin.tar.gz".to_string()),
        _ => Err(ToolkitError::NeocmakeDownloadFailed(format!(
            "Unsupported platform or architecture: {:?} {:?}",
            platform, arch
        ))),
    }
}

fn downloaded_file_type(asset_name: &str) -> zed::DownloadedFileType {
    if asset_name.ends_with(".zip") {
        zed::DownloadedFileType::Zip
    } else if asset_name.ends_with(".tar.gz") {
        zed::DownloadedFileType::GzipTar
    } else {
        zed::DownloadedFileType::Uncompressed
    }
}

fn binary_filename() -> &'static str {
    if cfg!(target_os = "windows") {
        "neocmakelsp.exe"
    } else {
        BINARY_NAME
    }
}

fn install_dir(version: &str) -> String {
    format!("{BINARY_NAME}-{version}")
}

fn binary_path(version: &str) -> String {
    format!("{}/{}", install_dir(version), binary_filename())
}

/// Downloads neocmakelsp binary from GitHub Releases.
fn download_binary(language_server_id: &zed::LanguageServerId) -> ToolkitResult<String> {
    log_message("downloading neocmakelsp from GitHub releases");

    let release = zed::latest_github_release(
        GITHUB_REPO,
        zed::GithubReleaseOptions {
            require_assets: true,
            pre_release: false,
        },
    )
    .map_err(|e| {
        log_message(&format!("failed to get GitHub release: {e}"));
        ToolkitError::NeocmakeDownloadFailed(format!("get GitHub release: {e}"))
    })?;

    let asset_name = get_asset_name()?;
    log_message(&format!("looking for asset: {asset_name}"));

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            ToolkitError::NeocmakeDownloadFailed(format!(
                "no matching asset found: {}. Available assets: {:?}",
                asset_name,
                release.assets.iter().map(|a| &a.name).collect::<Vec<_>>()
            ))
        })?;

    let install_dir = install_dir(&release.version);
    let binary_path = binary_path(&release.version);

    if std::fs::metadata(&binary_path).is_ok() {
        log_message(&format!("neocmakelsp download already exists: {binary_path}"));
        return Ok(binary_path);
    }

    log_message(&format!("download URL: {}", asset.download_url));
    log_message(&format!("target directory: {install_dir}"));

    zed::set_language_server_installation_status(
        language_server_id,
        &zed::LanguageServerInstallationStatus::Downloading,
    );

    zed::download_file(
        &asset.download_url,
        &install_dir,
        downloaded_file_type(&asset.name),
    )
    .map_err(|e| {
        log_message(&format!("failed to download file: {e}"));
        ToolkitError::NeocmakeDownloadFailed(format!("download file: {e}"))
    })?;

    #[cfg(not(target_os = "windows"))]
    zed::make_file_executable(&binary_path)
        .map_err(|e| ToolkitError::NeocmakeDownloadFailed(format!("set executable permission: {e}")))?;

    log_message(&format!("neocmakelsp downloaded to: {binary_path}"));
    Ok(binary_path)
}

/// Cleans up old versions of LSP binaries.
fn cleanup_old_binaries(current_version: &str) {
    log_message(&format!(
        "cleaning up old versions of neocmakelsp (keeping {current_version})"
    ));

    let entries = match std::fs::read_dir(".") {
        Ok(entries) => entries,
        Err(e) => {
            log_message(&format!("failed to list directory: {e}"));
            return;
        }
    };

    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Delete old versions
        if name_str.starts_with("neocmakelsp-") && name_str != current_version {
            log_message(&format!("deleting old version: {name_str}"));
            let _ = std::fs::remove_file(entry.path());
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// Finds neocmakelsp in PATH or downloads it.
pub fn get_or_download_binary(
    worktree: &zed::Worktree,
    language_server_id: &zed::LanguageServerId,
) -> ToolkitResult<String> {
    // First try PATH
    if let Some(path) = worktree.which(BINARY_NAME) {
        log_message(&format!("found neocmakelsp in PATH: {path}"));
        return Ok(path);
    }

    log_message("neocmakelsp not found in PATH, attempting download");
    let binary_path = download_binary(language_server_id)?;

    if let Some(current_dir) = binary_path.split('/').next() {
        cleanup_old_binaries(current_dir);
    }
    Ok(binary_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_windows_release_asset() {
        let asset = asset_name_for_platform(zed::Os::Windows, zed::Architecture::X8664).unwrap();

        assert_eq!(asset, "neocmakelsp-x86_64-pc-windows-msvc.zip");
    }

    #[test]
    fn selects_linux_release_asset() {
        let asset = asset_name_for_platform(zed::Os::Linux, zed::Architecture::X8664).unwrap();

        assert_eq!(asset, "neocmakelsp-x86_64-unknown-linux-gnu.tar.gz");
    }

    #[test]
    fn selects_universal_macos_release_asset() {
        let asset = asset_name_for_platform(zed::Os::Mac, zed::Architecture::Aarch64).unwrap();

        assert_eq!(asset, "neocmakelsp-universal-apple-darwin.tar.gz");
    }

    #[test]
    fn builds_versioned_install_paths() {
        assert_eq!(install_dir("v0.10.2"), "neocmakelsp-v0.10.2");
        assert!(binary_path("v0.10.2").starts_with("neocmakelsp-v0.10.2/"));
    }
}
