use embeddenator_retrieval::similarity::{
    compute_similarity, dot_product, hamming_distance, jaccard_similarity, SimilarityMetric,
};
use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};

#[test]
fn test_cosine_similarity_identical() {
    let config = ReversibleVSAConfig::default();
    let vec1 = SparseVec::encode_data(b"test data", &config, None);
    let vec2 = SparseVec::encode_data(b"test data", &config, None);

    let sim = compute_similarity(&vec1, &vec2, SimilarityMetric::Cosine);
    assert!(
        sim > 0.99,
        "Identical vectors should have ~1.0 cosine similarity, got {}",
        sim
    );
}

#[test]
fn test_cosine_similarity_different() {
    let config = ReversibleVSAConfig::default();
    let vec1 = SparseVec::encode_data(b"hello world", &config, None);
    let vec2 = SparseVec::encode_data(b"goodbye world", &config, None);

    let sim = compute_similarity(&vec1, &vec2, SimilarityMetric::Cosine);
    assert!(
        sim < 0.7,
        "Different vectors should have lower similarity, got {}",
        sim
    );
}

#[test]
fn test_cosine_similarity_orthogonal() {
    let config = ReversibleVSAConfig::default();
    let vec1 = SparseVec::encode_data(b"aaaaaaaa", &config, None);
    let vec2 = SparseVec::encode_data(b"zzzzzzzz", &config, None);

    let sim = compute_similarity(&vec1, &vec2, SimilarityMetric::Cosine);
    assert!(
        sim.abs() < 0.5,
        "Unrelated vectors should be roughly orthogonal, got {}",
        sim
    );
}

#[test]
fn test_hamming_distance_identical() {
    let config = ReversibleVSAConfig::default();
    let vec1 = SparseVec::encode_data(b"test", &config, None);
    let vec2 = SparseVec::encode_data(b"test", &config, None);

    let dist = hamming_distance(&vec1, &vec2);
    assert_eq!(
        dist, 0.0,
        "Identical vectors should have 0 Hamming distance"
    );
}

#[test]
fn test_hamming_distance_different() {
    let config = ReversibleVSAConfig::default();
    let vec1 = SparseVec::encode_data(b"hello", &config, None);
    let vec2 = SparseVec::encode_data(b"world", &config, None);

    let dist = hamming_distance(&vec1, &vec2);
    assert!(
        dist > 0.0,
        "Different vectors should have positive Hamming distance"
    );
}

#[test]
fn test_jaccard_similarity_identical() {
    let config = ReversibleVSAConfig::default();
    let vec1 = SparseVec::encode_data(b"test", &config, None);
    let vec2 = SparseVec::encode_data(b"test", &config, None);

    let sim = jaccard_similarity(&vec1, &vec2);
    assert!(
        (sim - 1.0).abs() < 0.01,
        "Identical vectors should have ~1.0 Jaccard similarity"
    );
}

#[test]
fn test_jaccard_similarity_different() {
    let config = ReversibleVSAConfig::default();
    let vec1 = SparseVec::encode_data(b"apple", &config, None);
    let vec2 = SparseVec::encode_data(b"zebra", &config, None);

    let sim = jaccard_similarity(&vec1, &vec2);
    assert!(
        sim < 0.7,
        "Different vectors should have lower Jaccard similarity"
    );
}

#[test]
fn test_dot_product_identical() {
    let config = ReversibleVSAConfig::default();
    let vec1 = SparseVec::encode_data(b"test", &config, None);
    let vec2 = SparseVec::encode_data(b"test", &config, None);

    let dot = dot_product(&vec1, &vec2);
    assert!(
        dot > 0,
        "Identical vectors should have positive dot product"
    );
}

#[test]
fn test_dot_product_orthogonal() {
    let config = ReversibleVSAConfig::default();
    let vec1 = SparseVec::encode_data(b"aaa", &config, None);
    let vec2 = SparseVec::encode_data(b"zzz", &config, None);

    let dot = dot_product(&vec1, &vec2);
    // Orthogonal vectors should have dot product near 0
    assert!(
        dot.abs() < 50,
        "Unrelated vectors should have near-zero dot product, got {}",
        dot
    );
}

