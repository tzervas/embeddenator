//! Unit tests for Resonator pattern completion and factorization

use embeddenator_retrieval::resonator::Resonator;
use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};

fn enc(data: &[u8]) -> SparseVec {
    SparseVec::encode_data(data, &ReversibleVSAConfig::default(), None)
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
    let codebook = vec![enc(b"pattern1"), enc(b"pattern2")];
    let resonator = Resonator::with_params(codebook.clone(), 20, 0.0001);
    assert_eq!(resonator.max_iterations, 20);
    assert_eq!(resonator.convergence_threshold, 0.0001);
    assert_eq!(resonator.codebook.len(), 2);
}

#[test]
fn test_resonator_project_clean_input() {
    let clean = enc(b"hello");
    let codebook = vec![clean.clone(), enc(b"world")];
    let resonator = Resonator::with_params(codebook, 10, 0.001);

    // Clean input should project to itself
    let projected = resonator.project(&clean);
    let similarity = clean.cosine(&projected);
    assert!(similarity > 0.9, "Similarity was {}", similarity);
}

#[test]
fn test_resonator_project_empty_codebook() {
    let resonator = Resonator::new();
    let input = enc(b"test");

    // Empty codebook should return input unchanged
    let projected = resonator.project(&input);
    assert_eq!(input.pos, projected.pos);
    assert_eq!(input.neg, projected.neg);
}

#[test]
fn test_resonator_factorize_empty_codebook() {
    let resonator = Resonator::new();
    let compound = enc(b"test");

    let result = resonator.factorize(&compound, 2);
    assert!(result.factors.is_empty());
    assert_eq!(result.iterations, 0);
    assert_eq!(result.final_delta, 0.0);
}

#[test]
fn test_resonator_factorize_zero_factors() {
    let codebook = vec![enc(b"pattern1")];
    let resonator = Resonator::with_params(codebook, 10, 0.001);
    let compound = enc(b"test");

    let result = resonator.factorize(&compound, 0);
    assert!(result.factors.is_empty());
    assert_eq!(result.iterations, 0);
    assert_eq!(result.final_delta, 0.0);
}

#[test]
fn test_resonator_factorize_convergence() {
    let factor1 = enc(b"hello");
    let factor2 = enc(b"world");
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

// ============================================================================
// Chunk Recovery Tests (#54)
// ============================================================================

use std::collections::HashMap;

#[test]
fn test_recover_chunks_empty_missing() {
    let resonator = Resonator::new();
    let available: HashMap<usize, Vec<u8>> = HashMap::new();
    let config = ReversibleVSAConfig::default();

    let recovered = resonator.recover_chunks(&available, &[], &config);
    assert!(recovered.is_empty());
}

#[test]
fn test_recover_chunks_with_both_neighbors() {
    let resonator = Resonator::new();
    let config = ReversibleVSAConfig::default();

    let mut available = HashMap::new();
    // Chunk 0: all 100s
    available.insert(0, vec![100u8; 64]);
    // Chunk 2: all 200s
    available.insert(2, vec![200u8; 64]);

    // Recover chunk 1 (missing between 0 and 2)
    let recovered = resonator.recover_chunks(&available, &[1], &config);

    assert_eq!(recovered.len(), 1);
    let chunk1 = recovered.get(&1).expect("Chunk 1 should be recovered");
    assert_eq!(chunk1.len(), 64);

    // Interpolation should give approximately 150 (average of 100 and 200)
    for byte in chunk1.iter() {
        assert_eq!(*byte, 150, "Interpolated byte should be average");
    }
}

#[test]
fn test_recover_chunks_with_single_neighbor_prev() {
    let codebook = vec![enc(b"pattern1"), enc(b"pattern2")];
    let resonator = Resonator::with_params(codebook, 10, 0.001);
    let config = ReversibleVSAConfig::default();

    let mut available = HashMap::new();
    available.insert(0, vec![42u8; 64]);

    // Recover chunk 1 with only previous neighbor
    let recovered = resonator.recover_chunks(&available, &[1], &config);

    assert_eq!(recovered.len(), 1);
    let chunk1 = recovered.get(&1).expect("Chunk 1 should be recovered");
    assert_eq!(chunk1.len(), 64);
}

#[test]
fn test_recover_chunks_with_single_neighbor_next() {
    let codebook = vec![enc(b"pattern1"), enc(b"pattern2")];
    let resonator = Resonator::with_params(codebook, 10, 0.001);
    let config = ReversibleVSAConfig::default();

    let mut available = HashMap::new();
    available.insert(1, vec![42u8; 64]);

    // Recover chunk 0 with only next neighbor
    let recovered = resonator.recover_chunks(&available, &[0], &config);

    assert_eq!(recovered.len(), 1);
    let chunk0 = recovered.get(&0).expect("Chunk 0 should be recovered");
    assert_eq!(chunk0.len(), 64);
}

#[test]
fn test_recover_chunks_codebook_fallback() {
    let codebook = vec![enc(b"test pattern data")];
    let resonator = Resonator::with_params(codebook, 10, 0.001);
    let config = ReversibleVSAConfig::default();

    // No neighbors available, should use codebook projection
    let available: HashMap<usize, Vec<u8>> = HashMap::new();
    let recovered = resonator.recover_chunks(&available, &[5], &config);

    assert_eq!(recovered.len(), 1);
    let chunk5 = recovered.get(&5).expect("Chunk 5 should be recovered");
    // Codebook recovery produces some data (size depends on pattern decoding)
    assert!(!chunk5.is_empty(), "Recovered chunk should have data");
}

#[test]
fn test_recover_chunks_zero_fill_fallback() {
    let resonator = Resonator::new(); // Empty codebook
    let config = ReversibleVSAConfig::default();

    // No neighbors and no codebook - should zero-fill
    let available: HashMap<usize, Vec<u8>> = HashMap::new();
    let recovered = resonator.recover_chunks(&available, &[10], &config);

    assert_eq!(recovered.len(), 1);
    let chunk10 = recovered.get(&10).expect("Chunk 10 should be recovered");
    assert_eq!(chunk10.len(), 4096);
    assert!(chunk10.iter().all(|&b| b == 0), "Should be zero-filled");
}

#[test]
fn test_recover_chunks_multiple_missing() {
    let resonator = Resonator::new();
    let config = ReversibleVSAConfig::default();

    let mut available = HashMap::new();
    available.insert(0, vec![10u8; 64]);
    available.insert(3, vec![40u8; 64]);

    // Recover chunks 1 and 2
    let recovered = resonator.recover_chunks(&available, &[1, 2], &config);

    assert_eq!(recovered.len(), 2);
    assert!(recovered.contains_key(&1));
    assert!(recovered.contains_key(&2));
}

#[test]
fn test_recover_chunks_preserves_chunk_size() {
    let resonator = Resonator::new();
    let config = ReversibleVSAConfig::default();

    let chunk_size = 256;
    let mut available = HashMap::new();
    available.insert(0, vec![0u8; chunk_size]);
    available.insert(2, vec![255u8; chunk_size]);

    let recovered = resonator.recover_chunks(&available, &[1], &config);

    let chunk1 = recovered.get(&1).expect("Chunk 1 should be recovered");
    assert_eq!(
        chunk1.len(),
        chunk_size,
        "Recovered chunk should match neighbor size"
    );
}
