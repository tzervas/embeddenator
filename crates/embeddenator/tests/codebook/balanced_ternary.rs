//! Balanced Ternary Word Tests
//!
//! Unit tests for BalancedTernaryWord encoding/decoding, metadata,
//! and parity computation.
//!
//! Run with: cargo test --test codebook

use embeddenator::{BalancedTernaryWord, WordMetadata};

#[test]
fn test_balanced_ternary_roundtrip() {
    let test_values = [
        0i64,
        1,
        -1,
        100,
        -100,
        12345,
        -12345,
        BalancedTernaryWord::MAX_VALUE / 2,
        BalancedTernaryWord::MIN_VALUE / 2,
    ];

    for &value in &test_values {
        let word = BalancedTernaryWord::new(value, WordMetadata::Data).unwrap();
        let decoded = word.decode();
        assert_eq!(value, decoded, "Failed roundtrip for {}", value);
    }
}

#[test]
fn test_balanced_ternary_metadata() {
    let word = BalancedTernaryWord::new(42, WordMetadata::SemanticOutlier).unwrap();
    assert_eq!(word.metadata(), WordMetadata::SemanticOutlier);
    assert_eq!(word.decode(), 42);
}

#[test]
fn test_balanced_ternary_range() {
    // Should succeed at boundaries
    assert!(BalancedTernaryWord::new(
        BalancedTernaryWord::MAX_VALUE,
        WordMetadata::Data
    )
    .is_some());
    assert!(BalancedTernaryWord::new(
        BalancedTernaryWord::MIN_VALUE,
        WordMetadata::Data
    )
    .is_some());

    // Should fail outside boundaries
    assert!(BalancedTernaryWord::new(
        BalancedTernaryWord::MAX_VALUE + 1,
        WordMetadata::Data
    )
    .is_none());
    assert!(BalancedTernaryWord::new(
        BalancedTernaryWord::MIN_VALUE - 1,
        WordMetadata::Data
    )
    .is_none());
}

#[test]
fn test_parity_computation() {
    let word = BalancedTernaryWord::new(12345, WordMetadata::Data).unwrap();
    let parity = word.compute_parity();
    assert!(
        parity >= -1 && parity <= 1,
        "Parity must be -1, 0, or 1, got {}",
        parity
    );
}

#[test]
fn test_metadata_preservation() {
    let test_metadata = [
        WordMetadata::Data,
        WordMetadata::SemanticOutlier,
        WordMetadata::Parity,
    ];

    for &metadata in &test_metadata {
        let word = BalancedTernaryWord::new(42, metadata).unwrap();
        assert_eq!(
            word.metadata(),
            metadata,
            "Metadata not preserved for {:?}",
            metadata
        );
    }
}

#[test]
fn test_edge_case_values() {
    // Test zero
    let zero = BalancedTernaryWord::new(0, WordMetadata::Data).unwrap();
    assert_eq!(zero.decode(), 0);
    assert_eq!(zero.compute_parity(), 0);

    // Test small positive/negative
    let pos = BalancedTernaryWord::new(1, WordMetadata::Data).unwrap();
    assert_eq!(pos.decode(), 1);

    let neg = BalancedTernaryWord::new(-1, WordMetadata::Data).unwrap();
    assert_eq!(neg.decode(), -1);
}