#[test]
fn test_all_metrics_consistency() {
    // All metrics should agree that identical vectors are most similar
    let config = ReversibleVSAConfig::default();
    let vec = SparseVec::encode_data(b"test vector", &config, None);

    let cosine = compute_similarity(&vec, &vec, SimilarityMetric::Cosine);
    let hamming = compute_similarity(&vec, &vec, SimilarityMetric::Hamming);
    let jaccard = compute_similarity(&vec, &vec, SimilarityMetric::Jaccard);
    let dot = compute_similarity(&vec, &vec, SimilarityMetric::DotProduct);

    assert!(cosine > 0.99, "Cosine should be ~1.0 for identical vectors");
    assert_eq!(hamming, 0.0, "Hamming should be 0 for identical vectors");
    assert!(
        jaccard > 0.99,
        "Jaccard should be ~1.0 for identical vectors"
    );
    assert!(
        dot > 0.0,
        "Dot product should be positive for identical vectors"
    );
}

#[test]
fn test_similarity_range_bounds() {
    let config = ReversibleVSAConfig::default();
    let vec1 = SparseVec::encode_data(b"data1", &config, None);
    let vec2 = SparseVec::encode_data(b"data2", &config, None);

    // Cosine should be in [-1, 1]
    let cosine = compute_similarity(&vec1, &vec2, SimilarityMetric::Cosine);
    assert!(
        (-1.0..=1.0).contains(&cosine),
        "Cosine should be in [-1, 1], got {}",
        cosine
    );

    // Hamming should be non-negative
    let hamming = compute_similarity(&vec1, &vec2, SimilarityMetric::Hamming);
    assert!(
        hamming >= 0.0,
        "Hamming distance should be non-negative, got {}",
        hamming
    );

    // Jaccard should be in [0, 1]
    let jaccard = compute_similarity(&vec1, &vec2, SimilarityMetric::Jaccard);
    assert!(
        (0.0..=1.0).contains(&jaccard),
        "Jaccard should be in [0, 1], got {}",
        jaccard
    );
}

#[test]
fn test_similarity_symmetry() {
    let config = ReversibleVSAConfig::default();
    let vec1 = SparseVec::encode_data(b"first", &config, None);
    let vec2 = SparseVec::encode_data(b"second", &config, None);

    // All metrics should be symmetric
    let cosine1 = compute_similarity(&vec1, &vec2, SimilarityMetric::Cosine);
    let cosine2 = compute_similarity(&vec2, &vec1, SimilarityMetric::Cosine);
    assert!(
        (cosine1 - cosine2).abs() < 1e-10,
        "Cosine should be symmetric"
    );

    let hamming1 = hamming_distance(&vec1, &vec2);
    let hamming2 = hamming_distance(&vec2, &vec1);
    assert!(
        (hamming1 - hamming2).abs() < 1e-10,
        "Hamming should be symmetric"
    );

    let jaccard1 = jaccard_similarity(&vec1, &vec2);
    let jaccard2 = jaccard_similarity(&vec2, &vec1);
    assert!(
        (jaccard1 - jaccard2).abs() < 1e-10,
        "Jaccard should be symmetric"
    );
}

#[test]
fn test_similarity_with_empty_vectors() {
    let empty1 = SparseVec {
        pos: vec![],
        neg: vec![],
    };
    let empty2 = SparseVec {
        pos: vec![],
        neg: vec![],
    };

    // Two empty vectors should be considered identical
    let jaccard = jaccard_similarity(&empty1, &empty2);
    assert_eq!(
        jaccard, 1.0,
        "Empty vectors should have Jaccard similarity of 1.0"
    );

    let hamming = hamming_distance(&empty1, &empty2);
    assert_eq!(
        hamming, 0.0,
        "Empty vectors should have Hamming distance of 0"
    );

    let dot = dot_product(&empty1, &empty2);
    assert_eq!(dot, 0, "Empty vectors should have dot product of 0");
}
