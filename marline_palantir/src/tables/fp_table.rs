use crate::types::{BlockID, Fingerprint};
use std::collections::HashMap;

pub struct FPTable {
    index: HashMap<Fingerprint, BlockID>,
}

impl FPTable {
    pub fn new() -> Self {
        Self { index: HashMap::new() }
    }
    pub fn contains(&self, fingerprint: &Fingerprint) -> bool {
        self.index.contains_key(fingerprint)
    }
    pub fn lookup(&self, fingerprint: &Fingerprint) -> Option<&BlockID> {
        self.index.get(fingerprint)
    }
    pub fn insert(&mut self, fingerprint: Fingerprint, block_id: BlockID) {
        self.index.insert(fingerprint, block_id);
    }
    pub fn remove(&mut self, fingerprint: &Fingerprint) -> Option<BlockID> {
        self.index.remove(fingerprint)
    }
    pub fn len(&self) -> usize {
        self.index.len()
    }
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
}
