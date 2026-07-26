use std::error::Error;
use std::hash::Hash;

use crate::index::Metric;

/// Per-key metric access for sketch indexes.
///
/// Metrics are stored alongside sketches for the same keys.
pub trait MetricsApi<K>: Send + Sync
where
    K: Clone + Eq + Hash + Send + Sync,
{
    /// The error type returned by metric operations.
    type Error: Error + Send + Sync + 'static;

    /// Returns the metric for the given key, or `None` if the key is absent.
    fn get_metric(&self, key: &K) -> Result<Option<Metric>, Self::Error>;

    /// Sets the metric for the given key.
    ///
    /// The key must already exist in the index. Returns the previous metric
    /// value, or `None` if this is the first time the metric is set.
    fn set_metric(&self, key: &K, value: Metric) -> Result<Option<Metric>, Self::Error>;
}
