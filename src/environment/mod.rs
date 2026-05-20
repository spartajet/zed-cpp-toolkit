pub mod msvc;
pub mod tools;
pub mod vswhere;
pub mod windows_sdk;

use crate::environment::tools::CommandRunner;
use crate::error::ToolkitResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsvcEnvironment {
    pub visual_studio_root: String,
    pub msvc_include: String,
    pub sdk_includes: Vec<String>,
}

pub fn discover_msvc_environment(runner: &impl CommandRunner) -> ToolkitResult<MsvcEnvironment> {
    let visual_studio_root = vswhere::discover_visual_studio(runner)?;
    let msvc_include = msvc::discover_msvc_include(runner, &visual_studio_root)?;
    let sdk_includes = windows_sdk::discover_windows_sdk_includes(runner);

    Ok(MsvcEnvironment {
        visual_studio_root,
        msvc_include,
        sdk_includes,
    })
}
