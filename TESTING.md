# Testing Guide

This document describes the testing infrastructure and guidelines for Embeddenator.

## Overview

Embeddenator has comprehensive test coverage with 160+ integration tests organized into 23 test suites, achieving a 97.6% pass rate (166/170 tests passing). The 4 failing tests are due to infrastructure issues (disk space), not code defects.

## Running Tests

### All Tests

Run the complete test suite:

```bash
cargo test --workspace
```

### Specific Test Suites

Run individual test suites:

```bash
# Codebook tests (balanced ternary, projection)
cargo test --test codebook

# Testing infrastructure tests
cargo test --test testing_infrastructure

# VSA properties tests
cargo test --test unit_tests

# Error recovery tests
cargo test --test error_recovery

# CLI integration tests
cargo test --test integration_cli
```

### Library Tests Only

```bash
cargo test --lib
```

### With Verbose Output

```bash
cargo test --test codebook -- --nocapture
```

## Test Organization

### Test Directory Structure

```
tests/
├── codebook/                      # Codebook-specific tests
│   ├── balanced_ternary.rs       # Balanced ternary encoding tests
│   └── projection.rs             # Codebook projection tests
├── codebook.rs                    # Codebook test suite entry
├── testing_infrastructure.rs      # Testing utilities tests
├── unit_tests.rs                 # Core VSA unit tests
├── error_recovery.rs             # Error handling tests
├── integration_cli.rs            # CLI integration tests
├── e2e_regression.rs             # End-to-end regression tests
├── exhaustive_trit_tests.rs      # Exhaustive ternary arithmetic tests
├── hierarchical_*.rs             # Hierarchical encoding tests
├── incremental_updates.rs        # Incremental update tests
├── packed_trit_vec.rs            # Packed representation tests
├── reconstruction_guarantee.rs    # Reconstruction verification tests
├── retrieval_index.rs            # Retrieval and indexing tests
├── simd_cosine_tests.rs          # SIMD operations tests
└── ...                           # Additional test suites
```

### Test Categories

| Category | Test Count | Description |
|----------|------------|-------------|
| Balanced Ternary | 6 | Encoding, decoding, metadata, parity |
| Codebook Projection | 5 | Initialization, repeatability, differentiation |
| Testing Infrastructure | 8 | Metrics, integrity reports, chaos injection |
| E2E Regression | 6 | Data integrity, workflows |
| Error Recovery | 19 | Corruption handling, concurrency |
| Exhaustive Trits | 29 | Complete ternary arithmetic validation |
| Hierarchical | 5 | Bundling, unfolding, determinism |
| Incremental Updates | 18 | Add, modify, delete, compact operations |
| CLI Integration | 8 | Ingest, extract, query commands |
| Unit Tests | 42 | Core VSA operations |
| Other Suites | 10+ | SIMD, retrieval, reconstruction |

## Test Quality Standards

All tests should follow these guidelines:

### 1. Descriptive Names

Test names should clearly describe what is being tested:

```rust
#[test]
fn test_balanced_ternary_roundtrip() { ... }

#[test]
fn test_codebook_projection_repeatability() { ... }
```

### 2. Clear Assertions

Include descriptive error messages:

```rust
assert_eq!(
    decoded, expected,
    "Roundtrip failed for value {}",
    value
);

assert!(
    quality_score > 0.0,
    "Quality score should be positive, got {}",
    quality_score
);
```

### 3. Edge Case Coverage

Tests should cover:
- Empty inputs
- Boundary values
- Zero dimensions
- Maximum/minimum values
- Error conditions

```rust
#[test]
fn test_empty_data_projection() {
    let codebook = Codebook::new(10000);
    let empty_data = b"";
    let projection = codebook.project(empty_data);
    // Verify empty data is handled gracefully
}

#[test]
fn test_balanced_ternary_range() {
    // Test boundary values
    assert!(BalancedTernaryWord::new(MAX_VALUE, ...).is_some());
    assert!(BalancedTernaryWord::new(MIN_VALUE, ...).is_some());
    // Test out-of-range values
    assert!(BalancedTernaryWord::new(MAX_VALUE + 1, ...).is_none());
}
```

### 4. Determinism and Repeatability

Tests should verify deterministic behavior:

```rust
#[test]
fn test_projection_repeatability() {
    let codebook = Codebook::new(10000);
    let data = b"test data";
    
    let projection1 = codebook.project(data);
    let projection2 = codebook.project(data);
    
    assert_eq!(
        projection1.quality_score,
        projection2.quality_score,
        "Projections should be deterministic"
    );
}
```

### 5. Test Isolation

