use crate::{BlockId, SuperFeature};
use std::collections::HashMap;

/// Super-feature table for a single tier of a backup version.
///
/// Implements an inverted index: for each `SuperFeature`, it stores a
/// list of `BlockId`s for which this super-feature was computed.
pub struct SFTable {
    index: HashMap<SuperFeature, Vec<BlockId>>,
}

impl SFTable {
    /// Creates an empty table
    pub fn new() -> Self {
        Self { index: HashMap::new() }
    }

    /// Adds a block using a set of super-features.
    /// Each `SuperFeature` in `features` will be added to the index
    /// with a reference to `block_id`.
    pub fn insert(&mut self, block_id: &BlockId, features: &[SuperFeature]) {
        for &sf in features {
            self.index.entry(sf).or_default().push(block_id.clone())
        }
    }

    /// Searching blocks by super-feature
    pub fn lookup(&self, feature: &SuperFeature) -> Option<&Vec<BlockId>> {
        self.index.get(feature)
    }

    /// Removes all entries referencing this block.
    /// Returns the number of removed entries.
    pub fn remove_block(&mut self, block_id: &BlockId) -> usize {
        let mut removed = 0;
        self.index.retain(|_, blocks| {
            let before = blocks.len();
            blocks.retain(|b| b != block_id);
            removed += before - blocks.len();
            !blocks.is_empty()
        });
        removed
    }

    /// Number of unique super-features in table.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Checking is a table empty
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
}
