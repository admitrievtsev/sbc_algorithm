use crate::types::{BlockID, SuperFeature};
use chunkfs::ChunkHash;
use marline_index::index::metrics::{Metric, MetricStorage, MetricsMap};
use marline_index::index::store::{IndexStorage, InvertedStorage};
use marline_index::index::IndexError;
use marline_index::index::{InvertedSketchIndex, SketchIndexApi};
use marline_index::sketch::U32Sketch;
use std::hash::Hash;
use std::sync::Arc;

struct SharedStore<K, F>(Arc<IndexStorage<K, F>>);

impl<K, F> InvertedStorage<K, F> for SharedStore<K, F>
where
    K: Clone + Eq + Hash + Send + Sync,
    F: Copy + Eq + Hash + Send + Sync,
{
    fn posting_list(&self, feature: F) -> Result<Vec<K>, IndexError> {
        self.0.posting_list(feature)
    }
    fn insert_posting(&self, feature: F, key: K) -> Result<(), IndexError> {
        self.0.insert_posting(feature, key)
    }
    fn remove_posting(&self, feature: F, key: &K) -> Result<(), IndexError> {
        self.0.remove_posting(feature, key)
    }
    fn len_postings(&self) -> Result<usize, IndexError> {
        self.0.len_postings()
    }
    fn clear_postings(&self) -> Result<(), IndexError> {
        self.0.clear_postings()
    }
}

struct IndexBackend<H: ChunkHash + Send + Sync, const N: usize> {
    index: InvertedSketchIndex<BlockID<H>, U32Sketch<N>, SharedStore<BlockID<H>, u32>>,
    store: SharedStore<BlockID<H>, u32>,
    metrics: MetricsMap<BlockID<H>>,
}
pub struct SFTable<H: ChunkHash + Send + Sync, const N: usize> {
    index: IndexBackend<H, N>,
}

impl<H: ChunkHash + Send + Sync, const N: usize> IndexBackend<H, N> {
    pub fn new() -> Self {
        let store = SharedStore(Arc::new(IndexStorage::new()));
        Self {
            index: InvertedSketchIndex::new(SharedStore(Arc::clone(&store.0))),
            store,
            metrics: MetricsMap::new(),
        }
    }
    fn put(&self, block_id: &BlockID<H>, features: &[SuperFeature], metric: Metric) {
        let vals: [u32; N] = features
            .iter()
            .map(SuperFeature::value)
            .collect::<Vec<_>>()
            .try_into()
            .expect("features length must match table tier width N");
        self.index.put(block_id, U32Sketch::new(vals).unwrap()).expect("index put failed");
        self.metrics.set_metric(block_id, metric);
    }
    fn get(&self, features: &[SuperFeature]) -> Option<BlockID<H>> {
        let vals: [u32; N] =
            features.iter().map(SuperFeature::value).collect::<Vec<_>>().try_into().ok()?;
        let query = U32Sketch::new(vals).ok()?;
        self.index.get(&query).unwrap_or(None)
    }
    fn remove_key(&self, block_id: &BlockID<H>) {
        let _ = self.index.remove(block_id);
        self.metrics.remove_metric(block_id);
    }
    fn remove_sf(&self, sf: &SuperFeature) {
        let value = sf.value();
        let keys: Vec<BlockID<H>> = self.store.posting_list(value).unwrap_or_default();
        for key in keys {
            let _ = self.index.remove(&key);
            self.metrics.remove_metric(&key);
        }
    }
    fn len(&self) -> usize {
        self.store.len_postings().unwrap_or(0)
    }
    fn is_empty(&self) -> bool {
        self.store.len_postings().map_or(true, |n| n == 0)
    }
    fn get_metric(&self, block_id: &BlockID<H>) -> Option<Metric> {
        self.metrics.get_metric(block_id)
    }
    fn set_metric(&self, block_id: &BlockID<H>, value: Metric) {
        self.metrics.set_metric(block_id, value);
    }
    fn get_with_upd_metric(
        &self,
        features: &[SuperFeature],
        f: impl FnOnce(Metric) -> Metric,
    ) -> Option<BlockID<H>> {
        let vals: [u32; N] =
            features.iter().map(SuperFeature::value).collect::<Vec<_>>().try_into().ok()?;
        let query = U32Sketch::new(vals).ok()?;
        let result = self.index.get(&query).unwrap_or(None)?;
        let old = self.metrics.get_metric(&result).unwrap_or(0);
        self.metrics.set_metric(&result, f(old));
        Some(result)
    }
    fn update_and_clean(
        &self,
        mut update_fn: impl FnMut(Metric) -> Metric,
        cleanup_fn: impl Fn(Metric) -> bool,
    ) -> Result<(), IndexError> {
        self.metrics.update_and_clean(&mut |m| *m = update_fn(*m), &|m| cleanup_fn(m));
        Ok(())
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
