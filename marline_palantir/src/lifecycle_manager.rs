use crate::metadata_manager::TierID;

type FilterFn = fn(self_metric: u64) -> bool;

pub struct LifecycleTierConfig {
    default_metric: u64,
    after_use_upd_fn: fn(u64) -> u64,
    between_backups_upd_fn: fn(u64) -> u64,
    drop_or_not_fn: FilterFn,
}

impl LifecycleTierConfig {
    pub fn new(
        default_metric: u64,
        after_use_upd_fn: fn(u64) -> u64,
        between_backups_upd_fn: fn(u64) -> u64,
        drop_or_not_fn: fn(u64) -> bool,
    ) -> Self {
        Self { default_metric, after_use_upd_fn, between_backups_upd_fn, drop_or_not_fn }
    }
}

pub struct LifecycleManager {
    configs: Vec<LifecycleTierConfig>,
}

impl LifecycleManager {
    pub fn new(configs: Vec<LifecycleTierConfig>) -> Self {
        Self { configs }
    }

    pub fn default() -> Self {
        let configs = Self::default_configs();
        Self::new(configs)
    }

    pub fn default_configs() -> Vec<LifecycleTierConfig> {
        vec![
            LifecycleTierConfig::new(0, |_| 0, |_| 0, |_| true),
            LifecycleTierConfig::new(6, |x| x, |x| x - 1, |x| x != 0),
            LifecycleTierConfig::new(3, |x| x, |x| x - 1, |x| x != 0),
        ]
    }

    pub fn default_tier_metric(&self, tier_id: TierID) -> u64 {
        self.configs[tier_id as usize].default_metric
    }

    pub fn tier_after_use_upd_fn(&self, tier_id: TierID) -> fn(u64) -> u64 {
        self.configs[tier_id as usize].after_use_upd_fn
    }

    pub fn tier_between_backups_upd_fn(&self, tier_id: TierID) -> fn(u64) -> u64 {
        self.configs[tier_id as usize].between_backups_upd_fn
    }

    pub fn tier_drop_or_not_fn(&self, tier_id: TierID) -> fn(u64) -> bool {
        self.configs[tier_id as usize].drop_or_not_fn
    }
}
