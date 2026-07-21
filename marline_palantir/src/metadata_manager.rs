use crate::tables::{FPTable, SFTable};
use crate::types::{BlockID, Fingerprint, SuperFeature};
use std::collections::HashMap;

pub type Version = u64;
pub type TierID = u8;

#[derive(Clone, Debug)]
pub struct TierConfig {
    pub tier_id: TierID,
    pub k: usize,            // Number of super-features per block in this tier
    pub max_versions: usize, // 0 is all versions
}

pub struct MetadataManager {
    fp_table: FPTable,
    sf_tables: HashMap<Version, HashMap<TierID, SFTable>>,
    current_version: Version,
    tier_configs: Vec<TierConfig>,
}

impl MetadataManager {
    pub fn new(tier_configs: Vec<TierConfig>) -> Self {
        Self {
            fp_table: FPTable::new(),
            sf_tables: HashMap::new(),
            current_version: 0,
            tier_configs,
        }
    }
    pub fn default() -> Self {
        Self::new(vec![
            TierConfig { tier_id: 1, k: 3, max_versions: 0 },
            TierConfig { tier_id: 2, k: 4, max_versions: 5 },
            TierConfig { tier_id: 3, k: 6, max_versions: 2 },
        ])
    }
    /// Function for accurate deduplication
    pub fn lookup_fingerprint(&self, fingerprint: &Fingerprint) -> Option<&BlockID> {
        self.fp_table.lookup(fingerprint)
    }
    /// Function of searching for base block tier-by-tier
    pub fn lookup_super_features(
        &self,
        super_features: &[SuperFeature],
    ) -> Option<(BlockID, TierID)> {
        for tier_config in &self.tier_configs {
            let tier_id = tier_config.tier_id;
            for super_feature in super_features {
                if super_feature.tier_id() != tier_id {
                    continue;
                }
                for (_ver, tiers) in &self.sf_tables {
                    if let Some(sf_table) = tiers.get(&tier_id) {
                        if let Some(candidates) = sf_table.lookup(super_feature) {
                            if let Some(block_id) = candidates.first() {
                                return Some((block_id.clone(), tier_id));
                            }
                        }
                    }
                }
            }
        }
        None
    }
    /// Add the block: FP + SF for all tiers
    pub fn add_block(
        &mut self,
        fingerprint: Fingerprint,
        super_features: &[SuperFeature],
        version: Version,
    ) {
        let block_id = BlockID::new(fingerprint, version);
        self.fp_table.insert(fingerprint, block_id.clone());
        let tier_tables = self.sf_tables.entry(version).or_insert_with(HashMap::new);
        for super_feature in super_features {
            let tier_id = super_feature.tier_id();
            tier_tables
                .entry(tier_id)
                .or_insert_with(SFTable::new)
                .insert(&block_id, &[*super_feature]);
        }
        for prev_version in 0..version {
            if let Some(prev_tiers) = self.sf_tables.get_mut(&prev_version) {
                for super_feature in super_features {
                    if let Some(sf_table) = prev_tiers.get_mut(&super_feature.tier_id()) {
                        sf_table.remove_sf(super_feature);
                    }
                }
            }
        }
    }
    /// Delete block from all tables
    pub fn remove_block(&mut self, block_id: &BlockID) {
        self.fp_table.remove(&block_id.fingerprint);
        for (_ver, tiers) in &mut self.sf_tables {
            for (_tier_id, sf_table) in tiers {
                sf_table.remove_block(block_id);
            }
        }
    }
    /// To start new version of backup
    pub fn start_version(&mut self, version: Version) {
        self.current_version = version;
        self.sf_tables.entry(version).or_insert_with(HashMap::new);
    }
    /// Finish version: delete all old SF-tables with lifecycle
    pub fn finish_version(&mut self) {
        let current = self.current_version;
        for tier_config in &self.tier_configs {
            if tier_config.max_versions > 0 {
                let cutoff = current.saturating_sub(tier_config.max_versions as u64);
                let expired: Vec<Version> = self.sf_tables.keys()
                    .filter(|&&v| v <= cutoff)
                    .cloned()
                    .collect();
                for version in expired {
                    if let Some(tiers) = self.sf_tables.get_mut(&version) {
                        tiers.remove(&tier_config.tier_id);
                    }
                    if let Some(tiers) = self.sf_tables.get(&version) {
                        if tiers.is_empty() {
                            self.sf_tables.remove(&version);
                        }
                    }
                }
            }
        }
    }
    pub fn fp_table_size(&self) -> usize {
        self.fp_table.len()
    }
    pub fn sf_table_size(&self) -> usize {
        self.sf_tables.values().flat_map(|tiers| tiers.values()).map(|t| t.len()).sum()
    }
}
