/// The similarity between two sketches.
///
/// Stores intersection and union sizes. The Jaccard similarity can be
/// derived from these values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimilarityScore {
    /// The size of the sketch intersection.
    intersection: usize,
    /// The size of the sketch union.
    union: usize,
}

impl SimilarityScore {
    /// Computes the Jaccard similarity coefficient.
    pub fn jaccard(self) -> f64 {
        self.intersection as f64 / self.union as f64
    }

    /// Computes the Jaccard similarity coefficient.
    pub fn jaccard_similarity(&self) -> f64 {
        (*self).jaccard()
    }

    /// Returns the size of the sketch intersection.
    pub fn intersection(self) -> usize {
        self.intersection
    }

    /// Returns the size of the sketch union.
    pub fn union(self) -> usize {
        self.union
    }

    /// Creates a score for two sketches of equal `size`.
    ///
    /// The union is computed as `size * 2 - intersection`.
    ///
    /// # Panics
    ///
    /// Panics if `size == 0`, if `intersection > size`, or if the union size
    /// overflows `usize`.
    pub fn new(intersection: usize, size: usize) -> Self {
        assert!(size > 0, "SimilarityScore union must be positive");
        assert!(intersection <= size, "intersection must be <= sketch size");

        let union = size
            .checked_mul(2)
            .and_then(|total| total.checked_sub(intersection))
            .expect("SimilarityScore union overflow");
        assert!(intersection <= union, "intersection must be <= union");

        SimilarityScore { intersection, union }
    }

    /// Creates a score for two sketches of potentially different sizes.
    ///
    /// The union is computed as `len_a + len_b - intersection`.
    ///
    /// # Panics
    ///
    /// Panics if `intersection > min(len_a, len_b)`, if the computed union is
    /// zero, or if the union size overflows `usize`.
    pub fn new_from_two(intersection: usize, len_a: usize, len_b: usize) -> Self {
        assert!(intersection <= len_a.min(len_b), "intersection must be <= min(len_a, len_b)");

        let union = len_a
            .checked_add(len_b)
            .and_then(|total| total.checked_sub(intersection))
            .expect("SimilarityScore union overflow");
        assert!(union > 0, "SimilarityScore union must be positive");
        assert!(intersection <= union, "intersection must be <= union");

        SimilarityScore { intersection, union }
    }
}

#[cfg(test)]
mod tests {
    use crate::sketch::SimilarityScore;

    #[test]
    fn new_computes_union() {
        let score = SimilarityScore::new(26, 52);
        assert_eq!(score.union(), 78);
    }

    #[test]
    fn new_full_overlap() {
        let score = SimilarityScore::new(100, 100);
        assert_eq!(score.intersection(), 100);
        assert_eq!(score.union(), 100);
    }

    #[test]
    fn new_zero_intersection() {
        let score = SimilarityScore::new(0, 50);
        assert_eq!(score.intersection(), 0);
        assert_eq!(score.union(), 100);
    }

    #[test]
    fn jaccard_similarity_half_overlap() {
        let score = SimilarityScore::new_from_two(5, 10, 5);
        assert!((score.jaccard_similarity() - 0.5).abs() < f64::EPSILON);
        assert!((score.jaccard() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_similarity_identical() {
        let score = SimilarityScore::new(10, 10);
        assert!((score.jaccard_similarity() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_similarity_no_overlap() {
        let score = SimilarityScore::new(0, 5);
        assert!((score.jaccard_similarity() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    #[should_panic(expected = "SimilarityScore union must be positive")]
    fn new_rejects_zero_union() {
        SimilarityScore::new(0, 0);
    }

    #[test]
    #[should_panic(expected = "intersection must be <= sketch size")]
    fn new_rejects_intersection_larger_than_size() {
        SimilarityScore::new(6, 5);
    }

    #[test]
    fn new_from_two_different_sizes() {
        let score = SimilarityScore::new_from_two(3, 4, 5);
        // union = 4 + 5 - 3 = 6
        assert_eq!(score.union(), 6);
        assert_eq!(score.intersection(), 3);
    }

    #[test]
    fn new_from_two_same_sizes() {
        let from_two = SimilarityScore::new_from_two(5, 10, 10);
        let from_one = SimilarityScore::new(5, 10);
        assert_eq!(from_two.union(), from_one.union());
        assert_eq!(from_two.intersection(), from_one.intersection());
    }

    #[test]
    #[should_panic(expected = "intersection must be <= min(len_a, len_b)")]
    fn new_from_two_rejects_impossible_intersection() {
        SimilarityScore::new_from_two(4, 3, 5);
    }
}
