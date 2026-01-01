//! Unit tests for Vector Symbolic Architecture (VSA)

use embeddenator::vsa::{SparseVec, VSAConfig};
use embeddenator::resonator::Resonator;
use std::collections::HashSet;

#[test]
fn test_sparse_vec_bundle() {
    let v1 = SparseVec {
        pos: vec![1, 2, 3],
        neg: vec![4, 5, 6],
    };
    let v2 = SparseVec {
        pos: vec![2, 3, 7],
        neg: vec![5, 6, 8],
    };
    let result = v1.bundle(&v2);

    assert!(result.pos.contains(&2));
    assert!(result.pos.contains(&3));
}

#[test]
fn test_sparse_vec_bind() {
    let v1 = SparseVec {
        pos: vec![1, 2, 3],
        neg: vec![4, 5, 6],
    };
    let v2 = SparseVec {
        pos: vec![2, 3, 7],
        neg: vec![5, 6, 8],
    };
    let result = v1.bind(&v2);

    assert!(result.pos.len() + result.neg.len() > 0);
}

#[test]
fn test_sparse_vec_cosine() {
    let v1 = SparseVec {
        pos: vec![1, 2, 3],
        neg: vec![4, 5, 6],
    };
    let v2 = v1.clone();
    let similarity = v1.cosine(&v2);

    assert!(similarity > 0.9);
}

#[test]
fn test_bundle_associativity() {
    let a = SparseVec {
        pos: vec![1, 2, 3],
        neg: vec![4, 5, 6],
    };
    let b = SparseVec {
        pos: vec![2, 3, 7],
        neg: vec![5, 6, 8],
    };
    let c = SparseVec {
        pos: vec![3, 7, 9],
        neg: vec![6, 8, 10],
    };

    let left = a.bundle(&b).bundle(&c);
    let right = a.bundle(&b.bundle(&c));

    let similarity = left.cosine(&right);
    assert!(
        similarity > 0.7,
        "Bundle associativity failed: similarity = {}",
        similarity
    );
}

#[test]
fn test_bind_self_inverse() {
    let a = SparseVec {
        pos: vec![1, 2, 3, 4, 5],
        neg: vec![6, 7, 8, 9, 10],
    };

    let result = a.bind(&a);
    assert!(!result.pos.is_empty() || !result.neg.is_empty());
}

#[test]
fn test_cosine_similarity_ranges() {
    let v1 = SparseVec {
        pos: vec![1, 2, 3],
        neg: vec![4, 5, 6],
    };
    let v2 = v1.clone();

    let self_sim = v1.cosine(&v2);
    assert!(self_sim > 0.9, "Self-similarity too low: {}", self_sim);

    let v3 = SparseVec {
        pos: vec![10, 20, 30],
        neg: vec![40, 50, 60],
    };
    let diff_sim = v1.cosine(&v3);
    assert!(
        diff_sim < 0.5,
        "Different vectors too similar: {}",
        diff_sim
    );
}

#[test]
fn test_from_data_determinism() {
    let data = b"test data for determinism";
    let v1 = SparseVec::from_data(data);
    let v2 = SparseVec::from_data(data);

    assert_eq!(v1.pos, v2.pos, "pos indices should match");
    assert_eq!(v1.neg, v2.neg, "neg indices should match");

    let similarity = v1.cosine(&v2);
    assert!(
        similarity > 0.999,
        "Determinism failed: identical data produced different vectors (similarity: {})",
        similarity
    );
}

#[test]
fn test_from_data_different_inputs() {
    let data1 = b"first input";
    let data2 = b"second input";

    let v1 = SparseVec::from_data(data1);
    let v2 = SparseVec::from_data(data2);

    assert_ne!(
        v1.pos, v2.pos,
        "Different inputs should produce different pos"
    );

    let similarity = v1.cosine(&v2);
    assert!(
        similarity < 0.5,
        "Different inputs too similar: {}",
        similarity
    );
}

#[test]
fn test_sparse_vec_random() {
    let v = SparseVec::random();

    assert!(
        !v.pos.is_empty(),
        "Random vector should have positive indices"
    );
    assert!(
        !v.neg.is_empty(),
        "Random vector should have negative indices"
    );

    let pos_set: HashSet<_> = v.pos.iter().collect();
    let neg_set: HashSet<_> = v.neg.iter().collect();
    assert!(
        pos_set.is_disjoint(&neg_set),
        "pos and neg should not overlap"
    );
}

#[test]
fn test_cleanup_threshold() {
    let correct = SparseVec {
        pos: vec![1, 2, 3, 4, 5],
        neg: vec![6, 7, 8, 9, 10],
    };

    let similar = SparseVec {
        pos: vec![1, 2, 3, 4, 11],
        neg: vec![6, 7, 8, 9, 12],
    };

    let noise = SparseVec {
        pos: vec![20, 21, 22, 23, 24],
        neg: vec![25, 26, 27, 28, 29],
    };

    let correct_sim = correct.cosine(&similar);
    let noise_sim = correct.cosine(&noise);

    assert!(
        correct_sim > 0.3,
        "Correct match should be >0.3: {}",
        correct_sim
    );
    assert!(noise_sim < 0.3, "Noise should be <0.3: {}", noise_sim);
}

