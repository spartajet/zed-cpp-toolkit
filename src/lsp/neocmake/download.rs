//! 从 GitHub Releases 下载 neocmakelsp 二进制。

use crate::debug::log_message;
use crate::error::{ToolkitError, ToolkitResult};
use zed_extension_api as zed;

const GITHUB_REPO: &str = "neocmakelsp/neocmakelsp";
const BINARY_NAME: &str = "neocmakelsp";

/// neocmakelsp releases 的平台特定资源名称模式。
fn platform_asset_name() -> Option<&'static str> {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Some("neocmakelsp-x86_64-pc-windows-msvc.zip");

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Some("neocmakelsp-x86_64-unknown-linux-gnu");

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Some("neocmakelsp-x86_64-apple-darwin");

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Some("neocmakelsp-aarch64-apple-darwin");

    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
    )))]
    {
        log_message("neocmakelsp: 不支持的平台无法下载");
        None
    }
}

/// 从 GitHub Releases 下载 neocmakelsp 二进制。
pub fn download_binary() -> ToolkitResult<String> {
    let asset_name = platform_asset_name()
        .ok_or_else(|| ToolkitError::NeocmakeDownloadFailed("不支持的平台".to_string()))?;

    log_message(&format!("从 GitHub releases 下载 neocmakelsp，资源: {asset_name}"));

    let release = zed::latest_github_release(GITHUB_REPO)
        .map_err(|e| ToolkitError::NeocmakeDownloadFailed(format!("获取 release: {e}")))?;

    log_message(&format!("最新版本: {}", release.version));

    let asset = release.assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            ToolkitError::NeocmakeDownloadFailed(format!("release 中未找到资源 {asset_name}"))
        })?;

    let extension_dir = zed::extensions_dir();
    let target_dir = format!("{extension_dir}/neocmakelsp");
    let _ = zed::make_dir(&target_dir);

    let binary_path = format!("{target_dir}/{BINARY_NAME}");

    // 检查是否已存在
    if let Ok(true) = zed::file_exists(&binary_path) {
        log_message(&format!("neocmakelsp 已存在于: {binary_path}"));
        return Ok(binary_path);
    }

    log_message(&format!("下载资源到: {binary_path}"));
    let downloaded_path = zed::download_file(
        &asset.download_url,
        &target_dir,
        Some(BINARY_NAME),
    )
        .map_err(|e| ToolkitError::NeocmakeDownloadFailed(format!("下载: {e}")))?;

    log_message(&format!("已下载到: {downloaded_path}"));

    // 在 Windows 上处理 .zip 解压
    if asset_name.ends_with(".zip") {
        log_message("从 zip 压缩包中提取 neocmakelsp");
        // Zed 的 download_file 会自动解压 zip，二进制应该在 binary_path
    }

    zed::make_file_executable(&downloaded_path)
        .map_err(|e| ToolkitError::NeocmakeDownloadFailed(format!("设置可执行: {e}")))?;

    log_message(&format!("neocmakelsp 就绪于: {downloaded_path}"));
    Ok(downloaded_path)
}

/// 在 PATH 中查找 neocmakelsp 或下载它。
pub fn get_or_download_binary(worktree: &zed::Worktree) -> ToolkitResult<String> {
    // 首先尝试 PATH
    if let Some(path) = worktree.which(BINARY_NAME) {
        log_message(&format!("在 PATH 中找到 neocmakelsp: {path}"));
        return Ok(path);
    }

    log_message("PATH 中未找到 neocmakelsp，尝试下载");
    download_binary()
}
