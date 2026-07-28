use crate::types::{Chunk, SuperFeature, SuperFeatureGenerator};
use crate::GEAR;
use num::integer::gcd;
use std::hash::{DefaultHasher, Hasher};

/// Generates super-features using a gear-hash rolling hash with random linear projections.
///
/// `PalantirHasher` implements the core feature-extraction algorithm of the
/// Palantir method.  It scans chunk bytes with a gear-hash rolling hash,
/// records minimum feature values per position, groups them by tier, and
/// produces a final set of [`SuperFeature`](crate::types::SuperFeature) values.
///
/// # How it works
///
/// 1. A rolling hash (`fp`) is updated for each byte using the `GEAR` table.
/// 2. When `fp` hits a zero value at the lowest `sampling_rate` bits, a
///    feature-minimisation step is triggered: each position's candidate value is
///    updated to `min(current, transform)` where `transform` is a linear
///    projection of the fingerprint.
/// 3. After the scan, raw features are grouped by `tier_list` sizes, sorted,
///    and hashed into the final [`SuperFeature`](crate::types::SuperFeature) values.
pub struct PalantirHasher {
    /// Number of trailing zero bits required to trigger a feature update.
    sampling_rate: u64,
    /// Random linear coefficients for feature-position transformations.
    linear_coefficients: Vec<u64>,
    /// Group sizes for each super-feature tier.
    tier_list: Vec<u32>,
    /// Total number of raw features (LCM of tier_list).
    features_num: usize,
}

/// Computes the least common multiple of `a` and `b`, returning `None` on overflow.
fn lcm_checked(a: u32, b: u32) -> Option<u32> {
    let gcd_val = gcd(a, b);
    (a / gcd_val).checked_mul(b)
}

/// Computes the least common multiple of all numbers in `nums`, returning `None` on overflow.
fn lcm_vec(nums: &[u32]) -> Option<u32> {
    let mut res: u32 = 1;
    for &i in nums {
        res = lcm_checked(res, i)?;
    }
    Some(res)
}

impl PalantirHasher {
    /// Creates a new `PalantirHasher`.
    ///
    /// The number of raw features is the least common multiple of all tier
    /// sizes.  Random linear coefficients are generated for each feature
    /// position.
    ///
    /// # Panics
    ///
    /// Panics if `lcm_vec(&tier_list)` overflows (i.e., the product of tier
    /// sizes exceeds `u32::MAX`).
    ///
    /// # Arguments
    /// * `sampling_rate` — Number of trailing-zero bits required on the
    ///   gear-hash fingerprint to trigger feature extraction.
    /// * `tier_list` — Group sizes for each tier (e.g., `vec![3, 4, 6]`).
    pub fn new(sampling_rate: u64, tier_list: Vec<u32>) -> Self {
        let features_num = lcm_vec(&tier_list).unwrap() as usize;
        let mut linear_coefficients = Vec::with_capacity(features_num);
        for _ in 0..features_num {
            linear_coefficients.push(rand::random());
        }
        Self { sampling_rate, linear_coefficients, tier_list, features_num }
    }
}

impl SuperFeatureGenerator for PalantirHasher {
    /// Generates super-features from a chunk's content.
    ///
    /// The algorithm:
    /// 1. Initialises `features_num` raw feature slots to `u64::MAX`.
    /// 2. Iterates each byte, updating the gear-hash fingerprint.
    /// 3. On a sampling hit, conditionally updates each feature slot with a
    ///    linear transformation of the fingerprint.
    /// 4. Groups the final features by tier, sorts each group, and hashes them
    ///    into a [`SuperFeature`](crate::types::SuperFeature).
    fn generate(&self, chunk: &Chunk) -> Vec<SuperFeature> {
        let data = chunk.as_bytes();
        let mut features = vec![u64::MAX; self.features_num];

        let mask = (1u64 << self.sampling_rate) - 1;
        let mut fp = 0u64;

        for &byte in data {
            fp = (fp << 1).wrapping_add(GEAR[byte as usize]);

            if fp & mask == 0 {
                for (i, feature) in features.iter_mut().enumerate().take(self.features_num) {
                    let transform =
                        self.linear_coefficients[i].wrapping_mul(fp).wrapping_add(byte as u64)
                            % (1u64 << 32);
                    if *feature > transform {
                        *feature = transform;
                    }
                }
            }
        }

        let mut super_features = Vec::new();
        for (tier_id, &group_size) in self.tier_list.iter().enumerate() {
            let gs = group_size as usize;
            let num_sf = self.features_num / gs;
            for sf_idx in 0..num_sf {
                let start = sf_idx * gs;
                let mut group: Vec<u64> = features[start..start + gs].to_vec();
                group.sort();
                let mut hasher = DefaultHasher::new();
                for &val in &group {
                    hasher.write_u64(val);
                }
                let hash = hasher.finish() as u32;
                super_features.push(SuperFeature::new(tier_id as u8, hash, 0));
            }
        }

        super_features
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Chunk;

    fn count_by_tier(sfs: &[SuperFeature]) -> Vec<usize> {
        let max_tier = sfs.iter().map(|sf| sf.tier_id() as usize).max().unwrap_or(0);
        let mut counts = vec![0usize; max_tier + 1];
        for sf in sfs {
            counts[sf.tier_id() as usize] += 1;
        }
        counts
    }

    #[test]
    fn superfeature_count_per_tier_matches_lcm() {
        let tier_list = vec![3u32, 4, 6];
        let hasher = PalantirHasher::new(8, tier_list.clone());
        let chunk = Chunk::new(vec![42u8; 1024]);
        let sfs = hasher.generate(&chunk);
        let counts = count_by_tier(&sfs);
        let features_num = lcm_vec(&tier_list).unwrap() as usize;

        for (i, &gs) in tier_list.iter().enumerate() {
            assert_eq!(
                counts[i],
                features_num / gs as usize,
                "tier {} (group size {}): expected {} super-features, got {}",
                i,
                gs,
                features_num / gs as usize,
                counts[i]
            );
        }
    }

    #[test]
    fn superfeature_count_for_coarse_tiers() {
        let tier_list = vec![2u32, 3];
        let hasher = PalantirHasher::new(8, tier_list.clone());
        let chunk = Chunk::new(vec![99u8; 512]);
        let sfs = hasher.generate(&chunk);
        let counts = count_by_tier(&sfs);
        let features_num = lcm_vec(&tier_list).unwrap() as usize;

        for (i, &gs) in tier_list.iter().enumerate() {
            assert_eq!(
                counts[i],
                features_num / gs as usize,
                "tier {} (group size {}): expected {} super-features, got {}",
                i,
                gs,
                features_num / gs as usize,
                counts[i]
            );
        }
    }
}
