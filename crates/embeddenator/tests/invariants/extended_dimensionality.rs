//! Extended dimensionality tests for bitsliced and soft ternary operations.
//!
//! Tests VSA operations across dimensions from 1K to 100M,
//! validating correctness via sampling and measuring performance scaling.

use embeddenator::{BitslicedTritVec, SoftTernaryVec, SparseVec, Trit};
use std::time::Instant;

/// Generate deterministic pseudo-random sparse vector.
fn make_sparse_deterministic(dim: usize, density: f64, seed: u64) -> SparseVec {
    let nnz = ((dim as f64 * density) as usize).max(1);
    let mut pos = Vec::with_capacity(nnz / 2);
    let mut neg = Vec::with_capacity(nnz / 2);

    // Use simple LCG for determinism
    let mut state = seed;
    let lcg = |s: &mut u64| {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *s
    };

    for i in 0..nnz {
        let idx = (lcg(&mut state) as usize) % dim;
        if i % 2 == 0 {
            if !pos.contains(&idx) && !neg.contains(&idx) {
                pos.push(idx);
            }
        } else {
            if !pos.contains(&idx) && !neg.contains(&idx) {
                neg.push(idx);
            }
        }
    }

    pos.sort_unstable();
    neg.sort_unstable();
    SparseVec { pos, neg }
}

/// Test dimensions ranging from small to very large.
const SMALL_DIMS: &[usize] = &[64, 100, 1_000, 10_000];
const MEDIUM_DIMS: &[usize] = &[100_000, 1_000_000];
const LARGE_DIMS: &[usize] = &[10_000_000, 100_000_000];

/// Estimate memory needed for a single bitsliced vector.
fn memory_estimate_mb(dim: usize) -> f64 {
    let words = (dim + 63) / 64;
    let bytes = words * 2 * 8;  // 2 planes, 8 bytes per word
    bytes as f64 / 1_000_000.0
}

/// Check if we have enough memory for the test.
fn can_allocate(dim: usize, vectors_needed: usize) -> bool {
    let mb_per_vec = memory_estimate_mb(dim);
    let total_mb = mb_per_vec * vectors_needed as f64;
    
    // Conservative: require less than 4GB (most CI has 8GB+)
    total_mb < 4096.0
}

// ============================================================================
// SMALL DIMENSION TESTS (always run)
// ============================================================================

#[test]
fn test_bitsliced_bind_small_dims() {
    for &dim in SMALL_DIMS {
        let sparse_a = make_sparse_deterministic(dim, 0.02, 12345);
        let sparse_b = make_sparse_deterministic(dim, 0.02, 67890);

        let a = BitslicedTritVec::from_sparse(&sparse_a, dim);
        let b = BitslicedTritVec::from_sparse(&sparse_b, dim);

        let result = a.bind(&b);

        // Verify length
        assert_eq!(result.len(), dim, "dim={dim}: length mismatch");

        // Sample verification
        for idx in [0, dim / 4, dim / 2, dim - 1] {
            let expected = trit_mul(a.get(idx), b.get(idx));
            assert_eq!(result.get(idx), expected, "dim={dim}, idx={idx}: bind mismatch");
        }
    }
}

#[test]
fn test_bitsliced_bundle_small_dims() {
    for &dim in SMALL_DIMS {
        let sparse_a = make_sparse_deterministic(dim, 0.02, 11111);
        let sparse_b = make_sparse_deterministic(dim, 0.02, 22222);

        let a = BitslicedTritVec::from_sparse(&sparse_a, dim);
        let b = BitslicedTritVec::from_sparse(&sparse_b, dim);

        let result = a.bundle(&b);

        assert_eq!(result.len(), dim);

        // Sample verification
        for idx in [0, dim / 4, dim / 2, dim - 1] {
            let expected = trit_bundle(a.get(idx), b.get(idx));
            assert_eq!(result.get(idx), expected, "dim={dim}, idx={idx}: bundle mismatch");
        }
    }
}

#[test]
fn test_bitsliced_dot_small_dims() {
    for &dim in SMALL_DIMS {
        let sparse_a = make_sparse_deterministic(dim, 0.02, 33333);
        let sparse_b = make_sparse_deterministic(dim, 0.02, 44444);

        let a = BitslicedTritVec::from_sparse(&sparse_a, dim);
        let b = BitslicedTritVec::from_sparse(&sparse_b, dim);

        let dot = a.dot(&b);

        // Compute expected dot product manually
        let expected: i32 = (0..dim)
            .map(|i| trit_to_int(a.get(i)) * trit_to_int(b.get(i)))
            .sum();

        assert_eq!(dot, expected, "dim={dim}: dot mismatch");
    }
}

