//! Testing Infrastructure Tests
//!
//! Tests for the testing module itself, including metrics, integrity reports,
//! storage footprint analysis, and chaos injection.
//!
//! Run with: cargo test --test testing_infrastructure

use embeddenator::testing::{ChaosInjector, IntegrityReport, StorageFootprint, TestMetrics};
use embeddenator::vsa::SparseVec;
use embeddenator::BitslicedTritVec;

#[test]
fn test_metrics_timing() {
    let mut metrics = TestMetrics::new("test_op");

    for _ in 0..10 {
        metrics.time_operation(|| {
            std::thread::sleep(std::time::Duration::from_micros(100));
        });
    }

    let stats = metrics.timing_stats();
    assert_eq!(stats.count, 10);
    assert!(stats.mean_ns > 50_000.0, "Expected at least 50µs mean"); // At least 50µs
}

#[test]
fn test_integrity_report() {
    let mut report = IntegrityReport::default();

    report.pass();
    report.pass();
    report.fail("test failure");

    assert_eq!(report.checks_total, 3);
    assert_eq!(report.checks_passed, 2);
    assert!(!report.is_ok());
    assert!(
        (report.pass_rate() - 66.67).abs() < 1.0,
        "Pass rate should be approximately 66.67%"
    );
}

#[test]
fn test_storage_footprint() {
    let footprint = StorageFootprint {
        raw_bytes: 10000,
        bitsliced_bytes: 4000,
        codebook_bytes: 500,
        metadata_bytes: 100,
        dimension: 10000,
        nnz: 200,
        ..Default::default()
    };

    assert!(
        (footprint.density() - 0.02).abs() < 0.001,
        "Density should be approximately 0.02"
    );
    assert!(
        footprint.compression_ratio() > 2.0,
        "Compression ratio should be > 2.0"
    );
}

#[test]
fn test_chaos_injector() {
    let sparse = SparseVec {
        pos: vec![0, 100, 500],
        neg: vec![50, 200],
    };
    let mut v = BitslicedTritVec::from_sparse(&sparse, 1000);
    let original_nnz = v.nnz();

    let injector = ChaosInjector::new(42);
    let flipped = injector.inject_bitflips(&mut v, 5);

    assert_eq!(flipped.len(), 5, "Should flip exactly 5 positions");
    // NNZ might change due to flips
    assert!(
        v.nnz() != original_nnz
            || flipped
                .iter()
                .any(|&p| sparse.pos.contains(&p) || sparse.neg.contains(&p)),
        "Bitflips should have an effect"
    );
}

#[test]
fn test_chaos_injector_reproducibility() {
    let sparse = SparseVec {
        pos: vec![0, 100, 500],
        neg: vec![50, 200],
    };

    // Two vectors from the same sparse vector
    let mut v1 = BitslicedTritVec::from_sparse(&sparse, 1000);
    let mut v2 = BitslicedTritVec::from_sparse(&sparse, 1000);

    // Same seed should produce same flips
    let injector1 = ChaosInjector::new(42);
    let injector2 = ChaosInjector::new(42);

    let flipped1 = injector1.inject_bitflips(&mut v1, 5);
    let flipped2 = injector2.inject_bitflips(&mut v2, 5);

    assert_eq!(
        flipped1, flipped2,
        "Same seed should produce same flips"
    );
}

#[test]
fn test_integrity_report_perfect_score() {
    let mut report = IntegrityReport::default();

    for _ in 0..10 {
        report.pass();
    }

    assert_eq!(report.checks_total, 10);
    assert_eq!(report.checks_passed, 10);
    assert!(report.is_ok());
    assert_eq!(report.pass_rate(), 100.0);
}

#[test]
fn test_integrity_report_no_checks() {
    let report = IntegrityReport::default();

    assert_eq!(report.checks_total, 0);
    assert_eq!(report.checks_passed, 0);
    // No checks means no failures, so it should be "ok"
    assert!(report.is_ok());
}

#[test]
fn test_storage_footprint_zero_dimension() {
    let footprint = StorageFootprint {
        dimension: 0,
        nnz: 0,
        ..Default::default()
    };

    // Should handle zero dimension gracefully
    assert_eq!(footprint.density(), 0.0);
}
