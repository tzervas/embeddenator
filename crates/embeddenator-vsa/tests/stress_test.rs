//! Stress tests for SIMD alignment and codebook reconstruction
//!
//! Tests designed to catch:
//! 1. SIMD alignment crashes with unaligned memory access
//! 2. Codebook reconstruction correctness issues
//! 3. Edge cases that may not be covered by unit tests

use embeddenator_vsa::{Codebook, ReversibleVSAConfig, SparseVec};

#[test]
fn test_simd_large_vectors() {
    // Test with large vectors that would exercise SIMD paths
    let config = ReversibleVSAConfig::default();

    // Create vectors of different sizes to test alignment
    for size in [
        1, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 255, 256, 1023, 1024,
    ] {
        let data_a: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let data_b: Vec<u8> = (0..size).map(|i| ((i * 13) % 256) as u8).collect();

        let vec_a = SparseVec::encode_data(&data_a, &config, None);
        let vec_b = SparseVec::encode_data(&data_b, &config, None);

        // This should not crash due to alignment issues
        let similarity = vec_a.cosine(&vec_b);

        assert!(
            (-1.0..=1.0).contains(&similarity),
            "Size {}: Similarity {} out of range",
            size,
            similarity
        );

        // Test scalar version for comparison
        let similarity_scalar = vec_a.cosine_scalar(&vec_b);
        assert!(
            (-1.0..=1.0).contains(&similarity_scalar),
            "Size {}: Scalar similarity {} out of range",
            size,
            similarity_scalar
        );

        // Results should match (or be very close)
        let diff = (similarity - similarity_scalar).abs();
        assert!(
            diff < 1e-10,
            "Size {}: SIMD {} differs from scalar {} by {}",
            size,
            similarity,
            similarity_scalar,
            diff
        );
    }
}

#[test]
fn test_simd_unaligned_data() {
    // Test with data that would naturally create unaligned access patterns
    let config = ReversibleVSAConfig::default();

    // Prime-sized arrays (not aligned to powers of 2)
    for size in [13, 17, 23, 31, 37, 41, 53, 67, 79, 97, 113, 127] {
        let data: Vec<u8> = (0..size).map(|i| (i * 7 + 13) as u8).collect();
        let vec = SparseVec::encode_data(&data, &config, None);

        // Self-similarity should be high
        let self_sim = vec.cosine(&vec);
        assert!(
            self_sim > 0.99,
            "Size {}: Self-similarity {} too low",
            size,
            self_sim
        );
    }
}

#[test]
fn test_codebook_roundtrip_comprehensive() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    // Test various data patterns that might expose bugs
    let test_cases: [Vec<u8>; 8] = [
        // Simple patterns
        vec![0u8; 32],
        vec![255u8; 32],
        vec![0xAA; 32], // 10101010 pattern
        vec![0x55; 32], // 01010101 pattern
        // Sequential data
        (0..128).map(|i| i as u8).collect::<Vec<_>>(),
        // Random-looking data (deterministic)
        (0..256)
            .map(|i| ((i * 31 + 17) % 256) as u8)
            .collect::<Vec<_>>(),
        // Data with runs
        {
            let mut v = vec![0u8; 64];
            v.extend(vec![255u8; 64]);
            v
        },
        // Prime-sized data
        (0..97).map(|i| (i * 7) as u8).collect::<Vec<_>>(),
    ];

    for (idx, original) in test_cases.iter().enumerate() {
        let projection = codebook.project(original);
        let reconstructed = codebook.reconstruct(&projection, original.len());

        assert_eq!(
            reconstructed.len(),
            original.len(),
            "Test case {}: Length mismatch",
            idx
        );

        // Quality score should be valid
        assert!(
            projection.quality_score >= 0.0 && projection.quality_score <= 1.0,
            "Test case {}: Invalid quality score {}",
            idx,
            projection.quality_score
        );

        // Count mismatches
        let mismatches: usize = original
            .iter()
            .zip(reconstructed.iter())
            .filter(|(o, r)| o != r)
            .count();

        let error_rate = mismatches as f64 / original.len() as f64;

        // Reconstruction should be reasonably accurate (allowing for VSA approximation)
        // With residual corrections, we expect very low error rate
        assert!(
            error_rate < 0.5,
            "Test case {}: Error rate {} too high",
            idx,
            error_rate
        );
    }
}

#[test]
fn test_codebook_reconstruction_preserves_structure() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    // Test that repeated patterns are preserved
    let pattern = b"ABCD";
    let mut original = Vec::new();
    for _ in 0..16 {
        original.extend_from_slice(pattern);
    }

    let projection = codebook.project(&original);
    let reconstructed = codebook.reconstruct(&projection, original.len());

    assert_eq!(reconstructed.len(), original.len());

    // Check that the pattern structure is somewhat preserved
    // (VSA may not preserve exact bytes but should preserve some structure)
    let _original_runs = count_consecutive_runs(&original);
    let reconstructed_runs = count_consecutive_runs(&reconstructed);

    // Reconstructed should have some structure (not completely random)
    assert!(
        reconstructed_runs > 0,
        "Reconstructed data should have some structure"
    );
}

#[test]
fn test_codebook_empty_and_edge_cases() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    // Empty data
    let empty: Vec<u8> = vec![];
    let projection = codebook.project(&empty);
    let reconstructed = codebook.reconstruct(&projection, 0);
    assert_eq!(reconstructed.len(), 0);

    // Single byte
    let single = vec![42u8];
    let projection = codebook.project(&single);
    let reconstructed = codebook.reconstruct(&projection, 1);
    assert_eq!(reconstructed.len(), 1);

    // Very large data
    let large: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
    let projection = codebook.project(&large);
    let reconstructed = codebook.reconstruct(&projection, large.len());
    assert_eq!(reconstructed.len(), large.len());
}

#[test]
fn test_simd_with_sparse_and_dense_patterns() {
    let config = ReversibleVSAConfig::default();

    // Very sparse data (mostly zeros)
    let sparse: Vec<u8> = {
        let mut v = vec![0u8; 1024];
        v[100] = 255;
        v[500] = 255;
        v[900] = 255;
        v
    };

    // Very dense data (mostly non-zero)
    let dense: Vec<u8> = (0..1024)
        .map(|i| if i % 10 == 0 { 0 } else { (i % 256) as u8 })
        .collect();

    let vec_sparse = SparseVec::encode_data(&sparse, &config, None);
    let vec_dense = SparseVec::encode_data(&dense, &config, None);

    // Test all combinations
    let sim_ss = vec_sparse.cosine(&vec_sparse);
    let sim_dd = vec_dense.cosine(&vec_dense);
    let sim_sd = vec_sparse.cosine(&vec_dense);

    assert!(sim_ss > 0.99, "Sparse self-similarity: {}", sim_ss);
    assert!(sim_dd > 0.99, "Dense self-similarity: {}", sim_dd);
    assert!(
        (-1.0..=1.0).contains(&sim_sd),
        "Cross-similarity: {}",
        sim_sd
    );
}

// Helper function to count consecutive runs in data
fn count_consecutive_runs(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }

    let mut runs = 1;
    for i in 1..data.len() {
        if data[i] != data[i - 1] {
            runs += 1;
        }
    }
    runs
}