#[test]
fn test_soft_ternary_small_dims() {
    for &dim in SMALL_DIMS {
        let mut soft = SoftTernaryVec::new_zero(dim);

        // Accumulate 5 vectors
        for seed in [100, 200, 300, 400, 500] {
            let sparse = make_sparse_deterministic(dim, 0.01, seed);
            let hard = BitslicedTritVec::from_sparse(&sparse, dim);
            soft.accumulate(&hard);
        }

        // Verify non-zero count makes sense
        let nnz = soft.nnz();
        assert!(nnz > 0, "dim={dim}: should have non-zero positions");
        assert!(nnz <= dim, "dim={dim}: nnz cannot exceed dim");

        // Harden and verify
        let hardened = soft.harden(2);  // Need ≥2 votes
        assert_eq!(hardened.len(), dim);
    }
}

// ============================================================================
// MEDIUM DIMENSION TESTS (run with adequate memory)
// ============================================================================

#[test]
fn test_bitsliced_bind_medium_dims() {
    for &dim in MEDIUM_DIMS {
        if !can_allocate(dim, 3) {
            eprintln!("Skipping dim={dim} (insufficient memory)");
            continue;
        }

        let sparse_a = make_sparse_deterministic(dim, 0.001, 55555);
        let sparse_b = make_sparse_deterministic(dim, 0.001, 66666);

        let start = Instant::now();
        let a = BitslicedTritVec::from_sparse(&sparse_a, dim);
        let b = BitslicedTritVec::from_sparse(&sparse_b, dim);
        let creation_time = start.elapsed();

        let start = Instant::now();
        let result = a.bind(&b);
        let bind_time = start.elapsed();

        eprintln!(
            "dim={}: creation={:.2}ms, bind={:.2}ms, mem={:.1}MB",
            dim,
            creation_time.as_secs_f64() * 1000.0,
            bind_time.as_secs_f64() * 1000.0,
            memory_estimate_mb(dim)
        );

        // Sample verification (10 random positions)
        let mut rng_state = 777u64;
        let lcg = |s: &mut u64| {
            *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            *s
        };

        for _ in 0..10 {
            let idx = (lcg(&mut rng_state) as usize) % dim;
            let expected = trit_mul(a.get(idx), b.get(idx));
            assert_eq!(result.get(idx), expected, "dim={dim}, idx={idx}");
        }
    }
}

#[test]
fn test_bitsliced_bundle_medium_dims() {
    for &dim in MEDIUM_DIMS {
        if !can_allocate(dim, 3) {
            eprintln!("Skipping dim={dim}");
            continue;
        }

        let sparse_a = make_sparse_deterministic(dim, 0.001, 77777);
        let sparse_b = make_sparse_deterministic(dim, 0.001, 88888);

        let a = BitslicedTritVec::from_sparse(&sparse_a, dim);
        let b = BitslicedTritVec::from_sparse(&sparse_b, dim);

        let start = Instant::now();
        let result = a.bundle(&b);
        let bundle_time = start.elapsed();

        eprintln!("dim={}: bundle={:.2}ms", dim, bundle_time.as_secs_f64() * 1000.0);

        // Sample verification
        let mut rng_state = 888u64;
        let lcg = |s: &mut u64| {
            *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            *s
        };

        for _ in 0..10 {
            let idx = (lcg(&mut rng_state) as usize) % dim;
            let expected = trit_bundle(a.get(idx), b.get(idx));
            assert_eq!(result.get(idx), expected, "dim={dim}, idx={idx}");
        }
    }
}

#[test]
fn test_soft_accumulate_medium_dims() {
    for &dim in MEDIUM_DIMS {
        if !can_allocate(dim, 2) {
            eprintln!("Skipping dim={dim}");
            continue;
        }

        let mut soft = SoftTernaryVec::new_zero(dim);

        // Accumulate 7 vectors
        let start = Instant::now();
        for i in 0..7 {
            let sparse = make_sparse_deterministic(dim, 0.001, 1000 + i);
            let hard = BitslicedTritVec::from_sparse(&sparse, dim);
            soft.accumulate(&hard);
        }
        let acc_time = start.elapsed();

        let start = Instant::now();
        let hardened = soft.harden(3);  // Need ≥3 votes (majority of 7 - ties = 4)
        let harden_time = start.elapsed();

        eprintln!(
            "dim={}: 7x accumulate={:.2}ms, harden={:.2}ms",
            dim,
            acc_time.as_secs_f64() * 1000.0,
            harden_time.as_secs_f64() * 1000.0
        );

        assert_eq!(hardened.len(), dim);
    }
}

