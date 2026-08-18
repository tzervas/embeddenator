//! Similarity metrics for VSA-based retrieval
//!
//! This module provides various similarity metrics for comparing sparse ternary vectors,
//! including cosine similarity, Hamming distance, and specialized VSA metrics.

use embeddenator_vsa::SparseVec;

/// Similarity metric enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SimilarityMetric {
    /// Cosine similarity (default, best for VSA)
    #[default]
    Cosine,
    /// Hamming distance for ternary vectors
    Hamming,
    /// Jaccard similarity (intersection over union)
    Jaccard,
    /// Dot product (unnormalized)
    DotProduct,
}

/// Compute similarity between two vectors using the specified metric
///
/// # Arguments
/// * `a` - First vector
/// * `b` - Second vector
/// * `metric` - Similarity metric to use
///
/// # Returns
/// Similarity score. For Cosine and Jaccard: [0, 1] (or [-1, 1] for Cosine).
/// For Hamming: distance (lower is more similar).
/// For DotProduct: unnormalized score.
///
/// # Examples
///
/// ```
/// use embeddenator_retrieval::similarity::{compute_similarity, SimilarityMetric};
/// use embeddenator_vsa::SparseVec;
///
/// let a = SparseVec::from_data(b"hello");
/// let b = SparseVec::from_data(b"hello");
/// let sim = compute_similarity(&a, &b, SimilarityMetric::Cosine);
/// assert!(sim > 0.9);
/// ```
pub fn compute_similarity(a: &SparseVec, b: &SparseVec, metric: SimilarityMetric) -> f64 {
    match metric {
        SimilarityMetric::Cosine => a.cosine(b),
        SimilarityMetric::Hamming => hamming_distance(a, b),
        SimilarityMetric::Jaccard => jaccard_similarity(a, b),
        SimilarityMetric::DotProduct => dot_product(a, b) as f64,
    }
}

/// Compute Hamming distance between two sparse ternary vectors
///
/// For sparse ternary vectors, we compute the number of dimensions where
/// the vectors differ. This is efficient for sparse representations.
///
/// # Arguments
/// * `a` - First vector
/// * `b` - Second vector
///
/// # Returns
/// Hamming distance (number of differing positions)
///
/// # Examples
///
/// ```
/// use embeddenator_retrieval::similarity::hamming_distance;
/// use embeddenator_vsa::SparseVec;
///
/// let a = SparseVec::from_data(b"test");
/// let b = SparseVec::from_data(b"test");
/// let dist = hamming_distance(&a, &b);
/// assert!(dist < 100.0); // Should be very low for identical inputs
/// ```
pub fn hamming_distance(a: &SparseVec, b: &SparseVec) -> f64 {
    // For sparse ternary vectors, count positions where values differ.
    // Each dimension d has value: +1 if d in pos, -1 if d in neg, else 0.
    //
    // A dimension differs if it has different values in a and b:
    // - a=+1 and b is not +1 (b is 0 or -1)
    // - a=-1 and b is not -1 (b is 0 or +1)
    // - b=+1 and a is not +1 (a is 0 or -1)
    // - b=-1 and a is not -1 (a is 0 or +1)
    //
    // We use a HashSet to collect unique differing dimensions, avoiding
    // double-counting when both sides detect the same differing dimension.

    use std::collections::HashSet;
    let mut differing: HashSet<usize> = HashSet::new();

    // Dimensions where a has +1 but b doesn't (b is 0 or -1)
    for &dim in &a.pos {
        if !b.pos.contains(&dim) {
            differing.insert(dim);
        }
    }

    // Dimensions where a has -1 but b doesn't (b is 0 or +1)
    for &dim in &a.neg {
        if !b.neg.contains(&dim) {
            differing.insert(dim);
        }
    }

    // Dimensions where b has +1 but a doesn't (a is 0 or -1)
    for &dim in &b.pos {
        if !a.pos.contains(&dim) {
            differing.insert(dim);
        }
    }

    // Dimensions where b has -1 but a doesn't (a is 0 or +1)
    for &dim in &b.neg {
        if !a.neg.contains(&dim) {
            differing.insert(dim);
        }
    }

    differing.len() as f64
}