#[test]
fn test_is_text_file() {
    use embeddenator::embrfs::is_text_file;

    let text_data = b"Hello, world!";
    assert!(is_text_file(text_data));

    let binary_data = vec![0u8, 1, 2, 3, 255, 0];
    assert!(!is_text_file(&binary_data));
}

#[test]
fn test_vsaconfig_new() {
    let config = VSAConfig::new(10000);
    assert_eq!(config.dimensionality, 10000);
    assert_eq!(config.target_non_zero, 200);
    assert!((config.sparsity - 0.02).abs() < 0.001);
    
    let config_50k = VSAConfig::new(50000);
    assert_eq!(config_50k.dimensionality, 50000);
    assert!((config_50k.sparsity - 0.004).abs() < 0.001);
}

#[test]
fn test_vsaconfig_presets() {
    let high = VSAConfig::high_precision();
    assert_eq!(high.dimensionality, 100_000);
    assert!((high.sparsity - 0.002).abs() < 0.001);
    
    let balanced = VSAConfig::balanced();
    assert_eq!(balanced.dimensionality, 50_000);
    assert!((balanced.sparsity - 0.004).abs() < 0.001);
    
    let fast = VSAConfig::fast();
    assert_eq!(fast.dimensionality, 10_000);
    assert!((fast.sparsity - 0.02).abs() < 0.001);
}

#[test]
fn test_vsaconfig_serialization() {
    let config = VSAConfig::balanced();
    let serialized = serde_json::to_string(&config).unwrap();
    let deserialized: VSAConfig = serde_json::from_str(&serialized).unwrap();
    assert_eq!(config.dimensionality, deserialized.dimensionality);
    assert_eq!(config.target_non_zero, deserialized.target_non_zero);
    assert!((config.sparsity - deserialized.sparsity).abs() < 0.001);
}

#[test]
fn test_permute_identity() {
    let vec = SparseVec::from_data(b"test data");
    let permuted = vec.permute(0);
    
    // permute(0) should be identical
    assert_eq!(vec.pos, permuted.pos);
    assert_eq!(vec.neg, permuted.neg);
}

#[test]
fn test_permute_cycle() {
    let vec = SparseVec::from_data(b"test data");
    let permuted = vec.permute(embeddenator::vsa::DIM);
    
    // permute(DIM) should complete cycle and be identical
    assert_eq!(vec.pos, permuted.pos);
    assert_eq!(vec.neg, permuted.neg);
}

#[test]
fn test_permute_changes_indices() {
    let vec = SparseVec::from_data(b"test data");
    let permuted = vec.permute(100);
    
    // Non-zero shift should change indices (unless all indices happen to map to same positions)
    let pos_changed = vec.pos != permuted.pos;
    let neg_changed = vec.neg != permuted.neg;
    
    // At least one array should be different (very unlikely both remain identical)
    assert!(pos_changed || neg_changed);
    
    // But structure should be preserved
    assert_eq!(vec.pos.len(), permuted.pos.len());
    assert_eq!(vec.neg.len(), permuted.neg.len());
}

#[test]
fn test_permute_round_trip() {
    let vec = SparseVec::from_data(b"test data");
    let shift = 123;
    
    let permuted = vec.permute(shift);
    let recovered = permuted.inverse_permute(shift);
    
    // Round-trip should recover original vector exactly
    assert_eq!(vec.pos, recovered.pos);
    assert_eq!(vec.neg, recovered.neg);
}

#[test]
fn test_permute_orthogonality() {
    let vec = SparseVec::from_data(b"test data");
    
    // Test multiple shifts to ensure orthogonality
    for shift in [100, 500, 1000, 2500] {
        let permuted = vec.permute(shift);
        let similarity = vec.cosine(&permuted);
        
        // Permuted vectors should be nearly orthogonal to original
        // With DIM=10000 and ~200 non-zero elements, expect very low similarity
        assert!(similarity < 0.1, "Shift {} gave similarity {}", shift, similarity);
    }
}

#[test]
fn test_thin_reduces_density() {
    // Create a vector with ~400 non-zero elements (twice the target)
    let mut test_vec = SparseVec::new();
    for i in (0..400).step_by(2) {
        test_vec.pos.push(i);
        test_vec.neg.push(i + 1);
    }
    
    let thinned = test_vec.thin(200);
    let total_elements = thinned.pos.len() + thinned.neg.len();
    
    // Should reduce to approximately 200 elements
    assert!(total_elements <= 200, "Expected <= 200 elements, got {}", total_elements);
    assert!(total_elements > 180, "Expected > 180 elements, got {}", total_elements);
}

#[test]
fn test_thin_no_change_when_smaller() {
    // Create a vector with ~200 non-zero elements
    let mut test_vec = SparseVec::new();
    for i in (0..200).step_by(2) {
        test_vec.pos.push(i);
        test_vec.neg.push(i + 1);
    }
    
    let thinned = test_vec.thin(500); // Target larger than current
    
    // Should return unchanged
    assert_eq!(test_vec.pos, thinned.pos);
    assert_eq!(test_vec.neg, thinned.neg);
}