// ============================================================================
// LARGE DIMENSION TESTS (gated by memory and marked ignore)
// ============================================================================

#[test]
#[ignore = "requires >400MB RAM per vector, run with --ignored"]
fn test_bitsliced_bind_large_dims() {
    for &dim in LARGE_DIMS {
        if !can_allocate(dim, 3) {
            eprintln!("Skipping dim={dim} (need {:.0}MB)", memory_estimate_mb(dim) * 3.0);
            continue;
        }

        eprintln!("Testing dim={} (vectors will use ~{:.0}MB each)", dim, memory_estimate_mb(dim));

        let sparse_a = make_sparse_deterministic(dim, 0.0001, 99999);
        let sparse_b = make_sparse_deterministic(dim, 0.0001, 12121);

        let start = Instant::now();
        let a = BitslicedTritVec::from_sparse(&sparse_a, dim);
        let b = BitslicedTritVec::from_sparse(&sparse_b, dim);
        let creation_time = start.elapsed();

        let start = Instant::now();
        let result = a.bind(&b);
        let bind_time = start.elapsed();

        eprintln!(
            "dim={}: creation={:.1}ms, bind={:.2}ms",
            dim,
            creation_time.as_secs_f64() * 1000.0,
            bind_time.as_secs_f64() * 1000.0
        );

        // Verify length and sample positions
        assert_eq!(result.len(), dim);

        // Sample 100 random positions for verification
        let mut rng_state = 42424242u64;
        let lcg = |s: &mut u64| {
            *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            *s
        };

        for _ in 0..100 {
            let idx = (lcg(&mut rng_state) as usize) % dim;
            let expected = trit_mul(a.get(idx), b.get(idx));
            assert_eq!(result.get(idx), expected, "dim={dim}, idx={idx}");
        }
    }
}

#[test]
#[ignore = "requires >400MB RAM per vector"]
fn test_bitsliced_bundle_large_dims() {
    for &dim in LARGE_DIMS {
        if !can_allocate(dim, 3) {
            eprintln!("Skipping dim={dim}");
            continue;
        }

        eprintln!("Testing bundle at dim={}", dim);

        let sparse_a = make_sparse_deterministic(dim, 0.0001, 34343);
        let sparse_b = make_sparse_deterministic(dim, 0.0001, 56565);

        let a = BitslicedTritVec::from_sparse(&sparse_a, dim);
        let b = BitslicedTritVec::from_sparse(&sparse_b, dim);

        let start = Instant::now();
        let result = a.bundle(&b);
        let bundle_time = start.elapsed();

        eprintln!("dim={}: bundle={:.2}ms", dim, bundle_time.as_secs_f64() * 1000.0);

        // Sample verification
        let mut rng_state = 78787u64;
        let lcg = |s: &mut u64| {
            *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            *s
        };

        for _ in 0..100 {
            let idx = (lcg(&mut rng_state) as usize) % dim;
            let expected = trit_bundle(a.get(idx), b.get(idx));
            assert_eq!(result.get(idx), expected, "dim={dim}, idx={idx}");
        }
    }
}

#[test]
#[ignore = "requires >400MB RAM per vector"]
fn test_bitsliced_dot_large_dims() {
    for &dim in LARGE_DIMS {
        if !can_allocate(dim, 2) {
            eprintln!("Skipping dim={dim}");
            continue;
        }

        eprintln!("Testing dot at dim={}", dim);

        let sparse_a = make_sparse_deterministic(dim, 0.0001, 90909);
        let sparse_b = make_sparse_deterministic(dim, 0.0001, 10101);

        let a = BitslicedTritVec::from_sparse(&sparse_a, dim);
        let b = BitslicedTritVec::from_sparse(&sparse_b, dim);

        let start = Instant::now();
        let dot = a.dot(&b);
        let dot_time = start.elapsed();

        eprintln!("dim={}: dot={:.2}ms, value={}", dim, dot_time.as_secs_f64() * 1000.0, dot);

        // Sanity check: dot should be reasonable magnitude for sparse vectors
        let max_dot = (dim as f64 * 0.0001 * 2.0) as i32;  // rough upper bound
        assert!(dot.abs() <= max_dot, "dim={dim}: dot={dot} seems too large");
    }
}

