use crate::types::{BlockID, SuperFeature};
use chunkfs::ChunkHash;
use marline_index::index::store::IndexStorage;
use marline_index::index::{IndexError, InvertedSketchIndex, Metric, MetricsApi, SketchIndexApi};
use marline_index::sketch::U32Sketch;

struct IndexBackend<H: ChunkHash + Send + Sync, const N: usize> {
    index: InvertedSketchIndex<BlockID<H>, U32Sketch<N>, IndexStorage<BlockID<H>, U32Sketch<N>>>,
}
pub struct SFTable<H: ChunkHash + Send + Sync, const N: usize> {
    index: IndexBackend<H, N>,
}

impl<H: ChunkHash + Send + Sync, const N: usize> IndexBackend<H, N> {
    pub fn new() -> Self {
        Self { index: InvertedSketchIndex::new(IndexStorage::new()) }
    }
    fn put(&self, block_id: &BlockID<H>, features: &[SuperFeature], metric: Metric) {
        let vals: [u32; N] = features
            .iter()
            .map(SuperFeature::value)
            .collect::<Vec<_>>()
            .try_into()
            .expect("features length must match table tier width N");
        self.index.put(block_id, U32Sketch::new(vals).unwrap()).expect("index put failed");
        self.index.set_metric(block_id, metric).unwrap();
    }
    fn get(&self, features: &[SuperFeature]) -> Option<BlockID<H>> {
        let vals: [u32; N] =
            features.iter().map(SuperFeature::value).collect::<Vec<_>>().try_into().ok()?;
        let query = U32Sketch::new(vals).ok()?;
        self.index.get(&query).unwrap_or(None)
    }
    fn remove_key(&self, block_id: &BlockID<H>) {
        let _ = self.index.remove(block_id);
    }
    fn remove_sf(&self, sf: &SuperFeature) {
        let value = sf.value();
        let keys: Vec<BlockID<H>> = self.index.keys_with_feature(value).unwrap_or_default();
        for key in keys {
            let _ = self.index.remove(&key);
        }
    }
    fn len(&self) -> usize {
        self.index.len().unwrap_or(0)
    }
    fn is_empty(&self) -> bool {
        self.index.is_empty().unwrap_or(true)
    }
    fn get_metric(&self, block_id: &BlockID<H>) -> Option<Metric> {
        self.index.get_metric(block_id).unwrap_or(None)
    }
    fn set_metric(&self, block_id: &BlockID<H>, value: Metric) {
        let _ = self.index.set_metric(block_id, value);
    }
    fn get_with_upd_metric(
        &self,
        features: &[SuperFeature],
        f: impl FnOnce(Metric) -> Metric,
    ) -> Option<BlockID<H>> {
        let vals: [u32; N] =
            features.iter().map(SuperFeature::value).collect::<Vec<_>>().try_into().ok()?;
        let query = U32Sketch::new(vals).ok()?;
        self.index.get_with_upd_metric(&query, f).unwrap_or(None)
    }
    fn update_and_clean(
        &self,
        update_fn: impl FnMut(Metric) -> Metric,
        cleanup_fn: impl Fn(Metric) -> bool,
    ) -> Result<(), IndexError> {
        self.index.update_and_clean(update_fn, cleanup_fn)
    }
}

impl<H: ChunkHash + Send + Sync, const N: usize> SFTable<H, N> {
    pub fn new() -> Self {
        Self { index: IndexBackend::new() }
    }
    pub fn insert(&mut self, block_id: &BlockID<H>, features: &[SuperFeature], metric: Metric) {
        self.index.put(block_id, features, metric);
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
    pub fn nearest(&self, features: &[SuperFeature]) -> Option<BlockID<H>> {
        self.index.get(features)
    }
    pub fn get_key_metric(&self, block_id: &BlockID<H>) -> Option<Metric> {
        self.index.get_metric(block_id)
    }
    pub fn set_key_metric(&self, block_id: &BlockID<H>, value: Metric) {
        self.index.set_metric(block_id, value);
    }
    pub fn get_with_upd_metric(
        &self,
        features: &[SuperFeature],
        f: impl FnOnce(Metric) -> Metric,
    ) -> Option<BlockID<H>> {
        self.index.get_with_upd_metric(features, f)
    }
    pub fn update_and_clean(
        &self,
        update_fn: impl FnMut(Metric) -> Metric,
        cleanup_fn: impl Fn(Metric) -> bool,
    ) -> Result<(), IndexError> {
        self.index.update_and_clean(update_fn, cleanup_fn)
    }
}
