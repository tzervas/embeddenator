//! Codebook Projection Tests
//!
//! Tests for codebook initialization and data projection operations.
//!
//! Run with: cargo test --test codebook

use embeddenator::Codebook;

#[test]
fn test_codebook_projection() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    let data = b"the quick brown fox jumps over the lazy dog";
    let projection = codebook.project(data);

    assert!(
        projection.quality_score > 0.0,
        "Quality score should be positive"
    );
    assert!(
        !projection.coefficients.is_empty() || !projection.residual.is_empty(),
        "Projection should have coefficients or residual"
    );
}

#[test]
fn test_codebook_initialization() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    // Verify initialization produces valid projections
    let test_data = b"test";
    let projection = codebook.project(test_data);
    assert!(
        projection.quality_score >= 0.0,
        "Codebook should be initialized and functional"
    );
}

#[test]
fn test_empty_data_projection() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    let empty_data = b"";
    let projection = codebook.project(empty_data);

    // Empty data should still produce a valid projection
    assert!(projection.quality_score >= 0.0);
}

#[test]
fn test_projection_repeatability() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    let data = b"test data for repeatability";
    let projection1 = codebook.project(data);
    let projection2 = codebook.project(data);

    // Same input should produce identical results
    assert_eq!(
        projection1.quality_score, projection2.quality_score,
        "Projections should be deterministic"
    );
    assert_eq!(
        projection1.coefficients.len(),
        projection2.coefficients.len(),
        "Coefficient counts should match"
    );
}

#[test]
fn test_different_data_different_projections() {
    let mut codebook = Codebook::new(10000);
    codebook.initialize_standard_basis();

    let data1 = b"first set of data";
    let data2 = b"completely different data";

    let projection1 = codebook.project(data1);
    let projection2 = codebook.project(data2);

    // Different inputs should produce different quality scores or residuals
    // (with extremely high probability)
    assert!(
        projection1.quality_score != projection2.quality_score
            || projection1.residual != projection2.residual
            || projection1.coefficients != projection2.coefficients,
        "Different data should produce different projections"
    );
}
