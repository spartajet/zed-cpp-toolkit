use crate::config::schema::UserConfig;

pub fn preset_config(name: &str) -> Option<UserConfig> {
    match name {
        "msvc-cmake-ninja" => Some(UserConfig::default()),
        "gcc-cmake-ninja" => Some(UserConfig::default()),
        "clang-cmake-ninja" => Some(UserConfig::default()),
        "gcc-make" => Some(UserConfig::default()),
        "clang-make" => Some(UserConfig::default()),
        "custom" => Some(UserConfig::default()),
        _ => None,
    }
}
