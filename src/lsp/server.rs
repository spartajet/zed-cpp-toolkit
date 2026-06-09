use crate::build::shell::{ShellKind, shell_for_root_path};
use crate::build::tasks::generate_cpp_tasks_json;
use crate::cmake::{CmakeTarget, TaskOptions, discover_compile_database, generate_tasks_json};
use crate::config::loader::load_effective_config;
#[cfg(test)]
use crate::config::merge::{resolve_config, resolve_config_for_root_path};
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
    let config = resolve_config_for_root_path(None, root_path)?;
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
    if let Err(error) = write_example_config_if_missing(root_path, runner) {
        crate::debug::log_error("failed to write example cpp-toolkit config", &error);
        log_message("continuing without blocking clangd startup");
    }

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

    let cmake_targets = if config.build.system == "cmake" {
        let discovered =
            discover_cmake_targets_from_build_dir(root_path, &config.build.build_dir, runner);
        match discovered {
            Ok(targets) => {
                log_message(&format!(
                    "discovered {} cmake target(s), {} executable target(s)",
                    targets.len(),
                    targets.iter().filter(|target| target.executable).count()
                ));
                targets
            }
            Err(error) => {
                log_message(&format!("cmake target discovery skipped: {error}"));
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let contents =
        generate_cpp_tasks_json(&task_config, shell_for_root_path(root_path), &cmake_targets)?;
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
    discover_cmake_targets_from_build_dir(root_path, "build", runner)
}

fn discover_cmake_targets_from_build_dir(
    root_path: &str,
    build_dir: &str,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<Vec<CmakeTarget>> {
    if let Some(targets) = discover_cmake_targets_from_file_api(root_path, build_dir)? {
        return Ok(targets);
    }

    let Some(build_ninja) = read_cmake_build_ninja(root_path, build_dir, runner)? else {
        return Ok(Vec::new());
    };
    Ok(parse_cmake_targets_from_ninja(&build_ninja))
}

fn read_cmake_build_ninja(
    root_path: &str,
    build_dir: &str,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<Option<String>> {
    match shell_for_root_path(root_path) {
        ShellKind::Powershell => {
            let build_ninja =
                join_windows_path(&join_windows_path(root_path, build_dir), "build.ninja");
            let escaped_path = powershell_single_quote(&build_ninja);
            let script = format!(
                "$ErrorActionPreference='Stop'; if (Test-Path -LiteralPath {escaped_path}) {{ Get-Content -Raw -LiteralPath {escaped_path} }}"
            );
            let args = vec!["-NoProfile".to_string(), "-Command".to_string(), script];
            let output = runner.run_command("powershell", &args)?;
            crate::environment::tools::ensure_success("powershell", output)
                .map(|stdout| (!stdout.is_empty()).then_some(stdout))
        }
        ShellKind::Sh => {
            let build_ninja = join_unix_components(root_path, &[build_dir, "build.ninja"]);
            let script = format!(
                "if [ -f {} ]; then cat -- {}; fi",
                sh_single_quote(&build_ninja),
                sh_single_quote(&build_ninja)
            );
            let args = vec!["-c".to_string(), script];
            let output = runner.run_command("sh", &args)?;
            crate::environment::tools::ensure_success("sh", output)
                .map(|stdout| (!stdout.is_empty()).then_some(stdout))
        }
    }
}

fn parse_cmake_targets_from_ninja(contents: &str) -> Vec<CmakeTarget> {
    dedupe_cmake_targets(
        contents
            .lines()
            .filter_map(parse_cmake_ninja_build_line)
            .collect::<Vec<_>>(),
    )
}

fn discover_cmake_targets_from_file_api(
    root_path: &str,
    build_dir: &str,
) -> ToolkitResult<Option<Vec<CmakeTarget>>> {
    let reply_dir = std::path::Path::new(root_path)
        .join(build_dir)
        .join(".cmake")
        .join("api")
        .join("v1")
        .join("reply");
    if !reply_dir.is_dir() {
        return Ok(None);
    }

    let Some(index_path) = latest_cmake_file_api_index(&reply_dir)? else {
        return Ok(None);
    };
    let index = read_json_file(&index_path)?;
    let Some(codemodel_file) = cmake_codemodel_file_name(&index) else {
        return Ok(None);
    };
    let codemodel = read_json_file(&reply_dir.join(codemodel_file))?;
    let mut targets = Vec::new();
    let configurations = codemodel
        .get("configurations")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten();

    for configuration in configurations {
        let Some(model_targets) = configuration
            .get("targets")
            .and_then(serde_json::Value::as_array)
        else {
            continue;
        };
        for model_target in model_targets {
            let Some(json_file) = model_target
                .get("jsonFile")
                .and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            let target = read_json_file(&reply_dir.join(json_file))?;
            if let Some(target) = cmake_target_from_file_api_json(&target) {
                targets.push(target);
            }
        }
    }

    Ok(Some(dedupe_cmake_targets(targets)))
}

fn latest_cmake_file_api_index(
    reply_dir: &std::path::Path,
) -> ToolkitResult<Option<std::path::PathBuf>> {
    let mut indexes = std::fs::read_dir(reply_dir)
        .map_err(|error| ToolkitError::IoMessage(error.to_string()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(std::ffi::OsStr::to_str)
                .is_some_and(|name| name.starts_with("index-") && name.ends_with(".json"))
        })
        .collect::<Vec<_>>();
    indexes.sort();
    Ok(indexes.pop())
}

fn read_json_file(path: &std::path::Path) -> ToolkitResult<serde_json::Value> {
    let contents = std::fs::read_to_string(path)
        .map_err(|error| ToolkitError::IoMessage(error.to_string()))?;
    serde_json::from_str(&contents).map_err(|error| ToolkitError::IoMessage(error.to_string()))
}

fn cmake_codemodel_file_name(index: &serde_json::Value) -> Option<&str> {
    index
        .get("reply")
        .and_then(|reply| reply.get("codemodel-v2"))
        .and_then(|codemodel| codemodel.get("jsonFile"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            index
                .get("objects")
                .and_then(serde_json::Value::as_array)?
                .iter()
                .find(|object| {
                    object.get("kind").and_then(serde_json::Value::as_str) == Some("codemodel")
                })?
                .get("jsonFile")
                .and_then(serde_json::Value::as_str)
        })
}

fn cmake_target_from_file_api_json(target: &serde_json::Value) -> Option<CmakeTarget> {
    if target
        .get("imported")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }

    let name = target.get("name")?.as_str()?.trim();
    if !is_user_buildable_cmake_target_name(name) {
        return None;
    }

    let target_type = target.get("type")?.as_str()?;
    let executable = match target_type {
        "EXECUTABLE" => true,
        "STATIC_LIBRARY" | "SHARED_LIBRARY" | "MODULE_LIBRARY" | "OBJECT_LIBRARY" => false,
        _ => return None,
    };
    let output = file_api_primary_artifact(target, executable);

    Some(CmakeTarget {
        name: name.to_string(),
        output,
        executable,
    })
}

fn file_api_primary_artifact(target: &serde_json::Value, executable: bool) -> Option<String> {
    let artifacts = target.get("artifacts")?.as_array()?;
    artifacts
        .iter()
        .filter_map(|artifact| artifact.get("path").and_then(serde_json::Value::as_str))
        .find(|path| !executable || executable_artifact_path(path))
        .or_else(|| {
            artifacts
                .iter()
                .filter_map(|artifact| artifact.get("path").and_then(serde_json::Value::as_str))
                .next()
        })
        .map(ToOwned::to_owned)
}

fn executable_artifact_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".exe")
        || (!lower.ends_with(".pdb")
            && !lower.ends_with(".ilk")
            && !lower.ends_with(".dll")
            && !lower.ends_with(".lib")
            && !lower.ends_with(".a")
            && !lower.ends_with(".so")
            && !lower.ends_with(".dylib"))
}

fn parse_cmake_ninja_build_line(line: &str) -> Option<CmakeTarget> {
    let line = line.trim();
    let rest = line.strip_prefix("build ")?;
    let (outputs, rule_and_inputs) = rest.split_once(':')?;
    let output = primary_ninja_output(outputs)?.trim();
    if output.is_empty() || output.contains("CMakeFiles/") || output.contains("CMakeFiles\\") {
        return None;
    }

    let rule = rule_and_inputs
        .split_whitespace()
        .next()
        .unwrap_or_default();
    if rule.contains("EXECUTABLE_LINKER") {
        let name = target_name_from_output(output);
        if !is_user_buildable_cmake_target_name(&name) {
            return None;
        }
        return Some(CmakeTarget {
            name,
            output: Some(output.to_string()),
            executable: true,
        });
    }

    if rule == "phony" {
        let name = output.trim();
        if !is_user_buildable_cmake_target_name(name)
            || matches!(name, "all" | "clean" | "edit_cache" | "rebuild_cache")
        {
            return None;
        }
        return Some(CmakeTarget {
            name: name.to_string(),
            output: None,
            executable: false,
        });
    }

    None
}

fn primary_ninja_output(outputs: &str) -> Option<&str> {
    let explicit_outputs = outputs.split('|').next()?.trim();
    explicit_outputs.split_whitespace().next()
}

fn target_name_from_output(output: &str) -> String {
    let file_name = output.rsplit(['/', '\\']).next().unwrap_or(output);
    file_name
        .strip_suffix(".exe")
        .unwrap_or(file_name)
        .to_string()
}

fn dedupe_cmake_targets(targets: Vec<CmakeTarget>) -> Vec<CmakeTarget> {
    let mut by_name = HashMap::<String, CmakeTarget>::new();
    for target in targets {
        by_name
            .entry(target.name.clone())
            .and_modify(|existing| {
                if should_replace_cmake_target(existing, &target) {
                    *existing = target.clone();
                }
            })
            .or_insert(target);
    }

    let mut targets = by_name.into_values().collect::<Vec<_>>();
    targets.sort_by(|left, right| left.name.cmp(&right.name));
    targets
}

fn should_replace_cmake_target(existing: &CmakeTarget, candidate: &CmakeTarget) -> bool {
    if candidate.executable && !existing.executable {
        return true;
    }

    if candidate.executable == existing.executable {
        return output_path_score(candidate.output.as_deref())
            > output_path_score(existing.output.as_deref());
    }

    false
}

fn output_path_score(output: Option<&str>) -> usize {
    let Some(output) = output else {
        return 0;
    };

    let has_directory = output.contains('/') || output.contains('\\');
    1 + usize::from(has_directory)
}

fn is_user_buildable_cmake_target_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains("::")
        && !name.contains('/')
        && !name.contains('\\')
        && !name.ends_with("_autogen")
        && !name.ends_with("_autogen_timestamp_deps")
        && !name.ends_with("_automoc_json_extraction")
        && !name.starts_with("cmake_object_order_depends_target_")
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

fn write_example_config_if_missing(
    root_path: &str,
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<()> {
    for example in CPP_TOOLKIT_EXAMPLE_CONFIGS {
        let components = [".zed", example.file_name];
        if workspace_file_exists(root_path, &components, runner)? {
            log_message(&format!(
                ".zed/{} already exists; leaving it unchanged",
                example.file_name
            ));
            continue;
        }

        write_workspace_text_file(root_path, &components, example.contents, runner)?;
        log_message(&format!("wrote .zed/{} example config", example.file_name));
    }
    Ok(())
}

fn workspace_file_exists(
    root_path: &str,
    components: &[&str],
    runner: &impl crate::environment::tools::CommandRunner,
) -> ToolkitResult<bool> {
    match shell_for_root_path(root_path) {
        crate::build::shell::ShellKind::Powershell => {
            let path = join_windows_components(root_path, components);
            let script = format!(
                "if (Test-Path -LiteralPath {}) {{ 'True' }} else {{ 'False' }}",
                powershell_single_quote(&path)
            );
            let args = vec!["-NoProfile".to_string(), "-Command".to_string(), script];
            let output = runner.run_command("powershell", &args)?;
            crate::environment::tools::ensure_success("powershell", output)
                .map(|stdout| stdout.trim().eq_ignore_ascii_case("true"))
        }
        crate::build::shell::ShellKind::Sh => {
            let path = join_unix_components(root_path, components);
            let script = format!(
                "if [ -e {} ]; then printf true; else printf false; fi",
                sh_single_quote(&path)
            );
            let args = vec!["-c".to_string(), script];
            let output = runner.run_command("sh", &args)?;
            crate::environment::tools::ensure_success("sh", output)
                .map(|stdout| stdout.trim() == "true")
        }
    }
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
            let args = vec!["-c".to_string(), script];
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

struct ExampleConfig {
    file_name: &'static str,
    contents: &'static str,
}

const CPP_TOOLKIT_EXAMPLE_CONFIGS: &[ExampleConfig] = &[
    ExampleConfig {
        file_name: "cpp-toolkit.windows.example.toml",
        contents: WINDOWS_EXAMPLE_CONFIG,
    },
    ExampleConfig {
        file_name: "cpp-toolkit.linux.example.toml",
        contents: LINUX_EXAMPLE_CONFIG,
    },
    ExampleConfig {
        file_name: "cpp-toolkit.macos.example.toml",
        contents: MACOS_EXAMPLE_CONFIG,
    },
];

const WINDOWS_EXAMPLE_CONFIG: &str = r#"# Example cpp-toolkit configuration for Windows/MSVC.
# Copy this file to .zed/cpp-toolkit.toml and edit it for your project.
# After editing, restart the language server (command palette -> "clangd: Restart") to apply changes.

preset = "msvc-cmake-ninja"

[toolchain]
name = "msvc"
cc = "cl"
cxx = "cl"

[build]
system = "cmake"
build_dir_style = "build"
build_type = "Debug"
configure = "cmake -S . -B {build_dir} -G Ninja -DCMAKE_BUILD_TYPE={build_type} -DCMAKE_C_COMPILER=cl -DCMAKE_CXX_COMPILER=cl -DCMAKE_EXPORT_COMPILE_COMMANDS=ON"
build = "cmake --build {build_dir}"
clean = "cmake --build {build_dir} --target clean"

# [run] is optional for cmake projects: executable targets are auto-discovered from build.ninja.
# Uncomment and edit to override the auto-discovered run command.
# [run]
# command = ".\\build\\app.exe"
# cwd = "$ZED_WORKTREE_ROOT"

[clangd]
command = "clangd"
compiler = "clang-cl"
compile_commands_dir = "build"
extra_flags = ["/std:c++20"]
query_driver = []
"#;

const LINUX_EXAMPLE_CONFIG: &str = r#"# Example cpp-toolkit configuration for Linux/GCC.
# Copy this file to .zed/cpp-toolkit.toml and edit it for your project.
# After editing, restart the language server (command palette -> "clangd: Restart") to apply changes.

preset = "gcc-cmake-ninja"

[toolchain]
name = "gcc"
cc = "gcc"
cxx = "g++"

[build]
system = "cmake"
build_dir_style = "build"
build_type = "Debug"
configure = "cmake -S . -B {build_dir} -G Ninja -DCMAKE_BUILD_TYPE={build_type} -DCMAKE_C_COMPILER=gcc -DCMAKE_CXX_COMPILER=g++ -DCMAKE_EXPORT_COMPILE_COMMANDS=ON"
build = "cmake --build {build_dir}"
clean = "cmake --build {build_dir} --target clean"

# [run] is optional for cmake projects: executable targets are auto-discovered from build.ninja.
# Uncomment and edit to override the auto-discovered run command.
# [run]
# command = "./build/app"
# cwd = "$ZED_WORKTREE_ROOT"

[clangd]
command = "clangd"
compiler = "g++"
compile_commands_dir = "build"
extra_flags = ["-std=c++20"]
query_driver = ["gcc", "g++"]
"#;

const MACOS_EXAMPLE_CONFIG: &str = r#"# Example cpp-toolkit configuration for macOS/Clang.
# Copy this file to .zed/cpp-toolkit.toml and edit it for your project.
# After editing, restart the language server (command palette -> "clangd: Restart") to apply changes.

preset = "clang-cmake-ninja"

[toolchain]
name = "clang"
cc = "clang"
cxx = "clang++"

[build]
system = "cmake"
build_dir_style = "build"
build_type = "Debug"
configure = "cmake -S . -B {build_dir} -G Ninja -DCMAKE_BUILD_TYPE={build_type} -DCMAKE_C_COMPILER=clang -DCMAKE_CXX_COMPILER=clang++ -DCMAKE_EXPORT_COMPILE_COMMANDS=ON"
build = "cmake --build {build_dir}"
clean = "cmake --build {build_dir} --target clean"

# [run] is optional for cmake projects: executable targets are auto-discovered from build.ninja.
# Uncomment and edit to override the auto-discovered run command.
# [run]
# command = "./build/app"
# cwd = "$ZED_WORKTREE_ROOT"

[clangd]
command = "clangd"
compiler = "clang++"
compile_commands_dir = "build"
extra_flags = ["-std=c++20"]
query_driver = ["clang", "clang++"]
"#;

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
        assert_eq!(calls[0].1[0], "-c");
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
        let runner = QueueRunner::new([
            existing_example_output(),
            existing_example_output(),
            existing_example_output(),
            CommandOutput {
                status: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
        ]);

        let result = prepare_workspace_config_with_config(
            "C:/repo",
            Some("CompileFlags: {}".to_string()),
            &config,
            &runner,
        );

        assert_eq!(result, Ok(()));
        let calls = runner.calls.borrow();
        assert!(calls.iter().any(|(command, args)| {
            command == "powershell"
                && args.iter().any(|arg| {
                    arg.contains("C++: Build") && arg.contains("'C:/repo\\.zed\\tasks.json'")
                })
        }));
        assert!(
            !calls
                .iter()
                .any(|(_, args)| args.iter().any(|arg| arg.contains("'C:/repo\\.clangd'")))
        );
    }

    #[test]
    fn writes_platform_example_configs_during_workspace_preparation() {
        let config = resolve_config(Some(crate::config::schema::UserConfig {
            preset: Some("gcc-cmake-ninja".to_string()),
            ..crate::config::schema::UserConfig::default()
        }))
        .unwrap();
        let runner = QueueRunner::new([
            CommandOutput {
                status: Some(0),
                stdout: "False\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "False\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "False\n".to_string(),
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
        assert_platform_example_written(
            &calls,
            "cpp-toolkit.windows.example.toml",
            "preset = \"msvc-cmake-ninja\"",
            "compiler = \"clang-cl\"",
        );
        assert_platform_example_written(
            &calls,
            "cpp-toolkit.linux.example.toml",
            "preset = \"gcc-cmake-ninja\"",
            "compiler = \"g++\"",
        );
        assert_platform_example_written(
            &calls,
            "cpp-toolkit.macos.example.toml",
            "preset = \"clang-cmake-ninja\"",
            "compiler = \"clang++\"",
        );
    }

    #[test]
    fn does_not_overwrite_existing_platform_example_config() {
        let runner = QueueRunner::new([
            CommandOutput {
                status: Some(0),
                stdout: "True\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "False\n".to_string(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            },
            CommandOutput {
                status: Some(0),
                stdout: "True\n".to_string(),
                stderr: String::new(),
            },
        ]);

        write_example_config_if_missing("C:/repo", &runner).unwrap();

        let calls = runner.calls.borrow();
        assert!(!platform_example_was_written(
            &calls,
            "cpp-toolkit.windows.example.toml"
        ));
        assert!(platform_example_was_written(
            &calls,
            "cpp-toolkit.linux.example.toml"
        ));
        assert!(!platform_example_was_written(
            &calls,
            "cpp-toolkit.macos.example.toml"
        ));
    }

    fn assert_platform_example_written(
        calls: &[(String, Vec<String>)],
        file_name: &str,
        preset: &str,
        compiler: &str,
    ) {
        assert!(
            calls.iter().any(|(command, args)| {
                command == "powershell"
                    && args.iter().any(|arg| {
                        arg.contains("[System.IO.File]::WriteAllText")
                            && arg.contains(file_name)
                            && arg.contains(preset)
                            && arg.contains(compiler)
                            && arg.contains("Copy this file to .zed/cpp-toolkit.toml")
                    })
            }),
            "expected {file_name} to be written"
        );
    }

    fn platform_example_was_written(calls: &[(String, Vec<String>)], file_name: &str) -> bool {
        calls.iter().any(|(command, args)| {
            command == "powershell"
                && args.iter().any(|arg| {
                    arg.contains("[System.IO.File]::WriteAllText") && arg.contains(file_name)
                })
        })
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
            // discover_cmake_targets_from_build_dir (powershell - no build.ninja found)
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

        write_generated_tasks("C:/repo", &config, &runner).unwrap();

        let calls = runner.calls.borrow();
        assert!(calls.iter().any(|(_, args)| {
            args.iter()
                .any(|arg| arg.contains("VsDevCmd.bat") && arg.contains("cmake --build build"))
        }));
    }

    #[test]
    fn discovers_cmake_targets_from_file_api_codemodel() {
        use std::fs;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_dir = std::env::temp_dir().join(format!(
            "zed-msvc-cmake-file-api-{}-{}",
            std::process::id(),
            test_id
        ));
        let reply_dir = temp_dir
            .join("build")
            .join(".cmake")
            .join("api")
            .join("v1")
            .join("reply");
        fs::create_dir_all(&reply_dir).unwrap();
        fs::write(
            reply_dir.join("index-2026-06-08T15-43-28-0570.json"),
            r#"{
              "reply": {
                "codemodel-v2": {
                  "jsonFile": "codemodel-v2-test.json"
                }
              }
            }"#,
        )
        .unwrap();
        fs::write(
            reply_dir.join("codemodel-v2-test.json"),
            r#"{
              "configurations": [
                {
                  "targets": [
                    {
                      "name": "QEnhancedCustomPlot",
                      "jsonFile": "target-lib.json"
                    },
                    {
                      "name": "demo_realtime",
                      "jsonFile": "target-demo.json"
                    },
                    {
                      "name": "demo_realtime_autogen",
                      "jsonFile": "target-autogen.json"
                    },
                    {
                      "name": "Qt6::Core",
                      "jsonFile": "target-imported.json"
                    }
                  ]
                }
              ]
            }"#,
        )
        .unwrap();
        fs::write(
            reply_dir.join("target-lib.json"),
            r#"{
              "name": "QEnhancedCustomPlot",
              "type": "STATIC_LIBRARY",
              "artifacts": [
                { "path": "QEnhancedCustomPlot.lib" }
              ]
            }"#,
        )
        .unwrap();
        fs::write(
            reply_dir.join("target-demo.json"),
            r#"{
              "name": "demo_realtime",
              "type": "EXECUTABLE",
              "artifacts": [
                { "path": "demos/demo_realtime/demo_realtime.exe" },
                { "path": "demos/demo_realtime/demo_realtime.pdb" }
              ]
            }"#,
        )
        .unwrap();
        fs::write(
            reply_dir.join("target-autogen.json"),
            r#"{
              "name": "demo_realtime_autogen",
              "type": "UTILITY"
            }"#,
        )
        .unwrap();
        fs::write(
            reply_dir.join("target-imported.json"),
            r#"{
              "name": "Qt6::Core",
              "type": "SHARED_LIBRARY",
              "imported": true,
              "artifacts": [
                { "path": "C:/Qt/bin/Qt6Cored.dll" }
              ]
            }"#,
        )
        .unwrap();

        let runner = QueueRunner::new([CommandOutput {
            status: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        }]);
        let targets = discover_cmake_targets_from_build_dir(
            temp_dir.to_str().expect("temp path should be valid UTF-8"),
            "build",
            &runner,
        )
        .unwrap();

        assert_eq!(
            targets,
            vec![
                CmakeTarget {
                    name: "QEnhancedCustomPlot".to_string(),
                    output: Some("QEnhancedCustomPlot.lib".to_string()),
                    executable: false,
                },
                CmakeTarget {
                    name: "demo_realtime".to_string(),
                    output: Some("demos/demo_realtime/demo_realtime.exe".to_string()),
                    executable: true,
                },
            ]
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn discovers_cmake_executable_targets_from_unix_ninja_build_file() {
        let runner = QueueRunner::new([CommandOutput {
            status: Some(0),
            stdout: "build qt_demo: CXX_EXECUTABLE_LINKER__qt_demo_Debug CMakeFiles/qt_demo.dir/main.cpp.o\nbuild qt_demo: phony qt_demo\nbuild /home/guo/Code/cpp/qt-demo/CMakeLists.txt: phony\n".to_string(),
            stderr: String::new(),
        }]);

        let targets =
            discover_cmake_targets_from_build_dir("/home/me/project", "cmake-build-debug", &runner)
                .unwrap();

        assert_eq!(
            targets,
            vec![CmakeTarget {
                name: "qt_demo".to_string(),
                output: Some("qt_demo".to_string()),
                executable: true,
            }]
        );

        let calls = runner.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "sh");
        assert_eq!(calls[0].1[0], "-c");
        assert!(calls[0].1[1].contains("/home/me/project/cmake-build-debug/build.ninja"));
    }

    #[test]
    fn ninja_target_dedupe_prefers_executable_with_directory_path() {
        let targets = dedupe_cmake_targets(vec![
            CmakeTarget {
                name: "demo_playback".to_string(),
                output: Some("demo_playback.exe".to_string()),
                executable: true,
            },
            CmakeTarget {
                name: "demo_playback".to_string(),
                output: Some("demos/demo_playback/demo_playback.exe".to_string()),
                executable: true,
            },
        ]);

        assert_eq!(
            targets,
            vec![CmakeTarget {
                name: "demo_playback".to_string(),
                output: Some("demos/demo_playback/demo_playback.exe".to_string()),
                executable: true,
            }]
        );
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

    fn existing_example_output() -> CommandOutput {
        CommandOutput {
            status: Some(0),
            stdout: "True\n".to_string(),
            stderr: String::new(),
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
            existing_example_output(),
            existing_example_output(),
            existing_example_output(),
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
            existing_example_output(),
            existing_example_output(),
            existing_example_output(),
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
                        && arg.contains("Compiler: clang-cl")
                        && !arg.contains("Compiler: g++")
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
            existing_example_output(),
            existing_example_output(),
            existing_example_output(),
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
