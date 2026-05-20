//! neocmakelsp 二进制查找和下载。
//!
//! 使用 PowerShell 命令从 GitHub Releases 下载 neocmakelsp。

use crate::debug::log_message;
use crate::error::{ToolkitError, ToolkitResult};
use crate::environment::tools::{CommandRunner, ZedCommandRunner};
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
        None
    }
}

/// 获取用户本地程序目录（用于存放下载的二进制）。
fn get_local_binary_dir() -> String {
    #[cfg(target_os = "windows")]
    {
        // 使用 %LOCALAPPDATA%\zed-msvc-toolkit
        std::env::var("LOCALAPPDATA")
            .map(|p| format!("{p}\\zed-msvc-toolkit\\neocmakelsp"))
            .unwrap_or_else(|_| ".\\neocmakelsp".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .map(|p| format!("{p}/.local/share/zed-msvc-toolkit/neocmakelsp"))
            .unwrap_or_else(|_| "./neocmakelsp".to_string())
    }
}

/// 从 GitHub Releases 下载 neocmakelsp 二进制。
fn download_binary() -> ToolkitResult<String> {
    let asset_name = platform_asset_name()
        .ok_or_else(|| ToolkitError::NeocmakeDownloadFailed("不支持的平台".to_string()))?;

    log_message(&format!("从 GitHub releases 下载 neocmakelsp，资源: {asset_name}"));

    let target_dir = get_local_binary_dir();
    let binary_path = format!("{target_dir}\\{BINARY_NAME}");

    // 检查是否已存在
    if std::path::Path::new(&binary_path).exists() {
        log_message(&format!("neocmakelsp 已存在于: {binary_path}"));
        return Ok(binary_path);
    }

    // 构建下载 URL
    let download_url = format!(
        "https://github.com/{}/releases/latest/download/{}",
        GITHUB_REPO, asset_name
    );

    log_message(&format!("下载 URL: {download_url}"));
    log_message(&format!("目标目录: {target_dir}"));

    // 使用 PowerShell 下载
    let runner = ZedCommandRunner;

    // 创建目标目录
    let mkdir_script = format!(
        "$ErrorActionPreference='Stop'; New-Item -ItemType Directory -Force -Path '{}' | Out-Null; 'created'",
        target_dir.replace('\\', "\\\\").replace('\'', "''")
    );
    let mkdir_args = vec![
        "-NoProfile".to_string(),
        "-Command".to_string(),
        mkdir_script,
    ];

    if let Err(e) = runner.run_command("powershell", &mkdir_args) {
        log_message(&format!("创建目录失败（可能已存在）: {e}"));
    }

    // 下载文件
    let download_script = format!(
        "$ErrorActionPreference='Stop'; \
         $ProgressPreference = 'SilentlyContinue'; \
         $url = '{}'; \
         $out = '{}'; \
         $dir = Split-Path -Parent $out; \
         if ($dir) {{ New-Item -ItemType Directory -Force -Path $dir | Out-Null }}; \
         Invoke-WebRequest -Uri $url -OutFile $out -UseBasicParsing; \
         'downloaded'",
        download_url.replace('\'', "''"),
        binary_path.replace('\\', "\\\\").replace('\'', "''")
    );

    let download_args = vec![
        "-NoProfile".to_string(),
        "-Command".to_string(),
        download_script,
    ];

    let output = runner.run_command("powershell", &download_args)
        .map_err(|e| ToolkitError::NeocmakeDownloadFailed(format!("执行下载命令: {e}")))?;

    if output.status != Some(0) {
        return Err(ToolkitError::NeocmakeDownloadFailed(format!(
            "下载失败: {}", output.stderr
        )));
    }

    log_message(&format!("已下载到: {binary_path}"));

    // 如果是 .zip 文件，需要解压
    if asset_name.ends_with(".zip") {
        log_message("解压 zip 文件");

        let unzip_script = format!(
            "$ErrorActionPreference='Stop'; \
             $zip = '{}'; \
             $dest = '{}'; \
             Expand-Archive -LiteralPath $zip -DestinationPath $dest -Force; \
             'unzipped'",
            binary_path.replace('\\', "\\\\").replace('\'', "''"),
            target_dir.replace('\\', "\\\\").replace('\'', "''")
        );

        let unzip_args = vec![
            "-NoProfile".to_string(),
            "-Command".to_string(),
            unzip_script,
        ];

        let output = runner.run_command("powershell", &unzip_args)
            .map_err(|e| ToolkitError::NeocmakeDownloadFailed(format!("解压: {e}")))?;

        if output.status != Some(0) {
            return Err(ToolkitError::NeocmakeDownloadFailed(format!(
                "解压失败: {}", output.stderr
            )));
        }

        log_message("解压完成");
    }

    log_message(&format!("neocmakelsp 就绪于: {binary_path}"));
    Ok(binary_path)
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
