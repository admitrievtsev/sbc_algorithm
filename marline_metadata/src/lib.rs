pub mod fp_table;
pub mod sf_table;

/// Number of backup version
pub type Version = u64;

/// Tier's ID (1 means the highest tier)
pub type TierID = usize;

/// Fingerprint 160 bits (SHA-1)
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct Fingerprint(pub [u8; 20]);

/// SuperFeature 64 bits. Hash of group of `s` features
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct SuperFeature(pub u64);

/// Block's ID consists of FP and version's number which block was first recorded
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct BlockId {
    pub fingerprint: Fingerprint,
    pub version: Version,
}

/// Configuration of a single tier of hierarchical super-features.
///
/// The parameters (k, s) determine the similarity detection threshold:
/// - `k` — number of super-features per block
/// - `s` — number of features in each super-feature
/// - `max_versions` — number of recent versions to retain (0 = all)
#[derive(Clone, Debug)]
pub struct TierConfig {
    pub tier_id: TierID,
    pub k: usize,
    pub s: usize,
    pub max_versions: usize,
}

impl TierConfig {
    /// Tier's configuration creation
    pub fn new(tier_id: TierID, k: usize, s: usize, max_versions: usize) -> Self {
        Self { tier_id, k, s, max_versions }
    }
}

/// Default configuration:
/// Tier-1: (k=3, s=4) — all versions
/// Tier-2: (k=4, s=3) — latest 5 versions
/// Tier-3: (k=6, s=2) — latest 2 versions
pub fn default_tier_configs() -> Vec<TierConfig> {
    vec![TierConfig::new(1, 3, 4, 0), TierConfig::new(2, 4, 3, 5), TierConfig::new(3, 6, 2, 2)]
}

/// Set of super-features for a single block, computed for all tiers.
///
/// `features_per_tier[i]` contains `k_i` super-features for the tier with index `i`.
/// The indexing corresponds to the order in `Vec<TierConfig>`.
#[derive(Clone, Debug)]
pub struct TieredFeatures {
    pub features_per_tier: Vec<Vec<SuperFeature>>,
}

impl TieredFeatures {
    /// Creates an empty feature set for a given number of tiers.
    pub fn new(num_tiers: usize) -> Self {
        Self { features_per_tier: vec![Vec::new(); num_tiers] }
    }

    /// Number of tiers
    pub fn num_tiers(&self) -> usize {
        self.features_per_tier.len()
    }
}
