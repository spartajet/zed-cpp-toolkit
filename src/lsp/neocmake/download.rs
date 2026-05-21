//! neocmakelsp 二进制查找和下载。
//!
//! 使用 Zed API 从 GitHub Releases 下载 neocmakelsp。

use crate::debug::log_message;
use crate::error::{ToolkitError, ToolkitResult};
use zed_extension_api as zed;

const GITHUB_REPO: &str = "neocmakelsp/neocmakelsp";
const BINARY_NAME: &str = "neocmakelsp";

/// 获取平台特定的资源名称。
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
            "不支持的平台或架构: {:?} {:?}",
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

/// 从 GitHub Releases 下载 neocmakelsp 二进制。
fn download_binary(language_server_id: &zed::LanguageServerId) -> ToolkitResult<String> {
    log_message("从 GitHub releases 下载 neocmakelsp");

    let release = zed::latest_github_release(
        GITHUB_REPO,
        zed::GithubReleaseOptions {
            require_assets: true,
            pre_release: false,
        },
    )
    .map_err(|e| {
        log_message(&format!("获取 GitHub release 失败: {e}"));
        ToolkitError::NeocmakeDownloadFailed(format!("获取 GitHub release: {e}"))
    })?;

    let asset_name = get_asset_name()?;
    log_message(&format!("查找资源: {asset_name}"));

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            ToolkitError::NeocmakeDownloadFailed(format!(
                "未找到匹配的资源: {}。可用资源: {:?}",
                asset_name,
                release.assets.iter().map(|a| &a.name).collect::<Vec<_>>()
            ))
        })?;

    let install_dir = install_dir(&release.version);
    let binary_path = binary_path(&release.version);

    if std::fs::metadata(&binary_path).is_ok() {
        log_message(&format!("已存在 neocmakelsp 下载版本: {binary_path}"));
        return Ok(binary_path);
    }

    log_message(&format!("下载 URL: {}", asset.download_url));
    log_message(&format!("目标目录: {install_dir}"));

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
        log_message(&format!("下载文件失败: {e}"));
        ToolkitError::NeocmakeDownloadFailed(format!("下载文件: {e}"))
    })?;

    #[cfg(not(target_os = "windows"))]
    zed::make_file_executable(&binary_path)
        .map_err(|e| ToolkitError::NeocmakeDownloadFailed(format!("设置可执行权限: {e}")))?;

    log_message(&format!("neocmakelsp 已下载到: {binary_path}"));
    Ok(binary_path)
}

/// 清理旧版本的 LSP 二进制。
fn cleanup_old_binaries(current_version: &str) {
    log_message(&format!(
        "清理旧版本的 neocmakelsp (保留 {current_version})"
    ));

    let entries = match std::fs::read_dir(".") {
        Ok(entries) => entries,
        Err(e) => {
            log_message(&format!("无法列出目录: {e}"));
            return;
        }
    };

    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // 删除旧版本
        if name_str.starts_with("neocmakelsp-") && name_str != current_version {
            log_message(&format!("删除旧版本: {name_str}"));
            let _ = std::fs::remove_file(entry.path());
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// 在 PATH 中查找或下载 neocmakelsp。
pub fn get_or_download_binary(
    worktree: &zed::Worktree,
    language_server_id: &zed::LanguageServerId,
) -> ToolkitResult<String> {
    // 首先尝试 PATH
    if let Some(path) = worktree.which(BINARY_NAME) {
        log_message(&format!("在 PATH 中找到 neocmakelsp: {path}"));
        return Ok(path);
    }

    log_message("PATH 中未找到 neocmakelsp，尝试下载");
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
