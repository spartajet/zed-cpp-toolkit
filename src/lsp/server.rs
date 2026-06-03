use crate::build::shell::shell_for_root_path;
use crate::build::tasks::generate_cpp_tasks_json;
use crate::cmake::{CmakeTarget, TaskOptions, discover_compile_database, generate_tasks_json};
use crate::config::loader::load_effective_config;
#[cfg(test)]
use crate::config::merge::resolve_config;
use crate::config::schema::EffectiveConfig;
use crate::debug::log_message;
use crate::environment::tools::{ZedCommandRunner, require_clangd};
use crate::environment::vswhere::discover_visual_studio;
use crate::environment::{MsvcEnvironment, discover_msvc_environment};
use crate::error::{ToolkitError, ToolkitResult};
use crate::lsp::clangd_config::ClangdConfigInput;
use crate::lsp::workspace_config::{ClangdFileDecision, decide_clangd_file};
use std::collections::HashMap;
use zed_extension_api as zed;

pub const LANGUAGE_SERVER_ID: &str = "cpp-toolkit-clangd";

pub fn clangd_args() -> Vec<String> {
    vec![
        "--header-insertion=never".to_string(),
        "--log=verbose".to_string(),
    ]
}

pub fn validate_language_server_id(id: &str) -> ToolkitResult<()> {
    if id == LANGUAGE_SERVER_ID {
        Ok(())
    } else {
        Err(ToolkitError::UnsupportedLanguageServer(id.to_string()))
    }
}

pub fn build_clangd_command(
    command: String,
    env: Vec<(String, String)>,
    query_driver: Vec<String>,
) -> zed::Command {
    let mut args = clangd_args();
    if !query_driver.is_empty() {
        args.push(format!("--query-driver={}", query_driver.join(",")));
    }

    zed::Command { command, args, env }
}

pub fn command_from_worktree(worktree: &zed::Worktree) -> ToolkitResult<zed::Command> {
    let config = load_effective_config(worktree)?;
    log_message(&format!(
        "looking up clangd via worktree.which({:?})",
        config.clangd.command
    ));
    let clangd = resolve_clangd_command(
        &config.clangd.command,
        worktree.which(&config.clangd.command),
    )?;
    log_message(&format!("clangd command resolved: {clangd}"));
    Ok(build_clangd_command(
        clangd,
        worktree.shell_env(),
        config.clangd.query_driver,
    ))
}

fn resolve_clangd_command(
    configured_command: &str,
    discovered: Option<String>,
) -> ToolkitResult<String> {
    if let Some(command) = discovered {
        return Ok(command);
    }

    if configured_command == "clangd" {
        return require_clangd(None);
    }

    if is_explicit_command_path(configured_command) {
        Ok(configured_command.to_string())
    } else {
        Err(ToolkitError::MissingTool(configured_command.to_string()))
    }
}

fn is_explicit_command_path(command: &str) -> bool {
    command.contains('/')
        || command.contains('\\')
        || command.as_bytes().get(1).is_some_and(|byte| *byte == b':')
}

