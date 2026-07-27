use std::collections::HashMap;
use std::hash::Hash;
use std::io;

use chunkfs::{ChunkHash, Data, DataContainer, IterableDatabase, Scrub, ScrubMeasurements};
use num::integer::gcd;

use crate::encoder::PalantirEncoder;
use crate::types::{Chunk, SuperFeature, SuperFeatureGenerator, TierConfig};

use marline_index::index::store::IndexStorage;
use marline_index::index::InvertedSketchIndex;
use marline_index::index::SketchIndexApi;
use marline_index::sketch::{FixedSketch, U32Sketch};

fn lcm_checked(a: u32, b: u32) -> Option<u32> {
    let gcd_val = gcd(a, b);
    (a / gcd_val).checked_mul(b)
}

fn lcm_vec(nums: &[u32]) -> Option<u32> {
    let mut res: u32 = 1;
    for &i in nums {
        res = lcm_checked(res, i)?;
    }
    Some(res)
}

/// Internal trait for a single tier of the similarity index.
trait TierIndex<H> {
    /// Looks up a stored chunk hash by its sketch values.
    fn search(&self, values: &[u32]) -> Option<H>;
    /// Inserts a chunk hash indexed by its sketch values.
    fn insert(&self, hash: &H, values: &[u32]);
}

/// A multi-tier similarity index backed by [`InvertedSketchIndex`] tiers.
///
/// Each tier corresponds to a group size in [`TierConfig::tier_list`].  The
/// number of super-features per tier is `lcm(tier_list) / group_size`, which
/// determines the sketch size for that tier.  Searches probe tiers from
/// coarsest (index 0) to finest; the first match is returned.
pub struct Index<H: Clone + Eq + Hash + Send + Sync + 'static> {
    tiers: Vec<Box<dyn TierIndex<H>>>,
}

/// A single tier wrapping an [`InvertedSketchIndex`] with a fixed sketch size `N`.
struct DefinedTier<H: Clone + Eq + Hash + Send + Sync, const N: usize> {
    tier: InvertedSketchIndex<H, U32Sketch<N>, IndexStorage<H, u32>>,
}

impl<H: Clone + Eq + Hash + Send + Sync, const N: usize> DefinedTier<H, N> {
    fn new() -> Self {
        Self { tier: InvertedSketchIndex::new(IndexStorage::new()) }
    }
}

impl<H: Clone + Eq + Hash + Send + Sync, const N: usize> TierIndex<H> for DefinedTier<H, N> {
    fn search(&self, values: &[u32]) -> Option<H> {
        let arr: [u32; N] = values.try_into().ok()?;
        let sketch = FixedSketch::new(arr).ok()?;
        self.tier.get(&sketch).ok()?
    }

    fn insert(&self, hash: &H, values: &[u32]) {
        if let Ok(arr) = values.try_into() {
            if let Ok(sketch) = FixedSketch::new(arr) {
                let _ = self.tier.put(hash, sketch);
            }
        }
    }
}

impl<H: Clone + Eq + Hash + Send + Sync + 'static> Index<H> {
    pub fn new(config: &TierConfig) -> Self {
        let features_num = lcm_vec(&config.tier_list).expect("tier_list LCM overflow") as usize;
        let mut tiers: Vec<Box<dyn TierIndex<H>>> = Vec::new();
        for &group_size in &config.tier_list {
            let num_sfs = features_num / group_size as usize;
            match num_sfs {
                2 => tiers.push(Box::new(DefinedTier::<H, 2>::new())),
                3 => tiers.push(Box::new(DefinedTier::<H, 3>::new())),
                4 => tiers.push(Box::new(DefinedTier::<H, 4>::new())),
                5 => tiers.push(Box::new(DefinedTier::<H, 5>::new())),
                6 => tiers.push(Box::new(DefinedTier::<H, 6>::new())),
                7 => tiers.push(Box::new(DefinedTier::<H, 7>::new())),
                8 => tiers.push(Box::new(DefinedTier::<H, 8>::new())),
                9 => tiers.push(Box::new(DefinedTier::<H, 9>::new())),
                10 => tiers.push(Box::new(DefinedTier::<H, 10>::new())),
                11 => tiers.push(Box::new(DefinedTier::<H, 11>::new())),
                12 => tiers.push(Box::new(DefinedTier::<H, 12>::new())),
                _ => panic!("unsupported number of super-features per tier: {}", num_sfs),
            }
        }
        Self { tiers }
    }
}

impl<H: Clone + Eq + Hash + Send + Sync + 'static> Default for Index<H> {
    fn default() -> Self {
        Self::new(&TierConfig::new(vec![4, 3, 2]))
    }
}

impl<H: Clone + Eq + Hash + Send + Sync + 'static> Index<H> {
    /// Searches for a stored chunk hash similar to the given super-features.
    ///
    /// Probes each tier in order (coarsest first).  Returns the first match
    /// found, or `None` if no similar chunk exists in any tier.
    pub fn search(&self, sfs: &[SuperFeature]) -> Option<H> {
        for (tier_idx, tier) in self.tiers.iter().enumerate() {
            let values: Vec<u32> = sfs
                .iter()
                .filter(|sf| sf.tier_id() == tier_idx as u8)
                .map(|sf| sf.value())
                .collect();
            if let Some(hash) = tier.search(&values) {
                return Some(hash);
            }
        }
        None
    }

    /// Inserts a chunk hash indexed by its super-features into all tiers.
    pub fn insert(&self, sfs: &[SuperFeature], hash: H) {
        for (tier_idx, tier) in self.tiers.iter().enumerate() {
            let values: Vec<u32> = sfs
                .iter()
                .filter(|sf| sf.tier_id() == tier_idx as u8)
                .map(|sf| sf.value())
                .collect();
            tier.insert(&hash, &values);
        }
    }
}

