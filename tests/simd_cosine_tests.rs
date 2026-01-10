//! Tests for SIMD-accelerated cosine similarity
//!
//! Verifies that SIMD implementations produce identical results to scalar
//! implementation and maintains all expected mathematical properties.

use embeddenator::{ReversibleVSAConfig, SparseVec};

#[cfg(feature = "simd")]
use embeddenator::simd_cosine::{cosine_simd, cosine_scalar};

#[test]
fn test_cosine_scalar_basic() {
    let a = SparseVec::from_data(b"hello world");
    let b = SparseVec::from_data(b"hello world");
    let c = SparseVec::from_data(b"goodbye world");

    // Identical vectors should have high similarity
    let sim_same = a.cosine_scalar(&b);
    assert!(
        sim_same > 0.9,
        "Expected high similarity for identical vectors, got {}",
        sim_same
    );

    // Different vectors should have lower similarity
    let sim_diff = a.cosine_scalar(&c);
    assert!(
        sim_diff < sim_same,
        "Different vectors should have lower similarity: {} vs {}",
        sim_diff,
        sim_same
    );
}

#[test]
fn test_cosine_properties() {
    let a = SparseVec::from_data(b"alpha");
    let b = SparseVec::from_data(b"beta");
    let c = SparseVec::from_data(b"gamma");

    // Symmetry: cosine(a, b) == cosine(b, a)
    let sim_ab = a.cosine_scalar(&b);
    let sim_ba = b.cosine_scalar(&a);
    assert!(
        (sim_ab - sim_ba).abs() < 1e-10,
        "Cosine should be symmetric: {} vs {}",
        sim_ab,
        sim_ba
    );

    // Self-similarity: cosine(a, a) should be 1.0
    let sim_aa = a.cosine_scalar(&a);
    assert!(
        (sim_aa - 1.0).abs() < 0.01,
        "Self-similarity should be 1.0, got {}",
        sim_aa
    );

    // Range: cosine should be in [-1, 1]
    for vec1 in [&a, &b, &c] {
        for vec2 in [&a, &b, &c] {
            let sim = vec1.cosine_scalar(vec2);
            assert!(
                sim >= -1.0 && sim <= 1.0,
                "Cosine should be in [-1, 1], got {}",
                sim
            );
        }
    }
}

#[test]
fn test_empty_vectors() {
    let empty = SparseVec {
        pos: vec![],
        neg: vec![],
    };
    let non_empty = SparseVec::from_data(b"test");

    // Empty vectors should have zero similarity with anything
    assert_eq!(empty.cosine_scalar(&empty), 0.0);
    assert_eq!(empty.cosine_scalar(&non_empty), 0.0);
    assert_eq!(non_empty.cosine_scalar(&empty), 0.0);
}

#[test]
#[cfg(feature = "simd")]
fn test_simd_matches_scalar_basic() {
    let test_cases = vec![
        (b"hello".as_slice(), b"hello".as_slice()),
        (b"hello".as_slice(), b"world".as_slice()),
        (b"test data".as_slice(), b"test data".as_slice()),
        (b"short".as_slice(), b"longer test string".as_slice()),
        (b"".as_slice(), b"non-empty".as_slice()),
    ];

    for (data_a, data_b) in test_cases {
        let a = SparseVec::from_data(data_a);
        let b = SparseVec::from_data(data_b);

        let scalar_result = cosine_scalar(&a, &b);
        let simd_result = cosine_simd(&a, &b);

        let diff = (scalar_result - simd_result).abs();
        assert!(
            diff < 1e-10,
            "SIMD result {} differs from scalar {} by {} (inputs: {:?}, {:?})",
            simd_result,
            scalar_result,
            diff,
            data_a,
            data_b
        );
    }
}

