use std::collections::HashMap;
use std::hash::Hash;
use std::io;

use chunkfs::{
    ChunkHash, Data, DataContainer, Database, IterableDatabase, Scrub, ScrubMeasurements,
};
use marline_scrub::decoder::{Decoder, GdeltaDecoder};

use crate::encoder::PalantirEncoder;
use crate::lifecycle_manager::LifecycleTierConfig;
use crate::metadata_manager::MetadataManager;
use crate::mock_rocksdb::MockRocksDBMap;
use crate::types::{BlockID, Chunk, SuperFeatureGenerator, TierConfig};
use marline_index::index::IndexError;

/// Core scrubbing pipeline that applies the Palantir method to a storage backend.
///
/// For every chunk in the database, `PalantirScrubber`:
/// 1. Generates super-features via [`SuperFeatureGenerator`].
/// 2. Looks up similar chunks in the multi-tier [`MetadataManager`].
/// 3. If a match is found, delta-encodes the chunk; otherwise stores it raw.
///
/// The decision to store a delta uses an adaptive compression-ratio threshold
/// that tracks a running average.
///
/// [`SuperFeatureGenerator`]: crate::types::SuperFeatureGenerator
pub struct PalantirScrubber<
    S,
    H: ChunkHash + Clone + Eq + Hash + Send + Sync + 'static,
    E,
    const N: usize,
> {
    /// The super-feature generator.
    sf_gen: S,
    /// Multi-tier similarity index.
    metadata_manager: MetadataManager<H, N>,
    /// Delta encoder.
    encoder: E,
    /// False-positive threshold for delta encoding ratio.
    fp_threshold: f64,
    /// Total chunks processed.
    chunks_processed: u64,
    /// Count of chunks stored as deltas.
    delta_stored: u64,
    /// Tracks chunk hash → base hash for stored deltas (chain rebuild).
    delta_bases: HashMap<H, H>,
}

impl<S, H: ChunkHash + Clone + Eq + Hash + Send + Sync + 'static, E, const N: usize>
    PalantirScrubber<S, H, E, N>
{
    /// Creates a new `PalantirScrubber`.
    ///
    /// # Arguments
    /// * `sf_gen` — The super-feature generator.
    /// * `encoder` — The delta encoder.
    /// * `tier_config` — Tier configuration (super-feature sizes per tier).
    /// * `lifecycle_configs` — Per-tier lifecycle policies.
    ///
    /// # Defaults
    ///
    /// | Field | Value |
    /// |-------|-------|
    /// | `fp_threshold` | `0.9` — false-positive ratio cap |
    /// | `avg_comp_ratio` | `1.0` — running average starts at 1.0 (no compression benefit) |
    /// | `chunks_processed` | `0` |
    pub fn new(
        sf_gen: S,
        encoder: E,
        tier_config: TierConfig<N>,
        lifecycle_configs: [LifecycleTierConfig; N],
    ) -> Self {
        Self {
            sf_gen,
            metadata_manager: MetadataManager::new(tier_config, lifecycle_configs),
            encoder,
            fp_threshold: 0.9,
            chunks_processed: 0,
            delta_stored: 0,
            delta_bases: HashMap::new(),
        }
    }

    pub fn delta_stored(&self) -> u64 {
        self.delta_stored
    }

    pub fn fp_table_size(&self) -> usize {
        self.metadata_manager.fp_table_size()
    }

    pub fn sf_table_size(&self) -> usize {
        self.metadata_manager.sf_table_size()
    }

    pub fn delta_bases(&self) -> &HashMap<H, H> {
        &self.delta_bases
    }

    pub fn update(&self) -> Result<(), IndexError> {
        self.metadata_manager.finish_version()
    }
}

impl<B, S, E, const N: usize> Scrub<[u8; 32], B, [u8; 32], MockRocksDBMap>
    for PalantirScrubber<S, [u8; 32], E, N>
where
    B: IterableDatabase<[u8; 32], DataContainer<[u8; 32]>>,
    S: SuperFeatureGenerator,
    E: PalantirEncoder,
{
    fn scrub<'a>(
        &mut self,
        database: &mut B,
        target_map: &mut MockRocksDBMap,
    ) -> io::Result<ScrubMeasurements>
    where
        [u8; 32]: 'a,
    {
        let start = std::time::Instant::now();
        let mut processed_data = 0;
        let data_left = 0;

        for (hash, container) in database.iterator_mut() {
            match container.extract() {
                Data::Chunk(chunk_data) => {
                    let chunk = Chunk::new(chunk_data.clone());
                    let super_features = self.sf_gen.generate(&chunk);
                    match self.metadata_manager.lookup_fingerprint(hash) {
                        Some(_) => {}
                        None => {
                            match self.metadata_manager.lookup_super_features(&super_features) {
                                Some((base_hash, _)) => {
                                    match decoder(target_map, &self.delta_bases, &base_hash.hash) {
                                        Ok(base_data) => {
                                            let delta = self.encoder.encode(chunk_data, &base_data);
                                            let delta_compressed =
                                                zstd::encode_all(delta.as_slice(), 0)?;
                                            let simple_compressed =
                                                zstd::encode_all(chunk_data.as_slice(), 0)?;
                                            let ratio = delta_compressed.len() as f64
                                                / simple_compressed.len() as f64;
                                            if ratio < self.fp_threshold {
                                                target_map.insert(hash.clone(), delta_compressed)?;
                                                self.delta_bases.insert(hash.clone(), base_hash.hash.clone());
                                                self.delta_stored += 1;
                                            } else {
                                                target_map
                                                    .insert(hash.clone(), chunk_data.clone())?;
                                            }
                                        }
                                        Err(_) => {
                                            target_map.insert(hash.clone(), chunk_data.clone())?;
                                        }
                                    }
                                    processed_data += chunk_data.len();
                                }
                                None => {
                                    target_map.insert(hash.clone(), chunk_data.clone())?;
                                    processed_data += chunk_data.len();
                                }
                            }

                            self.metadata_manager.add_block(
                                hash.clone(),
                                &super_features,
                                BlockID::new(hash.clone()),
                            );
                        }
                    }
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
}

fn decoder(
    target_map: &MockRocksDBMap,
    delta_bases: &HashMap<[u8; 32], [u8; 32]>,
    hash: &[u8; 32],
) -> io::Result<Vec<u8>> {
    let mut chain = Vec::new();
    let mut cur = *hash;
    while let Some(&base) = delta_bases.get(&cur) {
        chain.push(cur);
        cur = base;
    }
    let mut data = target_map.get(&cur)?;
    for &dh in chain.iter().rev() {
        let compressed = target_map.get(&dh)?;
        data = GdeltaDecoder::new(true).decode_chunk(data, &compressed);
    }
    Ok(data)
}
