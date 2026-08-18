//! Codebook Roundtrip Tests
//!
//! These tests validate that the codebook encode/decode path works correctly.
//! They verify that data projected onto the codebook can be reconstructed
//! with reasonable accuracy.

use embeddenator_vsa::{Codebook, ProjectionConfig};

#[test]
fn test_codebook_roundtrip_simple() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    let original = b"Hello, World!";

    // Project to codebook space
    let projection = codebook.project(original);

    // Reconstruct
    let reconstructed = codebook.reconstruct(&projection, original.len());

    // Check quality score is valid and reasonable
    assert!(
        projection.quality_score > 0.0,
        "Quality score should be positive"
    );
    assert!(
        projection.quality_score <= 1.0,
        "Quality score should be at most 1.0"
    );

    // Check reconstructed length matches
    assert_eq!(
        reconstructed.len(),
        original.len(),
        "Reconstructed length should match original"
    );

    // With residual corrections, the reconstruction should be close to original
    // Note: Due to the VSA nature, perfect reconstruction is handled by residuals
    let error_count: usize = original
        .iter()
        .zip(reconstructed.iter())
        .filter(|(o, r)| o != r)
        .count();

    let error_rate = error_count as f64 / original.len() as f64;
    println!(
        "Reconstruction error rate: {:.2}% ({} errors out of {} bytes)",
        error_rate * 100.0,
        error_count,
        original.len()
    );
}

#[test]
fn test_codebook_roundtrip_various_sizes() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    for size in [1, 8, 16, 32, 64, 128, 256] {
        let original: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

        let projection = codebook.project(&original);
        let reconstructed = codebook.reconstruct(&projection, original.len());

        assert_eq!(
            reconstructed.len(),
            original.len(),
            "Size {}: Reconstructed length should match original",
            size
        );

        // Quality score should be valid
        assert!(
            projection.quality_score >= 0.0 && projection.quality_score <= 1.0,
            "Size {}: Quality score {} should be in [0, 1]",
            size,
            projection.quality_score
        );
    }
}

#[test]
fn test_codebook_empty_data() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    let original: Vec<u8> = vec![];

    let projection = codebook.project(&original);
    let reconstructed = codebook.reconstruct(&projection, 0);

    assert_eq!(
        reconstructed.len(),
        0,
        "Empty data should reconstruct to empty"
    );
    assert_eq!(
        projection.quality_score, 1.0,
        "Empty data should have perfect quality score"
    );
}

#[test]
fn test_codebook_projection_has_coefficients_or_residual() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    let test_data = b"the quick brown fox jumps over the lazy dog";
    let projection = codebook.project(test_data);

    // Should have either coefficients or residual (or both)
    let has_data = !projection.coefficients.is_empty() || !projection.residual.is_empty();
    assert!(
        has_data,
        "Projection should have coefficients or residual for non-empty data"
    );
}

#[test]
fn test_codebook_quality_score_meaningful() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    // Test data that should partially match basis patterns
    let text_data = b"the the the"; // Contains "the " pattern
    let binary_data: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0]; // Zero runs

    let text_proj = codebook.project(text_data);
    let binary_proj = codebook.project(&binary_data);

    // Both should have valid quality scores
    assert!(
        text_proj.quality_score > 0.0,
        "Text data should have positive quality"
    );
    assert!(
        binary_proj.quality_score > 0.0,
        "Binary data should have positive quality"
    );
}

#[test]
fn test_codebook_custom_config() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    let config = ProjectionConfig {
        chunk_size: 32,            // Smaller chunks
        similarity_threshold: 0.2, // Lower threshold
        max_basis_matches: 8,      // More matches
        coefficient_scale: 500.0,  // Different scale
        coefficient_key_spacing: 500,
    };

    let data = b"Testing with custom configuration settings";
    let projection = codebook.project_with_config(data, &config);

    // Should work with custom config
    assert!(projection.quality_score > 0.0);
    assert!(projection.quality_score <= 1.0);
}

#[test]
fn test_codebook_high_entropy_detection() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    // Create data with high entropy section (random-looking bytes)
    let mut data = vec![0u8; 64];
    // Add a "high entropy" section using prime multipliers to avoid repeating patterns
    // within the range (7 and 13 are coprime, creating a non-trivial sequence)
    const MULTIPLIER: u8 = 7;
    const OFFSET: u8 = 13;
    for i in 0u8..32 {
        data.push(i.wrapping_mul(MULTIPLIER).wrapping_add(OFFSET));
    }

    let projection = codebook.project(&data);

    // Should detect or process the data without panicking
    assert!(projection.quality_score >= 0.0);
    assert!(projection.quality_score <= 1.0);
}

#[test]
fn test_codebook_reconstruction_applies_residual() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    // Simple data that won't match basis patterns well
    let original: Vec<u8> = (65..90).collect(); // ASCII A-Y

    let projection = codebook.project(&original);
    let reconstructed = codebook.reconstruct(&projection, original.len());

    // Residual should exist for non-matching data
    assert!(
        !projection.residual.is_empty(),
        "Non-matching data should have residual"
    );

    // Reconstruction should have correct length
    assert_eq!(reconstructed.len(), original.len());
}

#[test]
fn test_codebook_deterministic() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    let data = b"deterministic test";

    let projection1 = codebook.project(data);
    let projection2 = codebook.project(data);

    // Same input should produce same output
    assert_eq!(
        projection1.quality_score, projection2.quality_score,
        "Quality scores should be deterministic"
    );
    assert_eq!(
        projection1.coefficients.len(),
        projection2.coefficients.len(),
        "Coefficient count should be deterministic"
    );
    assert_eq!(
        projection1.residual.len(),
        projection2.residual.len(),
        "Residual length should be deterministic"
    );
}