#[test]
#[cfg(feature = "simd")]
fn test_simd_matches_scalar_encoded() {
    let config = ReversibleVSAConfig::default();

    let test_cases = vec![
        (b"test data 1".as_slice(), b"test data 1".as_slice()),
        (b"test data 1".as_slice(), b"test data 2".as_slice()),
        (b"alpha beta gamma".as_slice(), b"delta epsilon zeta".as_slice()),
        (b"the quick brown fox".as_slice(), b"the lazy dog".as_slice()),
    ];

    for (data_a, data_b) in test_cases {
        let a = SparseVec::encode_data(data_a, &config, None);
        let b = SparseVec::encode_data(data_b, &config, None);

        let scalar_result = cosine_scalar(&a, &b);
        let simd_result = cosine_simd(&a, &b);

        let diff = (scalar_result - simd_result).abs();
        assert!(
            diff < 1e-10,
            "SIMD result {} differs from scalar {} by {} (inputs: {:?}, {:?})",
            simd_result,
            scalar_result,
            diff,
            data_a,
            data_b
        );
    }
}

#[test]
#[cfg(feature = "simd")]
fn test_simd_synthetic_vectors() {
    // Test with synthetic vectors of various sparsities
    let sparsity_levels = vec![10, 50, 100, 200, 500];

    for sparsity in sparsity_levels {
        // Create vectors with partial overlap
        let pos_a: Vec<usize> = (0..sparsity).map(|i| i * 3).collect();
        let neg_a: Vec<usize> = (0..sparsity).map(|i| i * 3 + 1).collect();

        let pos_b: Vec<usize> = (sparsity / 2..sparsity + sparsity / 2)
            .map(|i| i * 3)
            .collect();
        let neg_b: Vec<usize> = (sparsity / 2..sparsity + sparsity / 2)
            .map(|i| i * 3 + 1)
            .collect();

        let a = SparseVec {
            pos: pos_a,
            neg: neg_a,
        };
        let b = SparseVec {
            pos: pos_b,
            neg: neg_b,
        };

        let scalar_result = cosine_scalar(&a, &b);
        let simd_result = cosine_simd(&a, &b);

        let diff = (scalar_result - simd_result).abs();
        assert!(
            diff < 1e-10,
            "SIMD result {} differs from scalar {} by {} (sparsity: {})",
            simd_result,
            scalar_result,
            diff,
            sparsity
        );
    }
}

#[test]
#[cfg(feature = "simd")]
fn test_simd_empty_vectors() {
    let empty = SparseVec {
        pos: vec![],
        neg: vec![],
    };
    let non_empty = SparseVec::from_data(b"test");

    assert_eq!(cosine_simd(&empty, &empty), 0.0);
    assert_eq!(cosine_simd(&empty, &non_empty), 0.0);
    assert_eq!(cosine_simd(&non_empty, &empty), 0.0);

    // Verify matches scalar
    assert_eq!(
        cosine_simd(&empty, &empty),
        cosine_scalar(&empty, &empty)
    );
    assert_eq!(
        cosine_simd(&empty, &non_empty),
        cosine_scalar(&empty, &non_empty)
    );
}

#[test]
#[cfg(feature = "simd")]
fn test_simd_self_similarity() {
    let config = ReversibleVSAConfig::default();
    let test_data = vec![
        b"short".as_slice(),
        b"medium length string".as_slice(),
        b"a much longer string with lots of content to encode".as_slice(),
    ];

    for data in test_data {
        let vec = SparseVec::encode_data(data, &config, None);

        let scalar_result = cosine_scalar(&vec, &vec);
        let simd_result = cosine_simd(&vec, &vec);

        // Self-similarity should be 1.0 or very close
        assert!(
            scalar_result > 0.99,
            "Scalar self-similarity should be ~1.0, got {}",
            scalar_result
        );
        assert!(
            simd_result > 0.99,
            "SIMD self-similarity should be ~1.0, got {}",
            simd_result
        );

        let diff = (scalar_result - simd_result).abs();
        assert!(
            diff < 1e-10,
            "SIMD self-similarity {} differs from scalar {} by {}",
            simd_result,
            scalar_result,
            diff
        );
    }
}

