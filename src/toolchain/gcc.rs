use crate::config::schema::EffectiveConfig;

pub fn prepare_task_config(config: &EffectiveConfig) -> EffectiveConfig {
    config.clone()
}