#[test]
fn test_bundle_with_config_thinning() {
    let config = VSAConfig::balanced(); // target_non_zero = 200
    
    // Create 10 vectors that will bundle to more than 200 non-zeros
    let vectors: Vec<SparseVec> = (0..10)
        .map(|i| SparseVec::from_data(format!("test data {}", i).as_bytes()))
        .collect();
    
    // Bundle them all with config
    let mut result = vectors[0].clone();
    for vec in &vectors[1..] {
        result = result.bundle_with_config(vec, Some(&config));
    }
    
    let total_elements = result.pos.len() + result.neg.len();
    
    // Should be thinned to approximately 200 elements
    assert!(total_elements <= 220, "Expected <= 220 elements, got {}", total_elements);
    assert!(total_elements >= 180, "Expected >= 180 elements, got {}", total_elements);
}

#[test]
fn test_resonator_new() {
    let resonator = Resonator::new();
    assert_eq!(resonator.max_iterations, 10);
    assert_eq!(resonator.convergence_threshold, 0.001);
    assert!(resonator.codebook.is_empty());
}

#[test]
fn test_resonator_with_params() {
    let codebook = vec![SparseVec::from_data(b"pattern1"), SparseVec::from_data(b"pattern2")];
    let resonator = Resonator::with_params(codebook.clone(), 20, 0.0001);
    assert_eq!(resonator.max_iterations, 20);
    assert_eq!(resonator.convergence_threshold, 0.0001);
    assert_eq!(resonator.codebook.len(), 2);
}

#[test]
fn test_resonator_project_clean_input() {
    let clean = SparseVec::from_data(b"hello");
    let codebook = vec![clean.clone(), SparseVec::from_data(b"world")];
    let resonator = Resonator::with_params(codebook, 10, 0.001);

    // Clean input should project to itself
    let projected = resonator.project(&clean);
    let similarity = clean.cosine(&projected);
    assert!(similarity > 0.9, "Similarity was {}", similarity);
}

#[test]
fn test_resonator_project_empty_codebook() {
    let resonator = Resonator::new();
    let input = SparseVec::from_data(b"test");

    // Empty codebook should return input unchanged
    let projected = resonator.project(&input);
    assert_eq!(input.pos, projected.pos);
    assert_eq!(input.neg, projected.neg);
}

#[test]
fn test_resonator_factorize_empty_codebook() {
    let resonator = Resonator::new();
    let compound = SparseVec::from_data(b"test");

    let result = resonator.factorize(&compound, 2);
    assert!(result.factors.is_empty());
    assert_eq!(result.iterations, 0);
    assert_eq!(result.final_delta, 0.0);
}

#[test]
fn test_resonator_factorize_zero_factors() {
    let codebook = vec![SparseVec::from_data(b"pattern1")];
    let resonator = Resonator::with_params(codebook, 10, 0.001);
    let compound = SparseVec::from_data(b"test");

    let result = resonator.factorize(&compound, 0);
    assert!(result.factors.is_empty());
    assert_eq!(result.iterations, 0);
    assert_eq!(result.final_delta, 0.0);
}

#[test]
fn test_resonator_factorize_convergence() {
    let factor1 = SparseVec::from_data(b"hello");
    let factor2 = SparseVec::from_data(b"world");
    let compound = factor1.bundle(&factor2);

    let codebook = vec![factor1.clone(), factor2.clone()];
    let resonator = Resonator::with_params(codebook, 20, 0.001);

    let result = resonator.factorize(&compound, 2);

    // Should return 2 factors
    assert_eq!(result.factors.len(), 2);
    // Should converge within reasonable iterations
    assert!(result.iterations <= 20);
    // Final delta should be reasonable
    assert!(result.final_delta >= 0.0);
    assert!(result.final_delta < 1.0);
}

#[test]
fn test_resonator_sign_threshold() {
    let resonator = Resonator::new();
    let similarities = vec![0.8, -0.3, 0.05, -0.9, 0.0];
    let ternary = resonator.sign_threshold(&similarities, 0.1);

    assert_eq!(ternary, vec![1, -1, 0, -1, 0]);
}

#[test]
fn test_resonator_sign_threshold_zero_threshold() {
    let resonator = Resonator::new();
    let similarities = vec![0.1, -0.1, 0.0];
    let ternary = resonator.sign_threshold(&similarities, 0.0);

    // With zero threshold, all non-zero values should be thresholded
    assert_eq!(ternary, vec![1, -1, 0]);
}

#[test]
fn test_resonator_sign_threshold_high_threshold() {
    let resonator = Resonator::new();
    let similarities = vec![0.5, -0.5, 0.05];
    let ternary = resonator.sign_threshold(&similarities, 0.6);

    // With high threshold, only strong similarities should pass
    assert_eq!(ternary, vec![0, 0, 0]);
}
