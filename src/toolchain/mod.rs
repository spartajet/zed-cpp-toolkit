pub mod clang;
pub mod custom;
pub mod gcc;
pub mod msvc;

use crate::config::schema::EffectiveConfig;
use crate::environment::tools::CommandRunner;
use crate::error::ToolkitResult;

pub fn prepare_task_config(
    config: &EffectiveConfig,
    runner: &impl CommandRunner,
) -> ToolkitResult<EffectiveConfig> {
    match config.toolchain.name.as_str() {
        "msvc" => msvc::prepare_task_config(config, runner),
        "gcc" => Ok(gcc::prepare_task_config(config)),
        "clang" => Ok(clang::prepare_task_config(config)),
        _ => Ok(custom::prepare_task_config(config)),
    }
}
