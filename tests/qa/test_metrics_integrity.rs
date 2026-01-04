//! Test Metrics and Integrity Validation Suite
//!
//! This test module validates the testing infrastructure itself
//! and provides comprehensive integrity checks for VSA operations.
//!
//! Run with: cargo test --test test_metrics_integrity

use embeddenator::bitsliced::BitslicedTritVec;
use embeddenator::hybrid::HybridTritVec;
use embeddenator::vsa::SparseVec;

/// Generate a reproducible random sparse vector
pub fn sparse_random(dim: usize, nnz: usize, seed: u64) -> SparseVec {
    // Split nnz roughly evenly between pos and neg
    let pos_count = nnz / 2;
    let neg_count = nnz - pos_count;

    let mut state = seed;
    let lcg = |s: &mut u64| -> u64 {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        *s
    };

    let mut pos = Vec::with_capacity(pos_count);
    let mut neg = Vec::with_capacity(neg_count);
    let mut used = std::collections::HashSet::new();

    for _ in 0..pos_count {
        loop {
            let idx = (lcg(&mut state) as usize) % dim;
            if used.insert(idx) {
                pos.push(idx);
                break;
            }
        }
    }

    for _ in 0..neg_count {
        loop {
            let idx = (lcg(&mut state) as usize) % dim;
            if used.insert(idx) {
                neg.push(idx);
                break;
            }
        }
    }

    pos.sort_unstable();
    neg.sort_unstable();

    SparseVec { pos, neg }
}

// ============================================================================
// INLINE TEST UTILITIES (since testing module is cfg(test) only in lib)
// ============================================================================

/// Simple performance timer
struct Timer {
    name: String,
    start: std::time::Instant,
    samples: Vec<u64>,
}

impl Timer {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start: std::time::Instant::now(),
            samples: Vec::new(),
        }
    }

    fn lap(&mut self) {
        self.samples.push(self.start.elapsed().as_nanos() as u64);
        self.start = std::time::Instant::now();
    }

    fn summary(&self) -> String {
        if self.samples.is_empty() {
            return format!("{}: no samples", self.name);
        }
        let sum: u64 = self.samples.iter().sum();
        let mean = sum as f64 / self.samples.len() as f64;
        let min = *self.samples.iter().min().unwrap();
        let max = *self.samples.iter().max().unwrap();
        format!(
            "{}: {} samples, mean={:.2}µs, min={:.2}µs, max={:.2}µs",
            self.name,
            self.samples.len(),
            mean / 1000.0,
            min as f64 / 1000.0,
            max as f64 / 1000.0
        )
    }
}

/// Storage footprint calculator
fn calc_sparse_storage(sparse: &SparseVec, dim: usize) -> (usize, f64) {
    let nnz = sparse.pos.len() + sparse.neg.len();
    let bytes = nnz * std::mem::size_of::<usize>() * 2;
    let density = nnz as f64 / dim as f64;
    (bytes, density)
}

fn calc_bitsliced_storage(dim: usize) -> usize {
    let words = (dim + 63) / 64;
    words * 2 * std::mem::size_of::<u64>()
}

fn bits_per_trit_bitsliced() -> f64 {
    2.0 // Exactly 2 bits per trit (pos plane + neg plane)
}

// ============================================================================
// INTEGRITY CHECKS
// ============================================================================

/// Verify bitsliced vector has no overlapping pos/neg bits
fn verify_no_overlap(bs: &BitslicedTritVec) -> Result<(), String> {
    let words = (bs.len() + 63) / 64;
    for w in 0..words {
        let overlap = bs.pos_word(w) & bs.neg_word(w);
        if overlap != 0 {
            return Err(format!(
                "Word {} has {} positions with both pos and neg set (overlap={:016x})",
                w,
                overlap.count_ones(),
                overlap
            ));
        }
    }
    Ok(())
}

/// Verify trailing bits are zero
fn verify_trailing_zeros(bs: &BitslicedTritVec) -> Result<(), String> {
    let words = (bs.len() + 63) / 64;
    if words == 0 {
        return Ok(());
    }
    let trailing = bs.len() % 64;
    if trailing != 0 {
        let mask = !((1u64 << trailing) - 1);
        let pos_trail = bs.pos_word(words - 1) & mask;
        let neg_trail = bs.neg_word(words - 1) & mask;
        if pos_trail != 0 || neg_trail != 0 {
            return Err(format!(
                "Trailing bits not zero: pos={:016x}, neg={:016x}",
                pos_trail, neg_trail
            ));
        }
    }
    Ok(())
}