#[test]
#[cfg(feature = "simd")]
fn test_simd_symmetry() {
    let config = ReversibleVSAConfig::default();
    let a = SparseVec::encode_data(b"first vector", &config, None);
    let b = SparseVec::encode_data(b"second vector", &config, None);

    let simd_ab = cosine_simd(&a, &b);
    let simd_ba = cosine_simd(&b, &a);

    assert!(
        (simd_ab - simd_ba).abs() < 1e-10,
        "SIMD cosine should be symmetric: {} vs {}",
        simd_ab,
        simd_ba
    );

    // Also verify both match scalar
    let scalar_ab = cosine_scalar(&a, &b);
    let scalar_ba = cosine_scalar(&b, &a);

    assert!((simd_ab - scalar_ab).abs() < 1e-10);
    assert!((simd_ba - scalar_ba).abs() < 1e-10);
}

#[test]
#[cfg(feature = "simd")]
fn test_simd_range() {
    let config = ReversibleVSAConfig::default();
    let test_cases = vec![
        (b"alpha".as_slice(), b"alpha".as_slice()),
        (b"alpha".as_slice(), b"beta".as_slice()),
        (b"alpha".as_slice(), b"completely different".as_slice()),
    ];

    for (data_a, data_b) in test_cases {
        let a = SparseVec::encode_data(data_a, &config, None);
        let b = SparseVec::encode_data(data_b, &config, None);

        let sim = cosine_simd(&a, &b);
        assert!(
            sim >= -1.0 && sim <= 1.0,
            "SIMD cosine should be in [-1, 1], got {} for {:?} vs {:?}",
            sim,
            data_a,
            data_b
        );
    }
}

#[test]
#[cfg(all(feature = "simd", feature = "proptest"))]
fn test_simd_property_based() {
    use proptest::prelude::*;

    proptest!(|(data_a: Vec<u8>, data_b: Vec<u8>)| {
        let a = SparseVec::from_data(&data_a);
        let b = SparseVec::from_data(&data_b);

        let scalar = cosine_scalar(&a, &b);
        let simd = cosine_simd(&a, &b);

        // Results should match within floating point tolerance
        prop_assert!((scalar - simd).abs() < 1e-9);
        
        // Should be in valid range
        prop_assert!(scalar >= -1.0 && scalar <= 1.0);
        prop_assert!(simd >= -1.0 && simd <= 1.0);
    });
}

#[test]
fn test_cosine_with_feature_gate() {
    // This test verifies that the cosine() method works correctly
    // regardless of whether SIMD is enabled or not
    let a = SparseVec::from_data(b"test vector a");
    let b = SparseVec::from_data(b"test vector b");

    let sim = a.cosine(&b);

    // Should produce valid result
    assert!(sim >= -1.0 && sim <= 1.0);

    // Should match scalar baseline
    let sim_scalar = a.cosine_scalar(&b);
    assert!((sim - sim_scalar).abs() < 1e-10);
}

#[test]
fn test_integration_with_retrieval() {
    // Verify SIMD cosine works correctly in retrieval context
    use embeddenator::TernaryInvertedIndex;
    use std::collections::HashMap;

    let config = ReversibleVSAConfig::default();
    let query = SparseVec::encode_data(b"search query", &config, None);

    let mut vectors = HashMap::new();
    vectors.insert(0, SparseVec::encode_data(b"document one", &config, None));
    vectors.insert(1, SparseVec::encode_data(b"document two", &config, None));
    vectors.insert(2, SparseVec::encode_data(b"document three", &config, None));

    let index = TernaryInvertedIndex::build_from_map(&vectors);
    let results = index.query_top_k_reranked(&query, &vectors, 10, 3);

    // Should get valid results
    assert!(!results.is_empty());
    
    // Cosine scores should be in valid range
    for result in results {
        assert!(result.cosine >= -1.0 && result.cosine <= 1.0);
    }
}
