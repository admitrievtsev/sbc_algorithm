use crate::lifecycle_manager::{LifecycleManager, LifecycleTierConfig};
use crate::tables::{FPTable, SFTable};
use crate::types::{BlockID, SuperFeature};
use chunkfs::ChunkHash;

pub type Version = u64;
pub type TierID = u8;

#[derive(Clone, Debug)]
pub struct TierConfig {
    pub tier_id: TierID,
    pub k: usize, // Number of super-features per block in this tier
}

pub struct MetadataManager<H: ChunkHash> {
    fp_table: FPTable<H>,
    sf_tables: Vec<Vec<SFTable<H>>>,
    tier_configs: Vec<TierConfig>,
    lifecycle_manager: LifecycleManager,
}

impl<H: ChunkHash> MetadataManager<H> {
    pub fn new(tier_configs: Vec<TierConfig>, lifecycle_configs: Vec<LifecycleTierConfig>) -> Self {
        let sf_tables: Vec<Vec<SFTable<H>>> = (0..tier_configs.len()).map(|_| Vec::new()).collect();
        Self {
            fp_table: FPTable::<H>::new(),
            sf_tables,
            tier_configs,
            lifecycle_manager: LifecycleManager::new(lifecycle_configs),
        }
    }

    pub fn default() -> Self {
        Self::new(
            vec![
                TierConfig { tier_id: 0, k: 3 },
                TierConfig { tier_id: 1, k: 4 },
                TierConfig { tier_id: 2, k: 6 },
            ],
            LifecycleManager::default_configs(),
        )
    }

    /// Function for accurate deduplication
    pub fn lookup_fingerprint(&self, fingerprint: &H) -> Option<&BlockID<H>> {
        self.fp_table.lookup(fingerprint)
    }

    /// Function of searching for base block tier-by-tier
    pub fn lookup_super_features(
        &mut self,
        super_features: &[SuperFeature],
    ) -> Option<(BlockID<H>, TierID)> {
        let mut first_index;
        let mut last_index = 0;

        for i in 0..self.tier_configs.len() {
            first_index = last_index;
            last_index += self.tier_configs[i].k;
            let slice = &super_features[first_index..last_index];

            for table in &mut self.sf_tables[i] {
                if let Some(block) = table.nearest(slice) {
                    table.update_metric(self.lifecycle_manager.tier_after_use_upd_fn(i as TierID));
                    return Some((block, i as TierID));
                }
            }
        }
        None
    }

    /// Add the block: FP + SF for all tiers
    pub fn add_block(
        &mut self,
        fingerprint: H,
        super_features: &[SuperFeature],
        version: Version,
    ) {
        let block_id = BlockID::<H>::new(fingerprint.clone(), version);
        self.fp_table.insert(fingerprint, block_id.clone());
        let mut by_tier: Vec<Vec<SuperFeature>> = vec![Vec::new(); self.tier_configs.len()];
        for sf in super_features {
            by_tier[sf.tier_id() as usize].push(*sf);
        }
        for tier_id in 0..self.tier_configs.len() {
            if !by_tier[tier_id].is_empty() {
                self.sf_tables[tier_id][0].insert(&block_id, &by_tier[tier_id]);
            }
        }
        for super_feature in super_features {
            let tier_id = super_feature.tier_id();
            for sf_table in &mut self.sf_tables[tier_id as usize][1..] {
                sf_table.remove_sf(super_feature);
            }
        }
    }

    /// To start new version of backup
    pub fn start_version(&mut self) {
        for (tier_id, tier_table) in self.sf_tables.iter_mut().enumerate() {
            tier_table.insert(
                0,
                SFTable::<H>::new(
                    tier_id as u8,
                    self.lifecycle_manager.default_tier_metric(tier_id as TierID),
                ),
            );
        }
    }

    /// Finish version: delete all old SF-tables with lifecycle
    pub fn finish_version(&mut self) {
        for (id, vec) in self.sf_tables.iter_mut().enumerate() {
            let upd_fn = self.lifecycle_manager.tier_between_backups_upd_fn(id as TierID);
            let filter_fn = self.lifecycle_manager.tier_drop_or_not_fn(id as TierID);

            vec.retain_mut(|table| {
                table.update_metric(upd_fn);
                filter_fn(table.get_metric())
            });
        }
    }

    pub fn fp_table_size(&self) -> usize {
        self.fp_table.len()
    }

    pub fn sf_table_size(&self) -> usize {
        self.sf_tables.iter().flat_map(|vec| vec.iter()).map(|table| table.len()).sum()
    }
}