/// Compute Jaccard similarity between two sparse ternary vectors
///
/// Jaccard similarity = |intersection| / |union|
/// For ternary vectors, we consider both positive and negative sets.
///
/// # Arguments
/// * `a` - First vector
/// * `b` - Second vector
///
/// # Returns
/// Jaccard similarity in [0, 1]
///
/// # Examples
///
/// ```
/// use embeddenator_retrieval::similarity::jaccard_similarity;
/// use embeddenator_vsa::SparseVec;
///
/// let a = SparseVec::from_data(b"apple");
/// let b = SparseVec::from_data(b"apple");
/// let sim = jaccard_similarity(&a, &b);
/// assert!(sim > 0.9);
/// ```
pub fn jaccard_similarity(a: &SparseVec, b: &SparseVec) -> f64 {
    // Intersection: dimensions that are in both a and b with same sign
    let mut intersection = 0usize;

    for &dim in &a.pos {
        if b.pos.contains(&dim) {
            intersection += 1;
        }
    }

    for &dim in &a.neg {
        if b.neg.contains(&dim) {
            intersection += 1;
        }
    }

    // Union: all dimensions in either a or b
    let union = a.pos.len() + a.neg.len() + b.pos.len() + b.neg.len() - intersection;

    if union == 0 {
        return 1.0; // Both empty
    }

    intersection as f64 / union as f64
}

/// Compute sparse dot product (unnormalized)
///
/// For sparse ternary vectors: sum of +1 for matching positive dims,
/// -1 for opposite signs, and 0 for non-overlapping.
///
/// # Arguments
/// * `a` - First vector
/// * `b` - Second vector
///
/// # Returns
/// Dot product (integer cast to f64 for consistency)
///
/// # Examples
///
/// ```
/// use embeddenator_retrieval::similarity::dot_product;
/// use embeddenator_vsa::SparseVec;
///
/// let a = SparseVec::from_data(b"data");
/// let b = SparseVec::from_data(b"data");
/// let dot = dot_product(&a, &b);
/// assert!(dot > 0);
/// ```
pub fn dot_product(a: &SparseVec, b: &SparseVec) -> i32 {
    let mut score = 0i32;

    // +1 for each matching positive dimension
    for &dim in &a.pos {
        if b.pos.contains(&dim) {
            score += 1;
        } else if b.neg.contains(&dim) {
            score -= 1;
        }
    }

    // +1 for each matching negative dimension
    for &dim in &a.neg {
        if b.neg.contains(&dim) {
            score += 1;
        } else if b.pos.contains(&dim) {
            score -= 1;
        }
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use embeddenator_vsa::ReversibleVSAConfig;

    fn make_vec(data: &[u8]) -> SparseVec {
        let config = ReversibleVSAConfig::default();
        SparseVec::encode_data(data, &config, None)
    }

    #[test]
    fn test_cosine_identical() {
        let a = make_vec(b"test");
        let b = make_vec(b"test");
        let sim = compute_similarity(&a, &b, SimilarityMetric::Cosine);
        assert!(
            sim > 0.99,
            "Identical vectors should have ~1.0 cosine similarity"
        );
    }

    #[test]
    fn test_cosine_different() {
        let a = make_vec(b"hello");
        let b = make_vec(b"world");
        let sim = compute_similarity(&a, &b, SimilarityMetric::Cosine);
        assert!(sim < 0.5, "Different vectors should have low similarity");
    }

    #[test]
    fn test_hamming_identical() {
        let a = make_vec(b"test");
        let b = make_vec(b"test");
        let dist = hamming_distance(&a, &b);
        assert_eq!(
            dist, 0.0,
            "Identical vectors should have 0 Hamming distance"
        );
    }

    #[test]
    fn test_hamming_different() {
        let a = make_vec(b"hello");
        let b = make_vec(b"world");
        let dist = hamming_distance(&a, &b);
        assert!(
            dist > 0.0,
            "Different vectors should have positive Hamming distance"
        );
    }

    #[test]
    fn test_jaccard_identical() {
        let a = make_vec(b"test");
        let b = make_vec(b"test");
        let sim = jaccard_similarity(&a, &b);
        assert!(
            (sim - 1.0).abs() < 0.01,
            "Identical vectors should have ~1.0 Jaccard similarity"
        );
    }

    #[test]
    fn test_jaccard_disjoint() {
        let a = make_vec(b"aaa");
        let b = make_vec(b"zzz");
        let sim = jaccard_similarity(&a, &b);
        assert!(
            sim < 0.5,
            "Different vectors should have low Jaccard similarity"
        );
    }

    #[test]
    fn test_dot_product_identical() {
        let a = make_vec(b"test");
        let b = make_vec(b"test");
        let dot = dot_product(&a, &b);
        assert!(
            dot > 0,
            "Identical vectors should have positive dot product"
        );
    }
}
