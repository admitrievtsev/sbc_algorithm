//! Storage traits and in-memory backend for sketch indexes.

use crate::index::error::IndexError;
use crate::sketch::Sketch;

pub mod index_storage;
pub use index_storage::IndexStorage;

/// Storage for feature → posting-list records.
pub trait InvertedStorage<K, F>: Send + Sync
where
    K: Clone + Send + Sync,
    F: Copy + Send + Sync,
{
    /// Returns all keys that contain the feature.
    fn posting_list(&self, feature: F) -> Result<Vec<K>, IndexError>;

    /// Adds the key to the feature's posting list.
    fn insert_posting(&self, feature: F, key: K) -> Result<(), IndexError>;

    /// Removes the key from the feature's posting list.
    fn remove_posting(&self, feature: F, key: &K) -> Result<(), IndexError>;

    /// Returns the number of distinct indexed features.
    fn len_postings(&self) -> Result<usize, IndexError>;

    /// Removes all posting lists.
    fn clear_postings(&self) -> Result<(), IndexError>;
}

/// Complete storage backend required by [`crate::index::InvertedSketchIndex`].
pub trait Store<K, S>: InvertedStorage<K, S::Feature>
where
    K: Clone + Send + Sync,
    S: Sketch,
{
    /// Inserts an entry. Existing keys are not updated.
    fn insert_entry(&self, key: K, sketch: S) -> Result<(), IndexError>;

    /// Removes an entry from all posting lists using only its key.
    fn remove_entry(&self, key: &K) -> Result<(), IndexError>;

    /// Removes all entries from the storage.
    fn clear(&self) -> Result<(), IndexError> {
        self.clear_postings()
    }
}
