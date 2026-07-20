use crate::{BlockId, Fingerprint};
use std::collections::HashMap;

/// Fingerprint table.
///
/// Used during the deduplication stage: if a new block's fingerprint
/// already exists in the table, the block is a duplicate,
/// and only a reference is stored.
pub struct FPTable {
    index: HashMap<Fingerprint, BlockId>,
}

impl FPTable {
    /// Creation of empty table
    pub fn new() -> Self {
        Self { index: HashMap::new() }
    }

    /// Checking for blocks with same fingerprint
    pub fn contains(&self, fingerprint: &Fingerprint) -> bool {
        self.index.contains_key(fingerprint)
    }
    /// Searching for blocks by fingerprint
    pub fn lookup(&self, fingerprint: &Fingerprint) -> Option<&BlockId> {
        self.index.get(fingerprint)
    }

    /// Adds a fingerprint associated with a `BlockId`.
    pub fn insert(&mut self, fingerprint: Fingerprint, block_id: BlockId) {
        self.index.insert(fingerprint, block_id);
    }

    /// Number of unique fingerprints in table
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Checking is a table empty
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
}
