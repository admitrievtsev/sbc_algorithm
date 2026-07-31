use marline_index::index::metrics::Metric;

use crate::metadata_manager::TierID;

type FilterFn = fn(self_metric: Metric) -> bool;

pub struct LifecycleTierConfig {
    default_metric: Metric,
    after_use_upd_fn: fn(Metric) -> Metric,
    between_backups_upd_fn: fn(Metric) -> Metric,
    drop_or_not_fn: FilterFn,
}

impl LifecycleTierConfig {
    pub fn new(
        default_metric: Metric,
        after_use_upd_fn: fn(Metric) -> Metric,
        between_backups_upd_fn: fn(Metric) -> Metric,
        drop_or_not_fn: FilterFn,
    ) -> Self {
        Self { default_metric, after_use_upd_fn, between_backups_upd_fn, drop_or_not_fn }
    }
}

pub struct LifecycleManager<const N: usize> {
    configs: [LifecycleTierConfig; N],
}

impl<const N: usize> LifecycleManager<N> {
    pub fn new(configs: [LifecycleTierConfig; N]) -> Self {
        Self { configs }
    }

    pub fn default_tier_metric(&self, tier_id: TierID) -> Metric {
        self.configs[tier_id as usize].default_metric
    }

    pub fn tier_after_use_upd_fn(&self, tier_id: TierID) -> fn(Metric) -> Metric {
        self.configs[tier_id as usize].after_use_upd_fn
    }

    pub fn tier_between_backups_upd_fn(&self, tier_id: TierID) -> fn(Metric) -> Metric {
        self.configs[tier_id as usize].between_backups_upd_fn
    }

    pub fn tier_drop_or_not_fn(&self, tier_id: TierID) -> FilterFn {
        self.configs[tier_id as usize].drop_or_not_fn
    }
}

impl Default for LifecycleManager<1> {
    fn default() -> Self {
        Self::new(Self::default_configs())
    }
}

impl LifecycleManager<1> {
    pub fn default_configs() -> [LifecycleTierConfig; 1] {
        [LifecycleTierConfig::new(0, |_| 0, |_| 0, |_| false)]
    }
}

impl LifecycleManager<2> {
    pub fn default_configs() -> [LifecycleTierConfig; 2] {
        [
            LifecycleTierConfig::new(0, |_| 0, |_| 0, |_| false),
            LifecycleTierConfig::new(6, |x| x, |x| x - 1, |x| x == 0),
        ]
    }
}

impl Default for LifecycleManager<3> {
    fn default() -> Self {
        let configs = Self::default_configs();
        Self::new(configs)
    }
}

impl LifecycleManager<3> {
    pub fn default_configs() -> [LifecycleTierConfig; 3] {
        [
            LifecycleTierConfig::new(0, |_| 0, |_| 0, |_| false),
            LifecycleTierConfig::new(6, |x| x, |x| x - 1, |x| x == 0),
            LifecycleTierConfig::new(3, |x| x, |x| x - 1, |x| x == 0),
        ]
    }
}

impl LifecycleManager<4> {
    pub fn default_configs() -> [LifecycleTierConfig; 4] {
        [
            LifecycleTierConfig::new(0, |_| 0, |_| 0, |_| false),
            LifecycleTierConfig::new(6, |x| x, |x| x - 1, |x| x == 0),
            LifecycleTierConfig::new(3, |x| x, |x| x - 1, |x| x == 0),
            LifecycleTierConfig::new(1, |x| x, |x| x - 1, |x| x == 0),
        ]
    }
}

impl LifecycleManager<5> {
    pub fn default_configs() -> [LifecycleTierConfig; 5] {
        [
            LifecycleTierConfig::new(0, |_| 0, |_| 0, |_| false),
            LifecycleTierConfig::new(6, |x| x, |x| x - 1, |x| x == 0),
            LifecycleTierConfig::new(3, |x| x, |x| x - 1, |x| x == 0),
            LifecycleTierConfig::new(1, |x| x, |x| x - 1, |x| x == 0),
            LifecycleTierConfig::new(0, |_| 0, |_| 0, |_| false),
        ]
    }
}