#[cfg(test)]
pub fn prepare_workspace_config(
    root_path: &str,
    existing_clangd: Option<String>,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<()> {
    let config = resolve_config(None)?;
    prepare_workspace_config_with_config(root_path, existing_clangd, &config, runner)
}

fn prepare_workspace_config_with_config(
    root_path: &str,
    existing_clangd: Option<String>,
    config: &EffectiveConfig,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<()> {
    log_message(&format!(
        "prepare_workspace_config started: root={root_path}, preset={}",
        config.preset
    ));

    if let Some(contents) = &existing_clangd {
        log_message(&format!(
            "existing .clangd found; bytes={}, preview={}",
            contents.len(),
            one_line_preview(contents)
        ));
        if is_user_clangd_config(contents) {
            log_message("skipping generated .clangd because workspace already provides one");
            if let Err(error) = write_generated_tasks(root_path, config, runner) {
                crate::debug::log_error("failed to write generated Zed tasks", &error);
                log_message("continuing without blocking clangd startup");
            }
            return Ok(());
        }
        log_message("existing generated .clangd found; refreshing it from cpp-toolkit config");
    }

    if should_run_legacy_msvc_compile_database_fallback(config) {
        let compile_db_path = ensure_compile_database(
            root_path,
            &config.build.build_dir,
            &config.build.build_type,
            runner,
        );
        match &compile_db_path {
            Some(path) => log_message(&format!("compile_commands.json detected in: {path}")),
            None => log_message("compile_commands.json not detected in root or build/"),
        }
    }

    write_generated_clangd(root_path, config, runner)?;
    if let Err(error) = write_generated_tasks(root_path, config, runner) {
        crate::debug::log_error("failed to write generated Zed tasks", &error);
        log_message("continuing without blocking clangd startup");
    }
    Ok(())
}

fn is_user_clangd_config(contents: &str) -> bool {
    !contents.contains("# Auto-generated by Zed MSVC C++ Assistant.")
        && !contents.contains("# Auto-generated by Zed C++ Toolkit.")
}

fn should_run_legacy_msvc_compile_database_fallback(config: &EffectiveConfig) -> bool {
    config.toolchain.name == "msvc" && config.build.system == "cmake"
}

fn write_generated_clangd(
    root_path: &str,
    config: &EffectiveConfig,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<()> {
    let environment = discover_optional_msvc_environment(config, runner);
    let (msvc_include, sdk_includes) = environment
        .map(|environment| (Some(environment.msvc_include), environment.sdk_includes))
        .unwrap_or((None, Vec::new()));
    let input = workspace_clangd_input(config, msvc_include, sdk_includes);

    match decide_clangd_file(root_path, None, &input) {
        ClangdFileDecision::Create { contents, .. } => {
            log_message(&format!("generated .clangd content:\n{contents}"));
            match write_clangd_file(root_path, &contents, runner) {
                Ok(()) => log_message("wrote generated .clangd to workspace root"),
                Err(error) => {
                    crate::debug::log_error("failed to write generated .clangd", &error);
                    log_message("continuing without blocking clangd startup");
                }
            }
            Ok(())
        }
        ClangdFileDecision::PreserveExisting { .. } => {
            log_message("preserving existing .clangd");
            Ok(())
        }
    }
}

fn workspace_clangd_input(
    config: &EffectiveConfig,
    msvc_include: Option<String>,
    sdk_includes: Vec<String>,
) -> ClangdConfigInput {
    ClangdConfigInput {
        compiler: config.clangd.compiler.clone(),
        compile_commands_dir: config.clangd.compile_commands_dir.clone(),
        extra_flags: config.clangd.extra_flags.clone(),
        msvc_include,
        sdk_includes,
    }
}

fn discover_optional_msvc_environment(
    config: &EffectiveConfig,
    runner: &impl crate::environment::tools::CommandRunner,
) -> Option<MsvcEnvironment> {
    if config.toolchain.name != "msvc" {
        return None;
    }

    log_message("discovering MSVC environment for generated .clangd");
    match discover_msvc_environment(runner) {
        Ok(environment) => {
            log_message(&format!(
                "MSVC environment discovered: vs_root={}, msvc_include={}, sdk_include_count={}",
                environment.visual_studio_root,
                environment.msvc_include,
                environment.sdk_includes.len()
            ));
            Some(environment)
        }
        Err(error) => {
            crate::debug::log_error("MSVC environment discovery failed", &error);
            log_message("continuing with config-only clangd flags");
            None
        }
    }
}

fn ensure_compile_database(
    root_path: &str,
    build_dir: &str,
    build_type: &str,
    runner: &impl crate::environment::tools::CommandRunner,
) -> Option<String> {
    if let Some(path) = discover_compile_database_from_host(root_path, build_dir, runner) {
        return Some(path);
    }

    if !host_path_exists(runner, &join_windows_path(root_path, "CMakeLists.txt")).unwrap_or(false) {
        log_message("CMakeLists.txt not found; skipping CMake configure");
        return None;
    }

    log_message(&format!(
        "compile_commands.json missing; running CMake configure for {build_dir}/"
    ));
    if let Err(error) =
        run_cmake_configure_for_compile_database(root_path, build_dir, build_type, runner)
    {
        crate::debug::log_error("CMake configure failed", &error);
        return None;
    }
    if let Err(error) = run_cmake_autogen_targets(root_path, build_dir, runner) {
        crate::debug::log_error("CMake autogen target failed", &error);
    }

    let discovered = discover_compile_database_from_host(root_path, build_dir, runner);
    match &discovered {
        Some(path) => log_message(&format!(
            "compile_commands.json generated by CMake configure in: {path}"
        )),
        None => log_message(&format!(
            "CMake configure completed but {build_dir}/compile_commands.json is still missing"
        )),
    }
    discovered
}

fn discover_compile_database_from_host(
    root_path: &str,
    build_dir_name: &str,
    runner: &impl crate::environment::tools::CommandRunner,
) -> Option<String> {
    if let Some(path) = discover_compile_database(root_path) {
        return Some(path);
    }

    let root_db = join_windows_path(root_path, "compile_commands.json");
    if host_path_exists(runner, &root_db).unwrap_or(false) {
        return Some(root_path.to_string());
    }

    let build_dir = join_windows_path(root_path, build_dir_name);
    let build_db = join_windows_path(&build_dir, "compile_commands.json");
    if host_path_exists(runner, &build_db).unwrap_or(false) {
        return Some(build_dir);
    }

    None
}

fn host_path_exists(
    runner: &impl crate::environment::tools::CommandRunner,
    path: &str,
) -> ToolkitResult<bool> {
    let escaped_path = powershell_single_quote(path);
    let script = format!(
        "$ErrorActionPreference='Stop'; if (Test-Path -LiteralPath {escaped_path}) {{ 'true' }} else {{ 'false' }}"
    );
    let args = vec!["-NoProfile".to_string(), "-Command".to_string(), script];
    let output = runner.run_command("powershell", &args)?;
    let stdout = crate::environment::tools::ensure_success("powershell", output)?;
    Ok(stdout.trim().eq_ignore_ascii_case("true"))
}

fn join_windows_path(root_path: &str, child: &str) -> String {
    format!(
        "{}\\{}",
        root_path.trim_end_matches(['\\', '/'].as_slice()),
        child.trim_start_matches(['\\', '/'].as_slice())
    )
}

fn run_cmake_configure_for_compile_database(
    root_path: &str,
    build_dir_name: &str,
    build_type: &str,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<()> {
    let visual_studio_root = discover_visual_studio(runner)?;
    let vs_dev_cmd = join_windows_path(&visual_studio_root, r"Common7\Tools\VsDevCmd.bat");
    let build_dir = join_windows_path(root_path, build_dir_name);
    let cmake_args = vec![
        "-S".to_string(),
        root_path.to_string(),
        "-B".to_string(),
        build_dir.clone(),
        "-G".to_string(),
        "Ninja".to_string(),
        "-DCMAKE_C_COMPILER=cl".to_string(),
        "-DCMAKE_CXX_COMPILER=cl".to_string(),
        format!("-DCMAKE_BUILD_TYPE={build_type}"),
        "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON".to_string(),
    ];
    let cmake_command = format!(
        "cmake {}",
        cmake_args
            .iter()
            .map(|arg| cmd_quote(arg))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let cmd_line = format!(
        "call {} -arch=x64 -host_arch=x64 && {cmake_command}",
        cmd_quote(&vs_dev_cmd)
    );
    let script = format!(
        "$ErrorActionPreference='Stop'; & cmd.exe /S /C {}",
        powershell_single_quote(&cmd_line)
    );
    log_message(&format!(
        "running CMake configure through VS developer environment: {cmake_command}"
    ));
    let args = vec!["-NoProfile".to_string(), "-Command".to_string(), script];
    let output = runner.run_command("powershell", &args)?;
    crate::environment::tools::ensure_success("powershell", output).map(|_| ())
}

fn run_cmake_autogen_targets(
    root_path: &str,
    build_dir_name: &str,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<()> {
    let Some(target) = first_autogen_target(root_path, build_dir_name, runner)? else {
        log_message("no CMake *_autogen target found; skipping Qt autogen");
        return Ok(());
    };

    let build_dir = join_windows_path(root_path, build_dir_name);
    let args = vec![
        "--build".to_string(),
        build_dir,
        "--target".to_string(),
        target.clone(),
    ];
    log_message(&format!("running CMake autogen target: {target}"));
    let output = runner.run_command("cmake", &args)?;
    crate::environment::tools::ensure_success("cmake", output).map(|_| ())
}

fn write_generated_tasks(
    root_path: &str,
    config: &EffectiveConfig,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<()> {
    let task_config = crate::toolchain::prepare_task_config(config, runner)?;
    let contents = generate_cpp_tasks_json(&task_config, shell_for_root_path(root_path))?;
    log_message(&format!("generated .zed/tasks.json content:\n{contents}"));
    write_tasks_file(root_path, &contents, runner)?;
    log_message("wrote generated .zed/tasks.json to workspace");
    Ok(())
}

#[allow(dead_code)]
fn write_legacy_msvc_tasks(
    root_path: &str,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<()> {
    let visual_studio_root = discover_visual_studio(runner)?;
    let vs_dev_cmd = join_windows_path(&visual_studio_root, r"Common7\Tools\VsDevCmd.bat");
    let targets = discover_cmake_targets(root_path, runner)?;
    log_message(&format!(
        "discovered {} CMake target(s) for Zed tasks: {:?}",
        targets.len(),
        targets
            .iter()
            .map(|target| target.name.as_str())
            .collect::<Vec<_>>()
    ));

    let options = TaskOptions {
        build_dir: "build".to_string(),
        build_type: "Debug".to_string(),
        vs_dev_cmd: Some(vs_dev_cmd),
        targets,
    };
    let contents = generate_tasks_json(&options)?;
    log_message(&format!("generated .zed/tasks.json content:\n{contents}"));
    write_tasks_file(root_path, &contents, runner)?;
    log_message("wrote generated .zed/tasks.json to workspace");
    Ok(())
}

fn discover_cmake_targets(
    root_path: &str,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<Vec<CmakeTarget>> {
    let build_ninja = join_windows_path(&join_windows_path(root_path, "build"), "build.ninja");
    let escaped_path = powershell_single_quote(&build_ninja);
    let script = format!(
        "$ErrorActionPreference='Stop'; if (!(Test-Path -LiteralPath {escaped_path})) {{ return }}; \
         $targets = @(); \
         Get-Content -LiteralPath {escaped_path} | ForEach-Object {{ \
             if ($_ -match '^build\\s+([^:]+?\\.exe):') {{ \
                 $output = $Matches[1].Trim(); \
                 if ($output -notmatch '(^|/)CMakeFiles/') {{ \
                     $name = [IO.Path]::GetFileNameWithoutExtension($output); \
                     $targets += \"$name|$output|exe\"; \
                 }} \
             }} elseif ($_ -match '^build\\s+([^: ]+): phony') {{ \
                 $name = $Matches[1].Trim(); \
                 if ($name -notmatch '(^all$|^clean$|^edit_cache$|^rebuild_cache$|_autogen$|_autogen_timestamp_deps$|_automoc_json_extraction$|^cmake_object_order_depends_target_)') {{ \
                     $targets += \"$name||target\"; \
                 }} \
             }} \
         }}; \
         $targets | Sort-Object -Unique"
    );
    let args = vec!["-NoProfile".to_string(), "-Command".to_string(), script];
    let output = runner.run_command("powershell", &args)?;
    let stdout = crate::environment::tools::ensure_success("powershell", output)?;
    Ok(dedupe_cmake_targets(
        stdout
            .lines()
            .filter_map(parse_cmake_target_line)
            .collect::<Vec<_>>(),
    ))
}

fn parse_cmake_target_line(line: &str) -> Option<CmakeTarget> {
    let mut parts = line.trim().splitn(3, '|');
    let name = parts.next()?.trim();
    let output = parts.next()?.trim();
    let kind = parts.next()?.trim();
    let name = name.trim();
    if name.is_empty() {
        return None;
    }

    let output = if output.is_empty() {
        None
    } else {
        Some(output.to_string())
    };

    Some(CmakeTarget {
        name: name.to_string(),
        output,
        executable: kind == "exe",
    })
}

fn dedupe_cmake_targets(targets: Vec<CmakeTarget>) -> Vec<CmakeTarget> {
    let mut by_name = HashMap::<String, CmakeTarget>::new();
    for target in targets {
        by_name
            .entry(target.name.clone())
            .and_modify(|existing| {
                if target.executable && !existing.executable {
                    *existing = target.clone();
                }
            })
            .or_insert(target);
    }

    let mut targets = by_name.into_values().collect::<Vec<_>>();
    targets.sort_by(|left, right| left.name.cmp(&right.name));
    targets
}

fn first_autogen_target(
    root_path: &str,
    build_dir_name: &str,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<Option<String>> {
    let build_ninja =
        join_windows_path(&join_windows_path(root_path, build_dir_name), "build.ninja");
    let escaped_path = powershell_single_quote(&build_ninja);
    let script = format!(
        "$ErrorActionPreference='Stop'; if (!(Test-Path -LiteralPath {escaped_path})) {{ return }}; \
         Select-String -LiteralPath {escaped_path} -Pattern '^build ([^: ]+_autogen): phony' | \
         ForEach-Object {{ $_.Matches[0].Groups[1].Value }} | Select-Object -First 1"
    );
    let args = vec!["-NoProfile".to_string(), "-Command".to_string(), script];
    let output = runner.run_command("powershell", &args)?;
    let stdout = crate::environment::tools::ensure_success("powershell", output)?;
    Ok(stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned))
}

pub fn prepare_workspace_config_from_worktree(worktree: &zed::Worktree) -> ToolkitResult<()> {
    let clangd_contents = worktree.read_text_file(".clangd").ok();
    log_message(&format!(
        "worktree .clangd read result: exists={}",
        clangd_contents.is_some()
    ));
    let config = load_effective_config(worktree)?;
    prepare_workspace_config_with_config(
        &worktree.root_path(),
        clangd_contents,
        &config,
        &ZedCommandRunner,
    )
}

fn write_clangd_file(
    root_path: &str,
    contents: &str,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<()> {
    write_workspace_text_file(root_path, &[".clangd"], contents, runner)
}

fn write_tasks_file(
    root_path: &str,
    contents: &str,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<()> {
    write_workspace_text_file(root_path, &[".zed", "tasks.json"], contents, runner)
}

fn write_workspace_text_file(
    root_path: &str,
    components: &[&str],
    contents: &str,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<()> {
    match shell_for_root_path(root_path) {
        crate::build::shell::ShellKind::Powershell => {
            let path = join_windows_components(root_path, components);
            let script = write_powershell_text_file_script(&path, contents);
            let args = vec!["-NoProfile".to_string(), "-Command".to_string(), script];
            let output = runner.run_command("powershell", &args)?;
            crate::environment::tools::ensure_success("powershell", output).map(|_| ())
        }
        crate::build::shell::ShellKind::Sh => {
            let path = join_unix_components(root_path, components);
            let script = write_sh_text_file_script(&path, contents);
            let args = vec!["-lc".to_string(), script];
            let output = runner.run_command("sh", &args)?;
            crate::environment::tools::ensure_success("sh", output).map(|_| ())
        }
    }
}

fn write_powershell_text_file_script(path: &str, contents: &str) -> String {
    let escaped_path = powershell_single_quote(path);
    let escaped_contents = powershell_single_quote(contents);
    format!(
        "$ErrorActionPreference='Stop'; $path = {escaped_path}; \
         $dir = Split-Path -Parent $path; \
         if ($dir) {{ New-Item -ItemType Directory -Force -Path $dir | Out-Null }}; \
         $encoding = New-Object System.Text.UTF8Encoding($false); \
         [System.IO.File]::WriteAllText($path, {escaped_contents}, $encoding)"
    )
}

fn write_sh_text_file_script(path: &str, contents: &str) -> String {
    let escaped_path = sh_single_quote(path);
    let escaped_contents = sh_single_quote(contents);
    format!(
        "path={escaped_path}; mkdir -p -- \"$(dirname -- \"$path\")\" && printf %s {escaped_contents} > \"$path\""
    )
}

fn join_windows_components(root_path: &str, components: &[&str]) -> String {
    let mut path = root_path
        .trim_end_matches(['\\', '/'].as_slice())
        .to_string();
    for component in components {
        path.push('\\');
        path.push_str(component.trim_matches(['\\', '/'].as_slice()));
    }
    path
}

fn join_unix_components(root_path: &str, components: &[&str]) -> String {
    let mut path = root_path.trim_end_matches('/').to_string();
    for component in components {
        path.push('/');
        path.push_str(component.trim_matches('/'));
    }
    path
}

fn powershell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn sh_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn cmd_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}
fn one_line_preview(contents: &str) -> String {
    const MAX_PREVIEW_CHARS: usize = 500;

    let mut preview = String::new();
    for character in contents.chars().take(MAX_PREVIEW_CHARS) {
        if character.is_control() {
            preview.push(' ');
        } else {
            preview.push(character);
        }
    }

    if contents.chars().count() > MAX_PREVIEW_CHARS {
        preview.push_str("...");
    }

    preview
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::tools::CommandOutput;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    #[test]
    fn clangd_args_disable_header_insertion() {
        assert_eq!(
            clangd_args(),
            vec!["--header-insertion=never", "--log=verbose"]
        );
    }

    #[test]
    fn accepts_expected_language_server_id() {
        assert_eq!(validate_language_server_id("cpp-toolkit-clangd"), Ok(()));
    }

    #[test]
    fn build_clangd_command_includes_query_driver() {
        let command = build_clangd_command(
            "clangd".to_string(),
            Vec::new(),
            vec!["gcc".to_string(), "g++".to_string()],
        );

        assert!(command.args.contains(&"--query-driver=gcc,g++".to_string()));
    }

    #[test]
    fn uses_discovered_configured_clangd_command() {
        let command = resolve_clangd_command(
            "custom-clangd",
            Some("/usr/local/bin/custom-clangd".to_string()),
        )
        .unwrap();

        assert_eq!(command, "/usr/local/bin/custom-clangd");
    }

    #[test]
    fn allows_explicit_clangd_path_when_not_discovered() {
        let command = resolve_clangd_command("/opt/llvm/bin/clangd", None).unwrap();

        assert_eq!(command, "/opt/llvm/bin/clangd");
    }

    #[test]
    fn reports_missing_named_clangd_variant_when_not_discovered() {
        let error = resolve_clangd_command("clangd-18", None).unwrap_err();

        assert_eq!(error, ToolkitError::MissingTool("clangd-18".to_string()));
    }

    #[test]
    fn reports_missing_default_clangd_when_not_discovered() {
        let error = resolve_clangd_command("clangd", None).unwrap_err();

        assert_eq!(error, ToolkitError::MissingClangd);
    }

    #[test]
    fn workspace_clangd_input_uses_effective_config() {
        let config = crate::config::schema::EffectiveConfig {
            preset: "gcc-cmake-ninja".to_string(),
            toolchain: crate::config::schema::EffectiveToolchain {
                name: "gcc".to_string(),
                cc: "gcc".to_string(),
                cxx: "g++".to_string(),
            },
            build: crate::config::schema::EffectiveBuild {
                system: "cmake".to_string(),
                build_dir_style: crate::config::schema::BuildDirStyle::Build,
                build_dir: "out/release".to_string(),
                build_type: "Release".to_string(),
                configure: Some("cmake -S . -B out/release".to_string()),
                build: Some("cmake --build out/release".to_string()),
                clean: None,
            },
            run: crate::config::schema::EffectiveRun {
                command: Some("./app".to_string()),
                cwd: "$ZED_WORKTREE_ROOT".to_string(),
            },
            clangd: crate::config::schema::EffectiveClangd {
                command: "clangd".to_string(),
                compiler: "g++".to_string(),
                compile_commands_dir: "out/release".to_string(),
                extra_flags: vec!["-std=c++20".to_string()],
                query_driver: vec!["gcc".to_string(), "g++".to_string()],
            },
        };

        let input = workspace_clangd_input(&config, None, Vec::new());

        assert_eq!(input.compiler, "g++");
        assert_eq!(input.compile_commands_dir, "out/release");
        assert_eq!(input.extra_flags, vec!["-std=c++20"]);
        assert_eq!(input.msvc_include, None);
        assert_eq!(input.sdk_includes, Vec::<String>::new());
    }

    #[test]
    fn workspace_clangd_input_preserves_msvc_includes_when_present() {
        let config = crate::config::schema::EffectiveConfig {
            preset: "msvc-cmake-ninja".to_string(),
            toolchain: crate::config::schema::EffectiveToolchain {
                name: "msvc".to_string(),
                cc: "cl".to_string(),
                cxx: "cl".to_string(),
            },
            build: crate::config::schema::EffectiveBuild {
                system: "cmake".to_string(),
                build_dir_style: crate::config::schema::BuildDirStyle::Build,
                build_dir: "build".to_string(),
                build_type: "Debug".to_string(),
                configure: None,
                build: None,
                clean: None,
            },
            run: crate::config::schema::EffectiveRun {
                command: None,
                cwd: "$ZED_WORKTREE_ROOT".to_string(),
            },
            clangd: crate::config::schema::EffectiveClangd {
                command: "clangd".to_string(),
                compiler: "clang-cl".to_string(),
                compile_commands_dir: "build".to_string(),
                extra_flags: Vec::new(),
                query_driver: Vec::new(),
            },
        };

        let input = workspace_clangd_input(
            &config,
            Some(r"C:\VS\VC\Tools\MSVC\14.40.33807\include".to_string()),
            vec![r"C:\Windows Kits\10\Include\10.0.22621.0\ucrt".to_string()],
        );

        assert_eq!(
            input.msvc_include,
            Some(r"C:\VS\VC\Tools\MSVC\14.40.33807\include".to_string())
        );
        assert_eq!(
            input.sdk_includes,
            vec![r"C:\Windows Kits\10\Include\10.0.22621.0\ucrt".to_string()]
        );
    }

    #[test]
    fn msvc_configure_uses_configured_build_dir_and_type() {
        let runner = QueueRunner::new([
            CommandOutput {
                status: Some(0),
                stdout: "C:\\VS\\2022\\Community\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
        ]);

        let result = run_cmake_configure_for_compile_database(
            "C:/repo",
            "out/debug",
            "RelWithDebInfo",
            &runner,
        );

        assert_eq!(result, Ok(()));
        let calls = runner.calls.borrow();
        assert_eq!(calls.len(), 2);
        let configure_script = &calls[1].1[2];
        assert!(configure_script.contains(r"C:/repo\out/debug"));
        assert!(configure_script.contains("-DCMAKE_BUILD_TYPE=RelWithDebInfo"));
    }

    #[test]
    fn writes_workspace_files_with_sh_for_unix_roots() {
        let runner = QueueRunner::new([CommandOutput {
            status: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        }]);

        write_tasks_file("/home/me/project", "[]", &runner).unwrap();

        let calls = runner.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "sh");
        assert_eq!(calls[0].1[0], "-lc");
        assert!(calls[0].1[1].contains("/home/me/project/.zed/tasks.json"));
        assert!(calls[0].1[1].contains("mkdir -p"));
    }

    #[test]
    fn quotes_shell_strings_with_single_quotes() {
        assert_eq!(sh_single_quote("plain"), "'plain'");
        assert_eq!(sh_single_quote("a'b"), "'a'\\''b'");
    }

    #[test]
    fn rejects_unexpected_language_server_id() {
        let error = validate_language_server_id("other-lsp").unwrap_err();

        assert_eq!(
            error,
            ToolkitError::UnsupportedLanguageServer("other-lsp".to_string())
        );
    }

    #[test]
    fn keeps_user_clangd_config_but_still_writes_tasks() {
        let config = resolve_config(Some(crate::config::schema::UserConfig {
            preset: Some("gcc-cmake-ninja".to_string()),
            ..crate::config::schema::UserConfig::default()
        }))
        .unwrap();
        let runner = QueueRunner::new([CommandOutput {
            status: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        }]);

        let result = prepare_workspace_config_with_config(
            "C:/repo",
            Some("CompileFlags: {}".to_string()),
            &config,
            &runner,
        );

        assert_eq!(result, Ok(()));
        let calls = runner.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "powershell");
        assert!(calls[0].1.iter().any(|arg| {
            arg.contains("C++: Build") && arg.contains("'C:/repo\\.zed\\tasks.json'")
        }));
        assert!(
            !calls[0]
                .1
                .iter()
                .any(|arg| arg.contains("'C:/repo\\.clangd'"))
        );
    }

    #[test]
    fn msvc_generated_tasks_use_visual_studio_developer_environment() {
        let config = resolve_config(Some(crate::config::schema::UserConfig {
            preset: Some("msvc-cmake-ninja".to_string()),
            ..crate::config::schema::UserConfig::default()
        }))
        .unwrap();
        let runner = QueueRunner::new([
            CommandOutput {
                status: Some(0),
                stdout: "C:\\VS\\2022\\Community\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
        ]);

        write_generated_tasks("C:/repo", &config, &runner).unwrap();

        let calls = runner.calls.borrow();
        assert!(calls.iter().any(|(_, args)| {
            args.iter()
                .any(|arg| arg.contains("VsDevCmd.bat") && arg.contains("cmake --build build"))
        }));
    }

    #[test]
    fn overridden_toolchain_name_disables_msvc_compile_database_fallback() {
        let config = resolve_config(Some(crate::config::schema::UserConfig {
            preset: Some("msvc-cmake-ninja".to_string()),
            toolchain: crate::config::schema::ToolchainConfig {
                name: Some("clang".to_string()),
                ..crate::config::schema::ToolchainConfig::default()
            },
            ..crate::config::schema::UserConfig::default()
        }))
        .unwrap();

        assert!(!should_run_legacy_msvc_compile_database_fallback(&config));
    }

    #[test]
    fn overridden_toolchain_name_disables_msvc_clangd_environment_detection() {
        let config = resolve_config(Some(crate::config::schema::UserConfig {
            preset: Some("msvc-cmake-ninja".to_string()),
            toolchain: crate::config::schema::ToolchainConfig {
                name: Some("clang".to_string()),
                ..crate::config::schema::ToolchainConfig::default()
            },
            ..crate::config::schema::UserConfig::default()
        }))
        .unwrap();
        let runner = QueueRunner::new([]);

        let environment = discover_optional_msvc_environment(&config, &runner);

        assert!(environment.is_none());
        assert!(runner.calls.borrow().is_empty());
    }

    struct QueueRunner {
        outputs: RefCell<VecDeque<CommandOutput>>,
        calls: RefCell<Vec<(String, Vec<String>)>>,
    }

    impl QueueRunner {
        fn new(outputs: impl IntoIterator<Item = CommandOutput>) -> Self {
            Self {
                outputs: RefCell::new(outputs.into_iter().collect()),
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl crate::environment::tools::CommandRunner for QueueRunner {
        fn run_command(&self, command: &str, args: &[String]) -> ToolkitResult<CommandOutput> {
            self.calls
                .borrow_mut()
                .push((command.to_string(), args.to_vec()));
            self.outputs
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| ToolkitError::IoMessage("unexpected command".to_string()))
        }
    }

    #[test]
    fn config_driven_workspace_generation_writes_clangd_and_tasks_without_msvc_discovery() {
        let config = resolve_config(Some(crate::config::schema::UserConfig {
            preset: Some("gcc-cmake-ninja".to_string()),
            clangd: crate::config::schema::ClangdConfig {
                extra_flags: Some(vec!["-std=c++20".to_string()]),
                ..crate::config::schema::ClangdConfig::default()
            },
            ..crate::config::schema::UserConfig::default()
        }))
        .unwrap();
        let runner = QueueRunner::new([
            CommandOutput {
                status: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
        ]);

        let result = prepare_workspace_config_with_config("C:/repo", None, &config, &runner);

        assert_eq!(result, Ok(()));
        let calls = runner.calls.borrow();
        assert_eq!(calls.len(), 2);
        assert!(calls.iter().any(|(command, args)| {
            command == "powershell"
                && args.iter().any(|arg| {
                    arg.contains("[System.IO.File]::WriteAllText")
                        && arg.contains("'C:/repo\\.clangd'")
                        && arg.contains("Compiler: g++")
                        && arg.contains("-std=c++20")
                })
        }));
        assert!(calls.iter().any(|(command, args)| {
            command == "powershell"
                && args.iter().any(|arg| {
                    arg.contains("[System.IO.File]::WriteAllText")
                        && arg.contains("'C:/repo\\.zed\\tasks.json'")
                        && arg.contains("C++: Build")
                        && arg.contains("cmake --build build")
                })
        }));
    }

    #[test]
    fn continues_when_clangd_config_is_missing() {
        let runner = QueueRunner::new([
            CommandOutput {
                status: Some(0),
                stdout: "false\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "false\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "false\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "C:\\VS\\2022\\Community\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "14.38.33130\n14.40.33807\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "10.0.19041.0\n10.0.22621.0\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "ucrt\num\nshared\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
        ]);

        let result = prepare_workspace_config("C:/repo", None, &runner);

        assert_eq!(result, Ok(()));
        assert!(runner.calls.borrow().iter().any(|(command, args)| {
            command == "powershell"
                && args.iter().any(|arg| {
                    arg.contains("[System.IO.File]::WriteAllText")
                        && arg.contains("'C:/repo\\.clangd'")
                })
        }));
    }

    #[test]
    fn runs_cmake_configure_when_compile_database_is_missing() {
        use std::fs;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_dir = std::env::temp_dir().join(format!(
            "zed-msvc-cmake-configure-{}-{}",
            std::process::id(),
            test_id
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(
            temp_dir.join("CMakeLists.txt"),
            "cmake_minimum_required(VERSION 3.20)\nproject(test)\n",
        )
        .unwrap();

        let runner = QueueRunner::new([
            CommandOutput {
                status: Some(0),
                stdout: "false\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "false\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "true\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "C:\\VS\\2022\\Community\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "test_autogen\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "false\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "false\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "C:\\VS\\2022\\Community\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "14.40.33807\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "10.0.22621.0\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "ucrt\num\nshared\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
        ]);

        let result = prepare_workspace_config(
            temp_dir.to_str().expect("temp path should be valid UTF-8"),
            None,
            &runner,
        );

        assert_eq!(result, Ok(()));
        assert!(runner.calls.borrow().iter().any(|(command, args)| {
            command == "powershell"
                && args.iter().any(|arg| {
                    arg.contains("VsDevCmd.bat")
                        && arg.contains("-DCMAKE_CXX_COMPILER=cl")
                        && arg.contains("-DCMAKE_EXPORT_COMPILE_COMMANDS=ON")
                })
        }));
        assert!(runner.calls.borrow().iter().any(|(command, args)| {
            command == "cmake"
                && args.iter().any(|arg| arg == "--target")
                && args.iter().any(|arg| arg == "test_autogen")
        }));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn continues_when_compilation_database_is_present_in_root() {
        // This test requires real filesystem, creating a temporary directory
        use std::fs;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);

        let temp_dir = std::env::temp_dir().join(format!(
            "zed-msvc-integration-root-{}-{}",
            std::process::id(),
            test_id
        ));
        fs::create_dir_all(&temp_dir).unwrap();

        // Create compile_commands.json
        fs::write(temp_dir.join("compile_commands.json"), r#"[]"#).unwrap();

        let runner = QueueRunner::new([
            CommandOutput {
                status: Some(0),
                stdout: "C:\\VS\\2022\\Community\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "14.40.33807\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "10.0.22621.0\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "ucrt\num\nshared\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
        ]);

        prepare_workspace_config(
            temp_dir.to_str().expect("temp path should be valid UTF-8"),
            None,
            &runner,
        )
        .unwrap();

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn continues_when_compilation_database_is_present_in_build_subdirectory() {
        use std::fs;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);

        let temp_dir = std::env::temp_dir().join(format!(
            "zed-msvc-integration-build-{}-{}",
            std::process::id(),
            test_id
        ));
        fs::create_dir_all(&temp_dir).unwrap();

        // Create build subdirectory and compile_commands.json
        let build_dir = temp_dir.join("build");
        fs::create_dir_all(&build_dir).unwrap();
        fs::write(build_dir.join("compile_commands.json"), r#"[]"#).unwrap();

        let runner = QueueRunner::new([
            CommandOutput {
                status: Some(0),
                stdout: "C:\\VS\\2022\\Community\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "14.40.33807\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "10.0.22621.0\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "ucrt\num\nshared\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
        ]);

        prepare_workspace_config(
            temp_dir.to_str().expect("temp path should be valid UTF-8"),
            None,
            &runner,
        )
        .unwrap();

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
