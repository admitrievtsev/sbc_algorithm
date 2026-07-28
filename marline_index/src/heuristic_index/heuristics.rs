//! Search heuristics for inverted-index candidate generation and ranking.

/// Configuration for heuristic-guided similarity search.
///
/// Default values preserve the original exhaustive overlap-count behaviour.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchConfig {
    /// Drop candidates whose Jaccard score cannot reach this threshold.
    ///
    /// `None` disables threshold pruning.
    pub min_jaccard: Option<f64>,

    /// Skip query features whose posting-list length exceeds this value.
    ///
    /// `None` probes every query feature.
    pub max_df: Option<usize>,

    /// Probe query features in ascending document-frequency order.
    pub rare_first: bool,

    /// Skip keys whose false-positive metric is greater than or equal to this value.
    ///
    /// `None` disables FP filtering. Requires a metrics map attached to the index.
    pub fp_metric_threshold: Option<u8>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self { min_jaccard: None, max_df: None, rare_first: false, fp_metric_threshold: None }
    }
}

impl SearchConfig {
    /// Creates a config with all heuristics disabled (exhaustive search).
    pub fn new() -> Self {
        Self::default()
    }
}

/// Minimum overlap between two equal-size sketches of length `n` required to
/// achieve Jaccard similarity ≥ `t`.
///
/// For equal sizes, `J = o / (2n - o) ≥ t` ⟹ `o ≥ ceil(t · 2n / (1 + t))`.
///
/// Returns `0` when `t ≤ 0`, and `n + 1` (unreachable) when `t > 1`.
pub fn min_overlap_for_jaccard(t: f64, n: usize) -> usize {
    if n == 0 || t <= 0.0 {
        return 0;
    }
    if t > 1.0 {
        return n + 1;
    }
    if (t - 1.0).abs() < f64::EPSILON {
        return n;
    }

    let needed = (t * 2.0 * n as f64) / (1.0 + t);
    needed.ceil() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_overlap_half_jaccard_size_six() {
        assert_eq!(min_overlap_for_jaccard(0.5, 6), 4);
    }

    #[test]
    fn min_overlap_perfect_match() {
        assert_eq!(min_overlap_for_jaccard(1.0, 6), 6);
    }

    #[test]
    fn min_overlap_zero_threshold() {
        assert_eq!(min_overlap_for_jaccard(0.0, 6), 0);
    }

    #[test]
    fn min_overlap_above_one_unreachable() {
        assert_eq!(min_overlap_for_jaccard(1.1, 6), 7);
    }

    #[test]
    fn search_config_default_disables_heuristics() {
        let cfg = SearchConfig::default();
        assert!(cfg.min_jaccard.is_none());
        assert!(cfg.max_df.is_none());
        assert!(!cfg.rare_first);
        assert!(cfg.fp_metric_threshold.is_none());
    }
}