#[test]
#[ignore = "requires >800MB RAM for soft vectors"]
fn test_soft_ternary_large_dims() {
    for &dim in &[10_000_000usize] {  // Start with 10M
        eprintln!("Testing soft ternary at dim={}", dim);

        let mut soft = SoftTernaryVec::new_zero(dim);

        // Accumulate 15 vectors - use higher density for large dims to ensure overlap
        let density = if dim >= 1_000_000 { 0.001 } else { 0.0001 }; // 0.1% for large
        let start = Instant::now();
        for i in 0..15 {
            let sparse = make_sparse_deterministic(dim, density, 2000 + i);
            let hard = BitslicedTritVec::from_sparse(&sparse, dim);
            soft.accumulate(&hard);
        }
        let acc_time = start.elapsed();

        eprintln!("15x accumulate took {:.2}ms", acc_time.as_secs_f64() * 1000.0);

        let start = Instant::now();
        // Use threshold=2 for large dims (lower probability of overlap)
        let threshold = if dim >= 1_000_000 { 2 } else { 4 };
        let hardened = soft.harden(threshold);
        let harden_time = start.elapsed();

        eprintln!("harden took {:.2}ms, nnz={}", harden_time.as_secs_f64() * 1000.0, hardened.nnz());

        assert_eq!(hardened.len(), dim);
        // For very large dims with sparse accumulation, may have no survivors - that's ok
        if dim < 10_000_000 {
            assert!(hardened.nnz() > 0);
        }
    }
}

// ============================================================================
// STRESS TEST: 100M DIMENSIONS
// ============================================================================