/// Count bit differences between two vectors
fn count_bit_differences(a: &BitslicedTritVec, b: &BitslicedTritVec) -> (u64, u64) {
    assert_eq!(a.len(), b.len());
    let words = (a.len() + 63) / 64;
    let mut pos_diffs = 0u64;
    let mut neg_diffs = 0u64;
    for w in 0..words {
        pos_diffs += (a.pos_word(w) ^ b.pos_word(w)).count_ones() as u64;
        neg_diffs += (a.neg_word(w) ^ b.neg_word(w)).count_ones() as u64;
    }
    (pos_diffs, neg_diffs)
}

// ============================================================================
// ALGEBRAIC INVARIANT TESTS
// ============================================================================

#[test]
fn test_bind_self_inverse_property() {
    println!("\n=== Bind Self-Inverse Test ===");
    let mut timer = Timer::new("bind_self_inverse");

    for seed in 0..20 {
        let dim = 10000;
        let sparse = sparse_random(dim, 100, seed);
        let a = BitslicedTritVec::from_sparse(&sparse, dim);

        timer.lap();
        let a_squared = a.bind(&a);
        timer.lap();

        // A ⊙ A should have all +1 at non-zero positions
        let original_nnz = a.nnz();
        let squared_pos = a_squared.to_sparse().pos.len();
        let squared_neg = a_squared.to_sparse().neg.len();

        assert_eq!(
            squared_neg, 0,
            "seed {}: A⊙A has {} negative trits (should be 0)",
            seed, squared_neg
        );
        assert_eq!(
            squared_pos, original_nnz,
            "seed {}: A⊙A has {} positive trits (expected {})",
            seed, squared_pos, original_nnz
        );
    }

    println!("{}", timer.summary());
    println!("✓ All 20 self-inverse tests passed");
}

#[test]
fn test_bind_commutativity() {
    println!("\n=== Bind Commutativity Test ===");
    let mut timer = Timer::new("bind_commutative");

    for seed in 0..20 {
        let dim = 10000;
        let a = BitslicedTritVec::from_sparse(&sparse_random(dim, 100, seed), dim);
        let b = BitslicedTritVec::from_sparse(&sparse_random(dim, 100, seed + 100), dim);

        timer.lap();
        let ab = a.bind(&b);
        let ba = b.bind(&a);
        timer.lap();

        let (pos_diff, neg_diff) = count_bit_differences(&ab, &ba);
        assert_eq!(
            pos_diff + neg_diff,
            0,
            "seed {}: A⊙B ≠ B⊙A, {} bit differences",
            seed,
            pos_diff + neg_diff
        );
    }

    println!("{}", timer.summary());
    println!("✓ All 20 commutativity tests passed");
}

#[test]
fn test_bundle_commutativity() {
    println!("\n=== Bundle Commutativity Test ===");

    for seed in 0..20 {
        let dim = 10000;
        let a = BitslicedTritVec::from_sparse(&sparse_random(dim, 100, seed), dim);
        let b = BitslicedTritVec::from_sparse(&sparse_random(dim, 100, seed + 100), dim);

        let ab = a.bundle(&b);
        let ba = b.bundle(&a);

        let (pos_diff, neg_diff) = count_bit_differences(&ab, &ba);
        assert_eq!(
            pos_diff + neg_diff,
            0,
            "seed {}: A⊕B ≠ B⊕A, {} bit differences",
            seed,
            pos_diff + neg_diff
        );
    }

    println!("✓ All 20 bundle commutativity tests passed");
}

#[test]
fn test_bundle_conflict_cancellation() {
    println!("\n=== Bundle Conflict Cancellation Test ===");

    let dim = 1000;

    // Create vectors with known overlapping positions
    let sparse_a = SparseVec {
        pos: vec![0, 10, 20, 30],
        neg: vec![5, 15],
    };
    let sparse_b = SparseVec {
        pos: vec![5, 25], // 5 conflicts with A's neg
        neg: vec![10],    // 10 conflicts with A's pos
    };

    let a = BitslicedTritVec::from_sparse(&sparse_a, dim);
    let b = BitslicedTritVec::from_sparse(&sparse_b, dim);
    let bundled = a.bundle(&b);

    // Position 5: A has N, B has P → should be Z
    assert_eq!(
        bundled.get(5),
        embeddenator::ternary::Trit::Z,
        "Position 5 should cancel to Z"
    );

    // Position 10: A has P, B has N → should be Z
    assert_eq!(
        bundled.get(10),
        embeddenator::ternary::Trit::Z,
        "Position 10 should cancel to Z"
    );

    // Position 0: only A has P → should be P
    assert_eq!(
        bundled.get(0),
        embeddenator::ternary::Trit::P,
        "Position 0 should be P"
    );

    println!("✓ Conflict cancellation works correctly");
}

