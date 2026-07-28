use super::chunk::Chunk;

/// A single tiered similarity fingerprint.
///
/// Each `SuperFeature` belongs to a specific tier and carries a hash value
/// derived from a group of raw features.  The tier structure enables
/// progressive similarity search: coarse tiers trade precision for speed,
/// while finer tiers provide higher accuracy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SuperFeature {
    /// Which tier this feature belongs to (0, 1, 2, …).
    tier_id: u8,
    /// The hashed super-feature value.
    value: u32,
}

impl SuperFeature {
    pub fn new(tier_id: u8, value: u32) -> Self {
        Self { tier_id, value }
    }

    /// Returns the tier index of this super-feature.
    pub fn tier_id(&self) -> u8 {
        self.tier_id
    }

    /// Returns the hashed value of this super-feature.
    pub fn value(&self) -> u32 {
        self.value
    }
}

/// Configuration for multi-tier super-feature grouping.
///
/// `tier_list` specifies the number of super-features per tier (and the
/// sketch width for each tier's index).  The values must match the
/// super-feature counts produced by the hasher.  For example, if the hasher
/// uses group sizes `[4, 3, 2]` (LCM = 12), it produces 12/4 = 3, 12/3 = 4,
/// and 12/2 = 6 super-features per tier, so `tier_list` should be `[3, 4, 6]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierConfig<const N: usize> {
    /// Number of super-features per tier, ordered from coarsest to finest.
    pub tier_list: [u32; N],
}

impl<const N: usize> TierConfig<N> {
    /// Creates a new `TierConfig` with the given super-feature counts per tier.
    pub fn new(tier_list: [u32; N]) -> Self {
        Self { tier_list }
    }
}

/// Generates a set of [`SuperFeature`] values from a [`Chunk`].
///
/// Implementations define how raw chunk bytes are converted into
/// similarity-preserving fingerprints that can be indexed and searched.
pub trait SuperFeatureGenerator {
    /// Computes super-features for the given chunk.
    fn generate(&self, chunk: &Chunk) -> Vec<SuperFeature>;
}
