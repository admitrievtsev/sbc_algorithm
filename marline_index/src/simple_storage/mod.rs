use std::{hash::Hash, sync::RwLock};

use crate::index::IndexError;
use crate::index::SketchIndexApi;
use crate::sketch::{SimilarityScore, Sketch};

pub struct LinearSearchIndex<K, S: Sketch> {
    entries: RwLock<Vec<(K, S)>>,
}

impl<K, S: Sketch> LinearSearchIndex<K, S> {
    pub fn new() -> Self {
        Self { entries: RwLock::new(Vec::new()) }
    }
}

impl<K, S: Sketch> Default for LinearSearchIndex<K, S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, S: Sketch> SketchIndexApi<K, S> for LinearSearchIndex<K, S>
where
    K: Clone + Eq + Hash + Send + Sync,
    S: Sketch,
{
    type Error = IndexError;

    fn put(&self, key: &K, sketch: S) -> Result<(), Self::Error> {
        let mut entries = self.entries.write().expect("rwlock poisoned");
        if let Some((_, existing_sketch)) =
            entries.iter_mut().find(|(entry_key, _)| entry_key == key)
        {
            *existing_sketch = sketch;
        } else {
            entries.push((key.clone(), sketch));
        }
        Ok(())
    }

    fn top_k(&self, query: &S, k: usize) -> Result<Vec<(K, f64)>, Self::Error> {
        let entries = self.entries.read().expect("rwlock poisoned");
        let mut scored: Vec<(K, f64)> = entries
            .iter()
            .map(|(key, sketch)| {
                let overlap = query.intersection_size(sketch);
                let score = SimilarityScore::new_from_two(overlap, query.len(), sketch.len());
                (key.clone(), score.jaccard())
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        Ok(scored)
    }

    fn get(&self, query: &S) -> Result<Option<K>, Self::Error> {
        Ok(self.top_k(query, 1)?.into_iter().next().map(|r| r.0))
    }

    fn remove(&self, key: &K) -> Result<(), Self::Error> {
        self.entries.write().expect("rwlock poisoned").retain(|(k, _)| k != key);
        Ok(())
    }

    fn clear(&self) -> Result<(), Self::Error> {
        self.entries.write().expect("rwlock poisoned").clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sketch::U32Sketch;

    fn mk(vals: [u32; 3]) -> U32Sketch<3> {
        U32Sketch::new(vals)
    }

    #[test]
    fn put_overwrites_existing_key() {
        let index = LinearSearchIndex::new();
        index.put(&1, mk([1, 2, 3])).unwrap();
        index.put(&1, mk([10, 11, 12])).unwrap();

        let results = index.top_k(&mk([10, 11, 12]), 10).unwrap();
        assert_eq!(results, vec![(1, 1.0)]);

        let old_query_results = index.top_k(&mk([1, 2, 3]), 10).unwrap();
        assert_eq!(old_query_results, vec![(1, 0.0)]);
    }

    #[test]
    fn remove_deletes_all_entries_for_key() {
        let index = LinearSearchIndex::new();
        index.put(&1, mk([1, 2, 3])).unwrap();
        index.remove(&1).unwrap();

        assert!(index.top_k(&mk([1, 2, 3]), 10).unwrap().is_empty());
    }
}