// ============================================================================
// STORAGE FOOTPRINT TESTS
// ============================================================================

#[test]
fn test_storage_footprint_calculations() {
    println!("\n=== Storage Footprint Test ===");

    let test_cases = vec![
        (10_000, 50, "very sparse 0.5%"),
        (10_000, 500, "moderate 5%"),
        (100_000, 500, "large sparse 0.5%"),
        (100_000, 5000, "large moderate 5%"),
        (1_000_000, 5000, "huge sparse 0.5%"),
    ];

    println!(
        "{:<25} {:>12} {:>12} {:>12} {:>8}",
        "Case", "Sparse", "Bitsliced", "Ratio", "Density"
    );
    println!("{:-<70}", "");

    for (dim, nnz, label) in test_cases {
        let sparse = sparse_random(dim, nnz, 42);
        let (sparse_bytes, density) = calc_sparse_storage(&sparse, dim);
        let bitsliced_bytes = calc_bitsliced_storage(dim);

        let ratio = bitsliced_bytes as f64 / sparse_bytes as f64;

        println!(
            "{:<25} {:>10}KB {:>10}KB {:>10.2}x {:>7.2}%",
            label,
            sparse_bytes / 1024,
            bitsliced_bytes / 1024,
            ratio,
            density * 100.0
        );

        // Verify calculations
        assert_eq!(sparse_bytes, (sparse.pos.len() + sparse.neg.len()) * std::mem::size_of::<usize>() * 2);
        assert_eq!(bitsliced_bytes, ((dim + 63) / 64) * 2 * 8);
    }

    println!("\nBits per trit (bitsliced): {:.1}", bits_per_trit_bitsliced());
    println!("✓ Storage calculations verified");
}

// ============================================================================
// BITFLIP DETECTION TESTS
// ============================================================================

/// Simple LCG for reproducible random positions
fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    *state
}

#[test]
fn test_bitflip_detection_single() {
    println!("\n=== Single Bitflip Detection Test ===");

    let dim = 10000;
    let original = BitslicedTritVec::from_sparse(&sparse_random(dim, 500, 42), dim);

    // Inject single bitflip at known position
    let mut corrupted = original.clone();
    let flip_pos = 1234;
    let original_trit = corrupted.get(flip_pos);
    let new_trit = match original_trit {
        embeddenator::ternary::Trit::P => embeddenator::ternary::Trit::N,
        embeddenator::ternary::Trit::N => embeddenator::ternary::Trit::P,
        embeddenator::ternary::Trit::Z => embeddenator::ternary::Trit::P,
    };
    corrupted.set(flip_pos, new_trit);

    let (pos_diff, neg_diff) = count_bit_differences(&original, &corrupted);
    println!(
        "Injected flip at {}: {:?} → {:?}",
        flip_pos, original_trit, new_trit
    );
    println!("Detected: {} pos bits, {} neg bits changed", pos_diff, neg_diff);

    assert!(
        pos_diff + neg_diff >= 1,
        "Failed to detect single bitflip"
    );
    println!("✓ Single bitflip detection works");
}

#[test]
fn test_bitflip_detection_multiple() {
    println!("\n=== Multiple Bitflip Detection Test ===");

    let dim = 10000;
    let original = BitslicedTritVec::from_sparse(&sparse_random(dim, 500, 42), dim);

    for num_flips in [1, 5, 10, 50, 100] {
        let mut corrupted = original.clone();
        let mut state = 12345u64;

        for _ in 0..num_flips {
            let pos = (lcg_next(&mut state) as usize) % dim;
            let current = corrupted.get(pos);
            let new_trit = match current {
                embeddenator::ternary::Trit::P => embeddenator::ternary::Trit::N,
                embeddenator::ternary::Trit::N => embeddenator::ternary::Trit::P,
                embeddenator::ternary::Trit::Z => embeddenator::ternary::Trit::P,
            };
            corrupted.set(pos, new_trit);
        }

        let (pos_diff, neg_diff) = count_bit_differences(&original, &corrupted);
        let total_diff = pos_diff + neg_diff;
        let cosine = original.cosine(&corrupted);

        println!(
            "Injected {} flips: detected {} bit changes, cosine={:.4}",
            num_flips, total_diff, cosine
        );

        assert!(
            total_diff >= 1,
            "Failed to detect any of {} injected flips",
            num_flips
        );
    }

    println!("✓ Multiple bitflip detection works");
}

