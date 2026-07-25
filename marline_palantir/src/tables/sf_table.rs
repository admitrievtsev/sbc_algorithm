use crate::types::{BlockID, SuperFeature};
use marline_index::index::store::IndexStorage;
use marline_index::index::{InvertedSketchIndex, SketchIndexApi};
use marline_index::sketch::U32Sketch;
use chunkfs::ChunkHash;

enum IndexBackend<H: ChunkHash> {
    T3(InvertedSketchIndex<BlockID<H>, U32Sketch<3>, IndexStorage<BlockID<H>, U32Sketch<3>>>),
    T4(InvertedSketchIndex<BlockID<H>, U32Sketch<4>, IndexStorage<BlockID<H>, U32Sketch<4>>>),
    T6(InvertedSketchIndex<BlockID<H>, U32Sketch<6>, IndexStorage<BlockID<H>, U32Sketch<6>>>),
}
pub struct SFTable<H: ChunkHash> {
    index: IndexBackend<H>,
    metric: u64,
}

impl<H: ChunkHash> IndexBackend<H> {
    pub fn new(tier_id: u8) -> Self {
        match tier_id {
            0 => Self::T3(InvertedSketchIndex::new(IndexStorage::new())),
            1 => Self::T4(InvertedSketchIndex::new(IndexStorage::new())),
            2 => Self::T6(InvertedSketchIndex::new(IndexStorage::new())),
            _ => panic!("Unsupported tier_id: {}", tier_id),
        }
    }
    fn put(&self, block_id: &BlockID<H>, features: &[SuperFeature]) {
        match self {
            Self::T3(idx) => {
                let vals: [u32; 3] = features
                    .iter()
                    .map(SuperFeature::value)
                    .collect::<Vec<_>>()
                    .try_into()
                    .expect("tier 0 requires 3 super-features");
                idx.put(block_id, U32Sketch::new(vals).unwrap()).expect("index put failed");
            }
            Self::T4(idx) => {
                let vals: [u32; 4] = features
                    .iter()
                    .map(SuperFeature::value)
                    .collect::<Vec<_>>()
                    .try_into()
                    .expect("tier 1 requires 4 super-features");
                idx.put(block_id, U32Sketch::new(vals).unwrap()).expect("index put failed");
            }
            Self::T6(idx) => {
                let vals: [u32; 6] = features
                    .iter()
                    .map(SuperFeature::value)
                    .collect::<Vec<_>>()
                    .try_into()
                    .expect("tier 2 requires 6 super-features");
                idx.put(block_id, U32Sketch::new(vals).unwrap()).expect("index put failed");
            }
        }
    }
    fn get(&self, features: &[SuperFeature]) -> Option<BlockID<H>> {
        match self {
            Self::T3(idx) => {
                let vals: [u32; 3] =
                    features.iter().map(SuperFeature::value).collect::<Vec<_>>().try_into().ok()?;
                let query = U32Sketch::new(vals).ok()?;
                idx.get(&query).unwrap_or(None)
            }
            Self::T4(idx) => {
                let vals: [u32; 4] =
                    features.iter().map(SuperFeature::value).collect::<Vec<_>>().try_into().ok()?;
                let query = U32Sketch::new(vals).ok()?;
                idx.get(&query).unwrap_or(None)
            }
            Self::T6(idx) => {
                let vals: [u32; 6] =
                    features.iter().map(SuperFeature::value).collect::<Vec<_>>().try_into().ok()?;
                let query = U32Sketch::new(vals).ok()?;
                idx.get(&query).unwrap_or(None)
            }
        }
    }
    fn remove_key(&self, block_id: &BlockID<H>) {
        match self {
            Self::T3(idx) => {
                let _ = idx.remove(block_id);
            }
            Self::T4(idx) => {
                let _ = idx.remove(block_id);
            }
            Self::T6(idx) => {
                let _ = idx.remove(block_id);
            }
        }
    }
    fn remove_sf(&self, sf: &SuperFeature) {
        let value = sf.value();
        match self {
            Self::T3(idx) => {
                let keys: Vec<BlockID<H>> = idx.keys_with_feature(value).unwrap_or_default();
                for key in keys {
                    let _ = idx.remove(&key);
                }
            }
            Self::T4(idx) => {
                let keys: Vec<BlockID<H>> = idx.keys_with_feature(value).unwrap_or_default();
                for key in keys {
                    let _ = idx.remove(&key);
                }
            }
            Self::T6(idx) => {
                let keys: Vec<BlockID<H>> = idx.keys_with_feature(value).unwrap_or_default();
                for key in keys {
                    let _ = idx.remove(&key);
                }
            }
        }
    }
    fn len(&self) -> usize {
        match self {
            Self::T3(idx) => idx.len().unwrap_or(0),
            Self::T4(idx) => idx.len().unwrap_or(0),
            Self::T6(idx) => idx.len().unwrap_or(0),
        }
    }
    fn is_empty(&self) -> bool {
        match self {
            Self::T3(idx) => idx.is_empty().unwrap_or(true),
            Self::T4(idx) => idx.is_empty().unwrap_or(true),
            Self::T6(idx) => idx.is_empty().unwrap_or(true),
        }
    }
}

impl<H: ChunkHash> SFTable<H> {
    pub fn new(tier_id: u8, metric: u64) -> Self {
        Self { index: IndexBackend::new(tier_id), metric }
    }
    pub fn insert(&mut self, block_id: &BlockID<H>, features: &[SuperFeature]) {
        self.index.put(block_id, features);
    }
    pub fn remove_block(&mut self, block_id: &BlockID<H>) {
        self.index.remove_key(block_id);
    }
    pub fn remove_sf(&mut self, feature: &SuperFeature) {
        self.index.remove_sf(feature);
    }
    pub fn len(&self) -> usize {
        self.index.len()
    }
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
    // top_k(1) из индекса
    pub fn nearest(&self, features: &[SuperFeature]) -> Option<BlockID<H>> {
        self.index.get(features)
    }
    pub fn update_metric(&mut self, upd_fn: fn(u64) -> u64) {
        self.metric = upd_fn(self.metric)
    }
    pub fn get_metric(&self) -> u64 {
        self.metric
    }
}
