//! neocmakelsp 二进制查找和下载。
//!
//! 使用 Zed API 从 GitHub Releases 下载 neocmakelsp。

use crate::debug::log_message;
use crate::error::{ToolkitError, ToolkitResult};
use zed_extension_api as zed;

const GITHUB_REPO: &str = "Decodetalkers/neocmakelsp";
const BINARY_NAME: &str = "neocmakelsp";

/// 获取平台特定的资源名称。
fn get_asset_name() -> ToolkitResult<String> {
    let (platform, arch) = zed::current_platform();

    let arch_str = match arch {
        zed::Architecture::X8664 => "x86_64",
        zed::Architecture::Aarch64 => "aarch64",
        _ => {
            return Err(ToolkitError::NeocmakeDownloadFailed(format!(
                "不支持的架构: {:?}",
                arch
            )));
        }
    };

    let os_str = match platform {
        zed::Os::Windows => "pc-windows-msvc.exe",
        zed::Os::Linux => "unknown-linux-gnu",
        zed::Os::Mac => "apple-darwin",
    };

    Ok(format!("{}-{}-{}", BINARY_NAME, arch_str, os_str))
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

    let binary_path = format!("{}-{}", BINARY_NAME, release.version);

    log_message(&format!("下载 URL: {}", asset.download_url));
    log_message(&format!("目标路径: {binary_path}"));

    zed::set_language_server_installation_status(
        language_server_id,
        &zed::LanguageServerInstallationStatus::Downloading,
    );

    zed::download_file(
        &asset.download_url,
        &binary_path,
        zed::DownloadedFileType::Uncompressed,
    )
    .map_err(|e| {
        log_message(&format!("下载文件失败: {e}"));
        ToolkitError::NeocmakeDownloadFailed(format!("下载文件: {e}"))
    })?;

    #[cfg(unix)]
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

    cleanup_old_binaries(&binary_path);
    Ok(binary_path)
}