// ============================================================================
// INTERNAL CONSISTENCY TESTS
// ============================================================================

#[test]
fn test_bitsliced_internal_consistency_various_dims() {
    println!("\n=== Bitsliced Internal Consistency Test ===");

    let dims = vec![
        1, 2, 31, 32, 33, 63, 64, 65, 127, 128, 129, 256, 1000, 10000,
    ];

    for &dim in &dims {
        let nnz = (dim / 10).max(1);
        let sparse = sparse_random(dim, nnz, 42);
        let bs = BitslicedTritVec::from_sparse(&sparse, dim);

        // Verify no overlap
        verify_no_overlap(&bs).unwrap_or_else(|e| panic!("dim={}: {}", dim, e));

        // Verify trailing zeros
        verify_trailing_zeros(&bs).unwrap_or_else(|e| panic!("dim={}: {}", dim, e));

        // Verify nnz
        assert_eq!(
            bs.nnz(),
            sparse.pos.len() + sparse.neg.len(),
            "dim={}: nnz mismatch",
            dim
        );

        // Verify roundtrip
        let rt = bs.to_sparse();
        assert_eq!(
            rt.pos.len() + rt.neg.len(),
            sparse.pos.len() + sparse.neg.len(),
            "dim={}: roundtrip nnz mismatch",
            dim
        );
    }

    println!("✓ All {} dimensions verified", dims.len());
}

#[test]
fn test_permute_preserves_integrity() {
    println!("\n=== Permute Integrity Test ===");

    let dim = 1024; // 64-aligned for optimized path
    let original = BitslicedTritVec::from_sparse(&sparse_random(dim, 100, 42), dim);
    let original_nnz = original.nnz();

    verify_no_overlap(&original).expect("Original has overlap");
    verify_trailing_zeros(&original).expect("Original has trailing bits");

    for shift in [0, 1, 32, 63, 64, 65, 100, 512, 1023] {
        let permuted = original.permute_optimized(shift);

        // NNZ preserved
        assert_eq!(
            permuted.nnz(),
            original_nnz,
            "shift={}: nnz changed {} → {}",
            shift,
            original_nnz,
            permuted.nnz()
        );

        // No overlap
        verify_no_overlap(&permuted)
            .unwrap_or_else(|e| panic!("shift={}: {}", shift, e));

        // Trailing zeros
        verify_trailing_zeros(&permuted)
            .unwrap_or_else(|e| panic!("shift={}: {}", shift, e));

        // Inverse restores original
        let restored = permuted.permute_optimized(dim - shift);
        let (pos_diff, neg_diff) = count_bit_differences(&original, &restored);
        assert_eq!(
            pos_diff + neg_diff,
            0,
            "shift={}: inverse permute failed, {} bit diffs",
            shift,
            pos_diff + neg_diff
        );
    }

    println!("✓ All permute shifts verified");
}

// ============================================================================
// HYBRID REPRESENTATION TESTS
// ============================================================================

