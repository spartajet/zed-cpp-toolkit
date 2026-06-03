use crate::config::merge::{parse_user_config, resolve_config};
use crate::config::schema::EffectiveConfig;
use crate::error::ToolkitResult;
use zed_extension_api as zed;

pub fn load_effective_config(worktree: &zed::Worktree) -> ToolkitResult<EffectiveConfig> {
    let user = match worktree.read_text_file(".zed/cpp-toolkit.toml") {
        Ok(contents) => Some(parse_user_config(&contents)?),
        Err(_) => None,
    };
    resolve_config(user)
}
