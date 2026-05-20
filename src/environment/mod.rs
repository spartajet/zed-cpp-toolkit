pub mod msvc;
pub mod tools;
pub mod vswhere;
pub mod windows_sdk;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsvcEnvironment {
    pub visual_studio_root: String,
    pub msvc_include: String,
    pub sdk_includes: Vec<String>,
}
