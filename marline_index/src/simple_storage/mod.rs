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

impl<K, S: Sketch> SketchIndexApi<K, S> for LinearSearchIndex<K, S>
where
    K: Clone + Eq + Hash + Send + Sync,
    S: Sketch,
{
    type Error = IndexError;

    fn put(&self, key: &K, sketch: S) -> Result<(), Self::Error> {
        self.entries.write().unwrap().push((key.clone(), sketch));
        Ok(())
    }

    fn top_k(&self, query: &S, k: usize) -> Result<Vec<(K, f64)>, Self::Error> {
        let entries = self.entries.read().unwrap();
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
        self.entries.write().unwrap().retain(|(k, _)| k != key);
        Ok(())
    }

    fn clear(&self) -> Result<(), Self::Error> {
        self.entries.write().unwrap().clear();
        Ok(())
    }
}
