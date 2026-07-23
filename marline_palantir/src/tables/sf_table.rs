use crate::types::{BlockID, SuperFeature};
use std::collections::HashMap;

pub struct SFTable {
    index: HashMap<SuperFeature, Vec<BlockID>>,
    metric: u64,
}
impl SFTable {
    pub fn new(metric: u64) -> Self {
        Self { index: HashMap::new(), metric }
    }
    pub fn insert(&mut self, block_id: &BlockID, features: &[SuperFeature]) {
        for &super_feature in features {
            self.index.entry(super_feature).or_default().push(block_id.clone())
        }
    }
    pub fn lookup(&self, feature: &SuperFeature) -> Option<&Vec<BlockID>> {
        self.index.get(feature)
    }
    pub fn remove_block(&mut self, block_id: &BlockID) -> usize {
        let mut removed = 0;
        self.index.retain(|_, blocks| {
            let before = blocks.len();
            blocks.retain(|b| b != block_id);
            removed += before - blocks.len();
            !blocks.is_empty()
        });
        removed
    }
    pub fn remove_sf(&mut self, feature: &SuperFeature) {
        self.index.remove(feature);
    }
    pub fn len(&self) -> usize {
        self.index.len()
    }
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
    // надо сделать
    pub fn nearest(&self, _features: &[SuperFeature]) -> Option<BlockID> {
        None
    }
    pub fn update_metric(&mut self, upd_fn: fn(u64) -> u64) {
        self.metric = upd_fn(self.metric)
    }
    pub fn get_metric(&self) -> u64 {
        self.metric
    }
}