/// Core scrubbing pipeline that applies the Palantir method to a storage backend.
///
/// For every chunk in the database, `PalantirScrubber`:
/// 1. Generates super-features via [`SuperFeatureGenerator`].
/// 2. Looks up similar chunks in the multi-tier [`Index`].
/// 3. If a match is found, delta-encodes the chunk; otherwise stores it raw.
///
/// The decision to store a delta uses an adaptive compression-ratio threshold
/// that tracks a running average.
///
/// [`SuperFeatureGenerator`]: crate::types::SuperFeatureGenerator
pub struct PalantirScrubber<S, H: Clone + Eq + Hash + Send + Sync + 'static, E> {
    /// The super-feature generator.
    sf_gen: S,
    /// Multi-tier similarity index.
    index: Index<H>,
    /// Delta encoder.
    encoder: E,
    /// False-positive threshold for delta encoding ratio.
    fp_threshold: f64,
    /// Running average compression ratio.
    avg_comp_ratio: f64,
    /// Total chunks processed.
    chunks_processed: u64,
}

impl<S, H: Clone + Eq + Hash + Send + Sync + 'static, E> PalantirScrubber<S, H, E> {
    /// Creates a new `PalantirScrubber`.
    ///
    /// # Arguments
    /// * `sf_gen` — The super-feature generator.
    /// * `index` — The multi-tier similarity index.
    /// * `encoder` — The delta encoder.
    ///
    /// # Defaults
    ///
    /// | Field | Value |
    /// |-------|-------|
    /// | `fp_threshold` | `0.9` — false-positive ratio cap |
    /// | `avg_comp_ratio` | `1.0` — running average starts at 1.0 (no compression benefit) |
    /// | `chunks_processed` | `0` |
    pub fn new(sf_gen: S, index: Index<H>, encoder: E) -> Self {
        Self { sf_gen, index, encoder, fp_threshold: 0.9, avg_comp_ratio: 1.0, chunks_processed: 0 }
    }
}

impl<CDCHash, B, S, E> Scrub<CDCHash, B, CDCHash, HashMap<CDCHash, Vec<u8>>>
    for PalantirScrubber<S, CDCHash, E>
where
    CDCHash: ChunkHash + Send + Sync + 'static,
    B: IterableDatabase<CDCHash, DataContainer<CDCHash>>,
    S: SuperFeatureGenerator,
    E: PalantirEncoder,
{
    /// Runs the Palantir scrub over all chunks in `database`.
    ///
    /// Every chunk is processed through the feature-generation → lookup → delta-or-store
    /// pipeline.  Delta decisions are based on an adaptive compression-ratio heuristic:
    /// a delta is stored only when `ratio < fp_threshold × avg_comp_ratio`, where
    /// `avg_comp_ratio` is an EMA that tracks recent compression efficiency.
    ///
    /// # Note
    ///
    /// `Data::TargetChunk` entries are silently skipped (decoder integration is pending).
    fn scrub<'a>(
        &mut self,
        database: &mut B,
        target_map: &mut HashMap<CDCHash, Vec<u8>>,
    ) -> io::Result<ScrubMeasurements>
    where
        CDCHash: 'a,
    {
        let start = std::time::Instant::now();
        let mut processed_data = 0;
        let data_left = 0;

        for (hash, container) in database.iterator_mut() {
            match container.extract() {
                Data::Chunk(chunk_data) => {
                    let chunk = Chunk::new(chunk_data.clone());
                    let super_features = self.sf_gen.generate(&chunk);

                    match self.index.search(&super_features) {
                        Some(base_hash) => {
                            if let Some(base_data) = target_map.get(&base_hash) {
                                let delta = self.encoder.encode(chunk_data, base_data);
                                let delta_compressed = zstd::encode_all(delta.as_slice(), 0)?;
                                let simple_compressed = zstd::encode_all(chunk_data.as_slice(), 0)?;
                                let ratio =
                                    delta_compressed.len() as f64 / simple_compressed.len() as f64;

                                if ratio < self.fp_threshold * self.avg_comp_ratio {
                                    target_map.insert(hash.clone(), delta);
                                    self.avg_comp_ratio = self.avg_comp_ratio * 0.95 + ratio * 0.05;
                                } else {
                                    target_map.insert(hash.clone(), chunk_data.clone());
                                }
                            } else {
                                target_map.insert(hash.clone(), chunk_data.clone());
                            }
                            processed_data += chunk_data.len();
                        }
                        None => {
                            target_map.insert(hash.clone(), chunk_data.clone());
                            processed_data += chunk_data.len();
                        }
                    }

                    self.index.insert(&super_features, hash.clone());
                    container.make_target(vec![hash.clone()]);
                    self.chunks_processed += 1;
                }
                Data::TargetChunk(_) => {}
            }
        }

        Ok(ScrubMeasurements {
            processed_data,
            running_time: start.elapsed(),
            data_left,
            clusterization_report: None,
        })
    }
    // todo: add update() method for metadata manager
}