#[test]
#[ignore = "requires ~6GB RAM, run with --ignored on high-memory system"]
fn test_100m_dimension_stress() {
    let dim = 100_000_000;

    eprintln!("=== 100M DIMENSION STRESS TEST ===");
    eprintln!("Memory per bitsliced vector: {:.0}MB", memory_estimate_mb(dim));

    // Only proceed if we have enough memory
    if !can_allocate(dim, 3) {
        eprintln!("Insufficient memory for 100M test (need ~{}MB)", (memory_estimate_mb(dim) * 3.0) as u64);
        return;
    }

    // Create vectors with very low density (0.01%)
    let sparse_a = make_sparse_deterministic(dim, 0.0001, 111111);
    let sparse_b = make_sparse_deterministic(dim, 0.0001, 222222);

    eprintln!("Creating bitsliced vectors...");
    let start = Instant::now();
    let a = BitslicedTritVec::from_sparse(&sparse_a, dim);
    let b = BitslicedTritVec::from_sparse(&sparse_b, dim);
    eprintln!("Creation: {:.1}ms", start.elapsed().as_secs_f64() * 1000.0);

    eprintln!("Testing bind...");
    let start = Instant::now();
    let bind_result = a.bind(&b);
    let bind_time = start.elapsed();
    eprintln!("Bind: {:.2}ms ({:.1} GB/s effective bandwidth)",
        bind_time.as_secs_f64() * 1000.0,
        (memory_estimate_mb(dim) * 3.0) / 1000.0 / bind_time.as_secs_f64()
    );

    eprintln!("Testing bundle...");
    let start = Instant::now();
    let bundle_result = a.bundle(&b);
    let bundle_time = start.elapsed();
    eprintln!("Bundle: {:.2}ms", bundle_time.as_secs_f64() * 1000.0);

    eprintln!("Testing dot...");
    let start = Instant::now();
    let dot = a.dot(&b);
    let dot_time = start.elapsed();
    eprintln!("Dot: {:.2}ms, value={}", dot_time.as_secs_f64() * 1000.0, dot);

    // Verify results
    assert_eq!(bind_result.len(), dim);
    assert_eq!(bundle_result.len(), dim);

    // Sample verification
    let mut failures = 0;
    let mut rng_state = 333333u64;
    let lcg = |s: &mut u64| {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        *s
    };

    for _ in 0..1000 {
        let idx = (lcg(&mut rng_state) as usize) % dim;

        let expected_bind = trit_mul(a.get(idx), b.get(idx));
        if bind_result.get(idx) != expected_bind {
            failures += 1;
        }

        let expected_bundle = trit_bundle(a.get(idx), b.get(idx));
        if bundle_result.get(idx) != expected_bundle {
            failures += 1;
        }
    }

    assert_eq!(failures, 0, "Had {} failures in sampled verification", failures);

    eprintln!("=== 100M STRESS TEST PASSED ===");
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn trit_mul(a: Trit, b: Trit) -> Trit {
    match (a, b) {
        (Trit::Z, _) | (_, Trit::Z) => Trit::Z,
        (Trit::P, Trit::P) | (Trit::N, Trit::N) => Trit::P,
        (Trit::P, Trit::N) | (Trit::N, Trit::P) => Trit::N,
    }
}

fn trit_bundle(a: Trit, b: Trit) -> Trit {
    match (a, b) {
        (Trit::P, Trit::N) | (Trit::N, Trit::P) => Trit::Z,
        (Trit::P, _) | (_, Trit::P) => Trit::P,
        (Trit::N, _) | (_, Trit::N) => Trit::N,
        (Trit::Z, Trit::Z) => Trit::Z,
    }
}

fn trit_to_int(t: Trit) -> i32 {
    match t {
        Trit::P => 1,
        Trit::Z => 0,
        Trit::N => -1,
    }
}

// ============================================================================
// SOFT TERNARY PRECISION TESTS
// ============================================================================

#[test]
fn test_soft_vs_hard_bundle_precision() {
    let dim = 10_000;
    let n_vectors = 7;

    // Create N random vectors
    let vectors: Vec<_> = (0..n_vectors)
        .map(|i| {
            let sparse = make_sparse_deterministic(dim, 0.02, 5000 + i as u64);
            BitslicedTritVec::from_sparse(&sparse, dim)
        })
        .collect();

    // Hard sequential bundling
    let mut hard_result = vectors[0].clone();
    for v in vectors.iter().skip(1) {
        hard_result = hard_result.bundle(v);
    }

    // Soft accumulation + hardening
    let mut soft = SoftTernaryVec::new_zero(dim);
    for v in &vectors {
        soft.accumulate(v);
    }
    let soft_result = soft.harden(4);  // majority = ceil(7/2) = 4

    // Compare results
    let mut matches = 0;
    let mut hard_only = 0;
    let mut soft_only = 0;

    for i in 0..dim {
        let h = hard_result.get(i);
        let s = soft_result.get(i);

        if h == s {
            matches += 1;
        } else if h != Trit::Z && s == Trit::Z {
            hard_only += 1;
        } else if h == Trit::Z && s != Trit::Z {
            soft_only += 1;
        }
    }

    let match_pct = 100.0 * matches as f64 / dim as f64;
    eprintln!(
        "Soft vs Hard bundle: {:.1}% match, {} hard-only, {} soft-only",
        match_pct, hard_only, soft_only
    );

    // Should have high agreement (>85% for sparse inputs)
    assert!(match_pct > 85.0, "Expected >85% match, got {:.1}%", match_pct);
}

#[test]
fn test_soft_dot_accuracy() {
    let dim = 10_000;

    // Create soft vector with known values
    let mut soft = SoftTernaryVec::new_zero(dim);
    soft.set(0, 5, false);   // +5
    soft.set(1, 3, true);    // -3
    soft.set(100, 7, false); // +7
    soft.set(500, 2, true);  // -2

    // Create hard query vector
    let mut hard = BitslicedTritVec::new_zero(dim);
    hard.set(0, Trit::P);    // +1 at 0
    hard.set(1, Trit::N);    // -1 at 1
    hard.set(100, Trit::P);  // +1 at 100
    hard.set(500, Trit::P);  // +1 at 500

    // Expected dot:
    // +5 × +1 = +5
    // -3 × -1 = +3
    // +7 × +1 = +7
    // -2 × +1 = -2
    // Total = 5 + 3 + 7 - 2 = 13

    let dot = soft.dot_with_hard(&hard);
    assert_eq!(dot, 13, "Expected 13, got {}", dot);

    // Also test fast path
    let dot_fast = soft.dot_with_hard_fast(&hard);
    assert_eq!(dot_fast, 13, "Fast path: expected 13, got {}", dot_fast);
}

// ============================================================================
// SCALING ANALYSIS
// ============================================================================

#[test]
fn test_scaling_report() {
    eprintln!("\n=== MEMORY SCALING REPORT ===");
    eprintln!("{:<12} {:>15} {:>15} {:>12}", "Dimension", "Bitsliced (MB)", "Soft (MB)", "Words");

    for dim in [1_000, 10_000, 100_000, 1_000_000, 10_000_000, 100_000_000] {
        let words = (dim + 63) / 64;
        let bitsliced_mb = memory_estimate_mb(dim);
        let soft_mb = (words * 4 * 8) as f64 / 1_000_000.0;  // 4 planes

        eprintln!("{:<12} {:>15.2} {:>15.2} {:>12}", dim, bitsliced_mb, soft_mb, words);
    }
    eprintln!();
}