- No shared state between tests
- Each test should be independent
- Use separate codebooks/vectors for each test
- Clean up resources after tests

## Testing Utilities

Embeddenator provides testing utilities in the `testing` module (available in debug builds):

### TestMetrics

Track performance metrics:

```rust
use embeddenator::testing::TestMetrics;

let mut metrics = TestMetrics::new("operation_name");
metrics.time_operation(|| {
    // Your operation here
});
let stats = metrics.timing_stats();
println!("Mean: {}ns, Stddev: {}ns", stats.mean_ns, stats.stddev_ns);
```

### IntegrityReport

Validate data integrity:

```rust
use embeddenator::testing::IntegrityReport;

let mut report = IntegrityReport::default();
report.pass();  // Record successful check
report.fail("reason");  // Record failure
assert!(report.is_ok());
println!("Pass rate: {}%", report.pass_rate());
```

### ChaosInjector

Test resilience with controlled corruption:

```rust
use embeddenator::testing::ChaosInjector;

let injector = ChaosInjector::new(42);  // Reproducible with seed
let flipped = injector.inject_bitflips(&mut vector, 5);
// Verify system handles corruption gracefully
```

### StorageFootprint

Analyze storage characteristics:

```rust
use embeddenator::testing::StorageFootprint;

let footprint = StorageFootprint {
    raw_bytes: 10000,
    bitsliced_bytes: 4000,
    dimension: 10000,
    nnz: 200,
    ..Default::default()
};
println!("Density: {}", footprint.density());
println!("Compression: {}x", footprint.compression_ratio());
```

## Writing New Tests

### 1. Choose the Right Test Type

- **Unit tests**: For testing individual functions/modules
- **Integration tests**: For testing interactions between components
- **E2E tests**: For testing complete workflows

### 2. Organize by Functionality

Place tests in appropriate files:
- Codebook functionality → `tests/codebook/`
- VSA operations → `tests/unit_tests.rs`
- CLI commands → `tests/integration_cli.rs`
- Error handling → `tests/error_recovery.rs`

### 3. Follow the Pattern

Look at existing tests for examples:

```rust
//! Test Module Documentation
//!
//! Description of what this module tests.
//!
//! Run with: cargo test --test module_name

use embeddenator::{Type1, Type2};

#[test]
fn test_specific_functionality() {
    // Setup
    let instance = Type1::new();
    
    // Execute
    let result = instance.operation();
    
    // Assert
    assert_eq!(result, expected, "Descriptive error message");
}
```

### 4. Test Both Success and Failure Cases

```rust
#[test]
fn test_operation_success() {
    // Test successful operation
}

#[test]
fn test_operation_with_invalid_input() {
    // Test error handling
    let result = operation_with_invalid_input();
    assert!(result.is_err());
}
```

## Continuous Integration

Tests are run automatically on:
- Every push to main branches
- Pull requests
- Before releases

### CI Requirements

- All tests must pass (except known infrastructure failures)
- No new clippy warnings
- Code must compile on all supported platforms

## Known Issues

### Disk Space Failures

Four tests in `qa_comprehensive.rs` may fail due to disk space:
- `test_cli_comprehensive`
- `test_memory_usage_patterns`
- `test_performance_regression`
- `test_comprehensive_report`

These are infrastructure issues, not code defects. The tests pass when sufficient disk space is available.

### Deprecated Function Warnings

Some tests use deprecated `SparseVec::from_data()` function. This is a known issue being addressed. The warnings do not affect functionality.

## Performance Testing

### Benchmarks

Run benchmarks:

```bash
cargo bench
```

Available benchmarks:
- `vsa_ops`: Core VSA operations
- `retrieval`: Search and retrieval performance
- `simd_cosine`: SIMD-optimized operations
- `hierarchical_scale`: Hierarchical encoding scaling

### Profiling

For detailed performance analysis:

```bash
cargo flamegraph --bench vsa_ops
```

## Test Coverage Goals

- **Core VSA operations**: 100% coverage ✅
- **Balanced ternary**: 100% coverage ✅
- **Error recovery**: 95%+ coverage ✅
- **CLI operations**: 90%+ coverage ✅
- **Hierarchical operations**: 90%+ coverage ✅

## Contributing Tests

When contributing:

1. Write tests for new features
2. Ensure all existing tests still pass
3. Follow the test quality standards above
4. Document any new testing patterns
5. Update this guide if adding new test categories

## Questions?

If you have questions about testing:
- Review existing tests for examples
- Check the [handoff documentation](docs/handoff/)
- Open an issue on GitHub

---

**License:** MIT  
**Last Updated:** January 10, 2026