#[test]
fn test_hybrid_representation_selection() {
    println!("\n=== Hybrid Representation Selection Test ===");

    let dim = 10000;

    // Very sparse - should stay sparse
    let very_sparse = sparse_random(dim, 10, 1);
    let hybrid_sparse = HybridTritVec::from_sparse(very_sparse, dim);
    assert!(
        hybrid_sparse.is_sparse(),
        "Very sparse (nnz=10) should stay sparse"
    );
    println!(
        "nnz=10 ({}%): is_sparse={}",
        10.0 / dim as f64 * 100.0,
        hybrid_sparse.is_sparse()
    );

    // Moderate - should convert to bitsliced
    let moderate = sparse_random(dim, 200, 2);
    let hybrid_moderate = HybridTritVec::from_sparse(moderate, dim);
    assert!(
        !hybrid_moderate.is_sparse(),
        "Moderate density (nnz=200) should be bitsliced"
    );
    println!(
        "nnz=200 ({}%): is_sparse={}",
        200.0 / dim as f64 * 100.0,
        hybrid_moderate.is_sparse()
    );

    // Edge case at threshold
    let threshold_nnz = (dim as f64 * embeddenator::hybrid::DENSITY_THRESHOLD) as usize;
    let edge = sparse_random(dim, threshold_nnz, 3);
    let hybrid_edge = HybridTritVec::from_sparse(edge, dim);
    println!(
        "nnz={} ({}%): is_sparse={}",
        threshold_nnz,
        threshold_nnz as f64 / dim as f64 * 100.0,
        hybrid_edge.is_sparse()
    );

    println!("✓ Hybrid selection works correctly");
}

#[test]
fn test_hybrid_cross_representation_ops() {
    println!("\n=== Hybrid Cross-Representation Operations Test ===");

    let dim = 10000;

    // Create one sparse and one bitsliced hybrid
    let sparse_hybrid = HybridTritVec::from_sparse(sparse_random(dim, 10, 1), dim);
    let dense_hybrid = HybridTritVec::from_sparse(sparse_random(dim, 300, 2), dim);

    assert!(sparse_hybrid.is_sparse());
    assert!(!dense_hybrid.is_sparse());

    // Bind - need to pass dim
    let bound = sparse_hybrid.bind(&dense_hybrid, dim);
    println!(
        "Bind: sparse×dense → nnz={}, is_sparse={}",
        bound.nnz(dim),
        bound.is_sparse()
    );

    // Bundle
    let bundled = sparse_hybrid.bundle(&dense_hybrid, dim);
    println!(
        "Bundle: sparse×dense → nnz={}, is_sparse={}",
        bundled.nnz(dim),
        bundled.is_sparse()
    );

    // Cosine
    let cos = sparse_hybrid.cosine(&dense_hybrid, dim);
    println!("Cosine(sparse, dense) = {:.4}", cos);

    // Verify commutativity
    let bound_rev = dense_hybrid.bind(&sparse_hybrid, dim);
    let bundled_rev = dense_hybrid.bundle(&sparse_hybrid, dim);

    assert_eq!(bound.nnz(dim), bound_rev.nnz(dim), "Bind not commutative by nnz");
    assert_eq!(
        bundled.nnz(dim),
        bundled_rev.nnz(dim),
        "Bundle not commutative by nnz"
    );

    println!("✓ Cross-representation operations work correctly");
}

// ============================================================================
// PERFORMANCE BASELINE TESTS
// ============================================================================

#[test]
fn test_operation_performance_baseline() {
    println!("\n=== Performance Baseline Test ===");

    let dim = 100_000;
    let nnz = 5000;
    let iterations = 10;

    let a = BitslicedTritVec::from_sparse(&sparse_random(dim, nnz, 1), dim);
    let b = BitslicedTritVec::from_sparse(&sparse_random(dim, nnz, 2), dim);

    // Bind timing
    let mut bind_timer = Timer::new("bind");
    for _ in 0..iterations {
        bind_timer.lap();
        let _ = a.bind(&b);
        bind_timer.lap();
    }
    println!("{}", bind_timer.summary());

    // Bundle timing
    let mut bundle_timer = Timer::new("bundle");
    for _ in 0..iterations {
        bundle_timer.lap();
        let _ = a.bundle(&b);
        bundle_timer.lap();
    }
    println!("{}", bundle_timer.summary());

    // Dot timing
    let mut dot_timer = Timer::new("dot");
    for _ in 0..iterations {
        dot_timer.lap();
        let _ = a.dot(&b);
        dot_timer.lap();
    }
    println!("{}", dot_timer.summary());

    // Cosine timing
    let mut cosine_timer = Timer::new("cosine");
    for _ in 0..iterations {
        cosine_timer.lap();
        let _ = a.cosine(&b);
        cosine_timer.lap();
    }
    println!("{}", cosine_timer.summary());

    // Permute timing
    let mut permute_timer = Timer::new("permute");
    for _ in 0..iterations {
        permute_timer.lap();
        let _ = a.permute_optimized(1024);
        permute_timer.lap();
    }
    println!("{}", permute_timer.summary());

    println!("✓ Performance baseline captured");
}
