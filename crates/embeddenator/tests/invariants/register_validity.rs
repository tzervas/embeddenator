//! Register-Level Validity Invariant Tests for BitslicedTritVec
//!
//! This module tests the fundamental register-level invariants for bitsliced
//! ternary vectors with dual u64 planes (pos, neg). These invariants are critical
//! for correct VSA operation semantics.
//!
//! # Invariants Tested
//!
//! 1. **No Overlap**: `(pos & neg) == 0` for every word
//!    - A trit position cannot simultaneously be +1 and -1
//!    - Overlap indicates invalid state that could produce incorrect results
//!
//! 2. **Padding Bits Zero**: Unused bits in the last word must be zero
//!    - For dimension D not divisible by 64, bits D%64..63 must be 0
//!    - Non-zero padding can affect popcount-based operations (nnz, dot)
//!
//! 3. **Operations Preserve Validity**: All VSA operations maintain invariants
//!    - Construction: new_zero, from_sparse, from_packed
//!    - Binary ops: bind, bundle
//!    - Unary ops: permute, negate
//!
//! # Encoding Reference
//!
//! ```text
//! pos[w] bit i | neg[w] bit i | Trit value
//! -------------|--------------|------------
//!      0       |      0       |   Z (zero)
//!      1       |      0       |   P (+1)
//!      0       |      1       |   N (-1)
//!      1       |      1       | INVALID (treated as Z)
//! ```

#![cfg(feature = "proptest")]

use embeddenator::{BitslicedTritVec, CarrySaveBundle, PackedTritVec, SparseVec, Trit, DIM};
use proptest::prelude::*;
use std::collections::BTreeMap;

// ============================================================================
// VALIDITY HELPER FUNCTIONS
// ============================================================================

/// Check that no word has overlapping bits in pos and neg planes.
///
/// This is the fundamental invariant: a trit position cannot simultaneously
/// be both positive and negative. Any overlap indicates a bug in construction
/// or operation logic.
///
/// # Returns
/// - `true` if all words satisfy `(pos[i] & neg[i]) == 0`
/// - `false` if any overlap is detected
fn check_no_overlap(v: &BitslicedTritVec) -> bool {
    let pos = v.pos_plane();
    let neg = v.neg_plane();
    
    for (i, (&p, &n)) in pos.iter().zip(neg.iter()).enumerate() {
        let overlap = p & n;
        if overlap != 0 {
            eprintln!(
                "Overlap detected in word {}: pos=0x{:016x}, neg=0x{:016x}, overlap=0x{:016x}",
                i, p, n, overlap
            );
            return false;
        }
    }
    true
}

/// Check that padding bits in the last word are zero.
///
/// For a vector of length L where L % 64 != 0, bits L%64..63 in the last word
/// must be zero. Non-zero padding bits would corrupt:
/// - `nnz()` count (popcount includes padding)
/// - `dot()` product (padding bits could match incorrectly)
/// - Serialization (non-canonical form)
///
/// # Returns
/// - `true` if padding bits are all zero (or if L % 64 == 0)
/// - `false` if any padding bit is set
fn check_padding_zero(v: &BitslicedTritVec) -> bool {
    let len = v.len();
    if len == 0 {
        return true;
    }
    
    let bits_in_last = len % 64;
    if bits_in_last == 0 {
        // Full word, no padding
        return true;
    }
    
    // Mask for valid bits: bits 0..(bits_in_last-1) are valid
    let valid_mask = (1u64 << bits_in_last) - 1;
    let padding_mask = !valid_mask;
    
    let pos = v.pos_plane();
    let neg = v.neg_plane();
    
    if let (Some(&last_pos), Some(&last_neg)) = (pos.last(), neg.last()) {
        let pos_padding = last_pos & padding_mask;
        let neg_padding = last_neg & padding_mask;
        
        if pos_padding != 0 || neg_padding != 0 {
            eprintln!(
                "Padding bits non-zero: len={}, bits_in_last={}, pos_padding=0x{:016x}, neg_padding=0x{:016x}",
                len, bits_in_last, pos_padding, neg_padding
            );
            return false;
        }
    }
    
    true
}

/// Full validity check combining all invariants.
///
/// A BitslicedTritVec is valid if and only if:
/// 1. No overlap exists in any word
/// 2. Padding bits are zero in the last word
///
/// This should be called after every operation in debug/test builds.
fn is_valid(v: &BitslicedTritVec) -> bool {
    check_no_overlap(v) && check_padding_zero(v)
}

/// Detailed validity check returning specific failure information.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ValidityError {
    Overlap { word_idx: usize, overlap_bits: u64 },
    NonZeroPadding { pos_padding: u64, neg_padding: u64, len: usize },
}

fn check_validity_detailed(v: &BitslicedTritVec) -> Result<(), ValidityError> {
    let pos = v.pos_plane();
    let neg = v.neg_plane();
    
    // Check no overlap
    for (i, (&p, &n)) in pos.iter().zip(neg.iter()).enumerate() {
        let overlap = p & n;
        if overlap != 0 {
            return Err(ValidityError::Overlap {
                word_idx: i,
                overlap_bits: overlap,
            });
        }
    }
    
    // Check padding
    let len = v.len();
    if len > 0 {
        let bits_in_last = len % 64;
        if bits_in_last != 0 {
            let padding_mask = !((1u64 << bits_in_last) - 1);
            if let (Some(&last_pos), Some(&last_neg)) = (pos.last(), neg.last()) {
                let pos_padding = last_pos & padding_mask;
                let neg_padding = last_neg & padding_mask;
                if pos_padding != 0 || neg_padding != 0 {
                    return Err(ValidityError::NonZeroPadding {
                        pos_padding,
                        neg_padding,
                        len,
                    });
                }
            }
        }
    }
    
    Ok(())
}

// ============================================================================
// PROPTEST STRATEGIES
// ============================================================================

/// Strategy for generating valid SparseVec instances.
fn sparse_vec_strategy(max_nnz: usize, dim: usize) -> impl Strategy<Value = SparseVec> {
    prop::collection::vec(
        (0usize..dim, prop_oneof![Just(1i8), Just(-1i8)]),
        0..max_nnz,
    )
    .prop_map(|pairs| {
        let mut by_idx: BTreeMap<usize, i8> = BTreeMap::new();
        for (idx, sign) in pairs {
            by_idx.insert(idx, sign);
        }
        
        let mut pos = Vec::new();
        let mut neg = Vec::new();
        
        for (idx, sign) in by_idx {
            match sign {
                1 => pos.push(idx),
                -1 => neg.push(idx),
                _ => {}
            }
        }
        
        SparseVec { pos, neg }
    })
}

/// Strategy for generating valid BitslicedTritVec via SparseVec conversion.
#[allow(dead_code)]
fn bitsliced_vec_strategy(max_nnz: usize, dim: usize) -> impl Strategy<Value = BitslicedTritVec> {
    sparse_vec_strategy(max_nnz, dim).prop_map(move |sparse| {
        BitslicedTritVec::from_sparse(&sparse, dim)
    })
}

/// Strategy for dimensions including edge cases.
fn dimension_strategy() -> impl Strategy<Value = usize> {
    prop_oneof![
        // Edge cases around word boundaries
        Just(1usize),
        Just(63),
        Just(64),
        Just(65),
        Just(127),
        Just(128),
        Just(129),
        Just(255),
        Just(256),
        Just(257),
        // Random dimensions
        1..1000usize,
    ]
}

// ============================================================================
// PROPERTY TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 512,
        max_shrink_iters: 1000,
        .. ProptestConfig::default()
    })]

    /// Property: Any constructed vector (zero, from_sparse) is valid.
    ///
    /// Tests that construction methods always produce valid vectors.
    #[test]
    fn prop_construction_valid(dim in dimension_strategy(), sparse in sparse_vec_strategy(256, DIM)) {
        // Test new_zero
        let zero = BitslicedTritVec::new_zero(dim);
        prop_assert!(is_valid(&zero), "new_zero({}) produced invalid vector", dim);
        
        // Test from_sparse with matching dimension
        let from_sparse = BitslicedTritVec::from_sparse(&sparse, dim);
        prop_assert!(
            is_valid(&from_sparse),
            "from_sparse with dim={} produced invalid vector",
            dim
        );
    }

    /// Property: Binding two valid vectors produces valid output.
    ///
    /// Tests that the bind operation preserves the no-overlap and padding invariants.
    /// Bind formula: out_pos = (a_pos & b_pos) | (a_neg & b_neg)
    ///               out_neg = (a_pos & b_neg) | (a_neg & b_pos)
    #[test]
    fn prop_bind_preserves_validity(
        dim in dimension_strategy(),
        sparse_a in sparse_vec_strategy(128, DIM),
        sparse_b in sparse_vec_strategy(128, DIM)
    ) {
        let a = BitslicedTritVec::from_sparse(&sparse_a, dim);
        let b = BitslicedTritVec::from_sparse(&sparse_b, dim);
        
        // Pre-condition: inputs are valid
        prop_assume!(is_valid(&a));
        prop_assume!(is_valid(&b));
        
        // Operation under test
        let result = a.bind(&b);
        
        // Post-condition: result is valid
        prop_assert!(
            is_valid(&result),
            "bind produced invalid vector for dim={}",
            dim
        );
        
        // Also verify length matches
        prop_assert_eq!(result.len(), dim.min(a.len()).min(b.len()));
    }

    /// Property: Bundling two valid vectors produces valid output.
    ///
    /// Tests that the bundle operation preserves invariants.
    /// Bundle formula: out_pos = (a_pos & !b_neg) | (b_pos & !a_neg)
    ///                 out_neg = (a_neg & !b_pos) | (b_neg & !a_pos)
    #[test]
    fn prop_bundle_preserves_validity(
        dim in dimension_strategy(),
        sparse_a in sparse_vec_strategy(128, DIM),
        sparse_b in sparse_vec_strategy(128, DIM)
    ) {
        let a = BitslicedTritVec::from_sparse(&sparse_a, dim);
        let b = BitslicedTritVec::from_sparse(&sparse_b, dim);
        
        prop_assume!(is_valid(&a));
        prop_assume!(is_valid(&b));
        
        let result = a.bundle(&b);
        
        prop_assert!(
            is_valid(&result),
            "bundle produced invalid vector for dim={}",
            dim
        );
    }

    /// Property: Permutation preserves validity.
    ///
    /// Tests that cyclic rotation maintains the invariants.
    #[test]
    fn prop_permute_preserves_validity(
        dim in dimension_strategy(),
        sparse in sparse_vec_strategy(128, DIM),
        shift in 0usize..1000
    ) {
        let v = BitslicedTritVec::from_sparse(&sparse, dim);
        prop_assume!(is_valid(&v));
        
        let permuted = v.permute(shift);
        
        prop_assert!(
            is_valid(&permuted),
            "permute({}) produced invalid vector for dim={}",
            shift, dim
        );
        
        // Also test optimized permute for 64-aligned dimensions
        if dim % 64 == 0 && dim > 0 {
            let opt_permuted = v.permute_optimized(shift);
            prop_assert!(
                is_valid(&opt_permuted),
                "permute_optimized({}) produced invalid vector for dim={}",
                shift, dim
            );
        }
    }

    /// Property: Conversion from PackedTritVec produces valid BitslicedTritVec.
    ///
    /// Tests the from_packed conversion maintains invariants.
    #[test]
    fn prop_from_packed_valid(
        dim in dimension_strategy(),
        sparse in sparse_vec_strategy(128, DIM)
    ) {
        // Create packed via sparse (known good path)
        let packed = PackedTritVec::from_sparsevec(&sparse, dim);
        
        // Convert to bitsliced
        let bitsliced = BitslicedTritVec::from_packed(&packed);
        
        prop_assert!(
            is_valid(&bitsliced),
            "from_packed produced invalid vector for dim={}",
            dim
        );
        
        // Verify roundtrip preserves content
        let back_to_packed = bitsliced.to_packed();
        for i in 0..dim {
            prop_assert_eq!(
                packed.get(i), 
                back_to_packed.get(i),
                "Roundtrip mismatch at index {} for dim={}",
                i, dim
            );
        }
    }

    /// Property: Negation preserves validity.
    ///
    /// Negation swaps pos and neg planes, which must preserve no-overlap.
    #[test]
    fn prop_negate_preserves_validity(
        dim in dimension_strategy(),
        sparse in sparse_vec_strategy(128, DIM)
    ) {
        let v = BitslicedTritVec::from_sparse(&sparse, dim);
        prop_assume!(is_valid(&v));
        
        let negated = v.negate();
        
        prop_assert!(
            is_valid(&negated),
            "negate produced invalid vector for dim={}",
            dim
        );
    }

    /// Property: CarrySaveBundle finalize produces valid output.
    ///
    /// Tests that the carry-save accumulator produces valid results.
    #[test]
    fn prop_carry_save_valid(
        dim in dimension_strategy(),
        count in 1usize..10,
        sparse_list in prop::collection::vec(sparse_vec_strategy(64, DIM), 1..10)
    ) {
        let mut acc = CarrySaveBundle::new(dim);
        
        for sparse in sparse_list.iter().take(count) {
            let v = BitslicedTritVec::from_sparse(sparse, dim);
            if is_valid(&v) {
                acc.accumulate(&v);
            }
        }
        
        let result = acc.finalize();
        
        prop_assert!(
            is_valid(&result),
            "CarrySaveBundle::finalize produced invalid vector for dim={}",
            dim
        );
    }
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

/// Test word boundary trits (positions 63, 64, 127, 128).
#[test]
fn test_word_boundary_trits() {
    for &dim in &[65, 129, 193, 257, 512] {
        let mut v = BitslicedTritVec::new_zero(dim);
        
        // Set trits at word boundaries
        let boundaries = [0, 63, 64, 127, 128, 191, 192, 255, 256];
        
        for &pos in &boundaries {
            if pos < dim {
                // Alternate between P and N
                let trit = if pos % 2 == 0 { Trit::P } else { Trit::N };
                v.set(pos, trit);
            }
        }
        
        assert!(
            is_valid(&v),
            "Word boundary setting produced invalid vector for dim={}", dim
        );
        
        // Verify each trit is set correctly
        for &pos in &boundaries {
            if pos < dim {
                let expected = if pos % 2 == 0 { Trit::P } else { Trit::N };
                assert_eq!(
                    v.get(pos), expected,
                    "Trit at boundary {} incorrect for dim={}", pos, dim
                );
            }
        }
        
        // Test operations at boundaries
        let v2 = v.clone();
        let bound = v.bind(&v2);
        assert!(is_valid(&bound), "bind at boundaries invalid for dim={}", dim);
        
        let bundled = v.bundle(&v2);
        assert!(is_valid(&bundled), "bundle at boundaries invalid for dim={}", dim);
    }
}

/// Test partial last word (dimensions not multiple of 64).
#[test]
fn test_partial_last_word() {
    // Test various partial fill amounts
    for remainder in 1..64 {
        let dim = 64 + remainder; // 65, 66, ..., 127
        
        let mut v = BitslicedTritVec::new_zero(dim);
        
        // Set the last valid trit
        v.set(dim - 1, Trit::P);
        
        assert!(
            is_valid(&v),
            "Setting last trit failed validity for dim={}", dim
        );
        
        // Try to verify padding is zero by checking detailed validity
        assert!(
            check_validity_detailed(&v).is_ok(),
            "Detailed validity check failed for dim={}", dim
        );
        
        // Verify get/set roundtrip
        assert_eq!(v.get(dim - 1), Trit::P);
        
        // Ensure positions beyond dim don't affect validity
        // (out-of-bounds sets should be no-ops)
        v.set(dim, Trit::N); // Should be ignored
        assert!(is_valid(&v), "Out-of-bounds set corrupted validity");
    }
}

/// Test empty vector.
#[test]
fn test_empty_vector() {
    let v = BitslicedTritVec::new_zero(0);
    
    assert!(is_valid(&v), "Empty vector should be valid");
    assert_eq!(v.len(), 0);
    assert_eq!(v.nnz(), 0);
    assert!(v.is_empty());
    
    // Operations on empty vectors
    let v2 = BitslicedTritVec::new_zero(0);
    let bound = v.bind(&v2);
    assert!(is_valid(&bound), "bind of empty vectors invalid");
    
    let bundled = v.bundle(&v2);
    assert!(is_valid(&bundled), "bundle of empty vectors invalid");
}

/// Test single trit vector.
#[test]
fn test_single_trit_vector() {
    for trit in [Trit::Z, Trit::P, Trit::N] {
        let mut v = BitslicedTritVec::new_zero(1);
        v.set(0, trit);
        
        assert!(is_valid(&v), "Single trit {:?} vector invalid", trit);
        assert_eq!(v.len(), 1);
        assert_eq!(v.get(0), trit);
        
        // Self-bind
        let bound = v.bind(&v);
        assert!(is_valid(&bound), "Self-bind of single {:?} invalid", trit);
        
        // Expected result of self-bind
        let expected = match trit {
            Trit::Z => Trit::Z, // 0 * 0 = 0
            Trit::P => Trit::P, // 1 * 1 = 1
            Trit::N => Trit::P, // -1 * -1 = 1
        };
        assert_eq!(bound.get(0), expected);
    }
}

// ============================================================================
// OVERLAP DETECTION TEST
// ============================================================================

/// Manually corrupt a vector and verify our checker detects it.
///
/// This validates that `check_no_overlap` actually works by creating
/// intentionally invalid states.
#[test]
fn test_overlap_detection() {
    // Create a valid vector
    let dim = 128;
    let sparse = SparseVec { pos: vec![10, 20, 30], neg: vec![40, 50, 60] };
    let mut v = BitslicedTritVec::from_sparse(&sparse, dim);
    
    // Verify it starts valid
    assert!(is_valid(&v), "Initial vector should be valid");
    
    // Manually corrupt by setting overlapping bits
    // This simulates a bug that might set both pos and neg for same index
    {
        // Get mutable access to internals (this is why from_raw exists)
        let mut pos = v.pos_plane().to_vec();
        let mut neg = v.neg_plane().to_vec();
        
        // Create overlap at bit 10 of word 0
        pos[0] |= 1u64 << 10;
        neg[0] |= 1u64 << 10;
        
        // Reconstruct with corrupted data
        v = BitslicedTritVec::from_raw(dim, pos, neg);
    }
    
    // Now it should fail validation
    assert!(
        !check_no_overlap(&v),
        "Corrupted vector should fail overlap check"
    );
    
    // Detailed check should report the specific word
    match check_validity_detailed(&v) {
        Err(ValidityError::Overlap { word_idx, overlap_bits }) => {
            assert_eq!(word_idx, 0, "Overlap should be in word 0");
            assert_eq!(overlap_bits & (1u64 << 10), 1u64 << 10, "Overlap should include bit 10");
        }
        other => panic!("Expected Overlap error, got {:?}", other),
    }
}

/// Test that padding corruption is detected.
#[test]
fn test_padding_corruption_detection() {
    // Create a vector with partial last word
    let dim = 100; // 100 % 64 = 36, so bits 36-63 are padding
    let sparse = SparseVec { pos: vec![0, 50], neg: vec![25, 75] };
    let mut v = BitslicedTritVec::from_sparse(&sparse, dim);
    
    // Verify it starts valid
    assert!(is_valid(&v), "Initial vector should be valid");
    
    // Corrupt padding bits
    {
        let mut pos = v.pos_plane().to_vec();
        
        // Set a bit in the padding region (bit 40, which is > 36)
        pos[1] |= 1u64 << 40;
        
        v = BitslicedTritVec::from_raw(dim, pos, v.neg_plane().to_vec());
    }
    
    // Should fail padding check
    assert!(
        !check_padding_zero(&v),
        "Corrupted padding should fail check"
    );
    
    // Detailed check should report padding error
    match check_validity_detailed(&v) {
        Err(ValidityError::NonZeroPadding { pos_padding, len, .. }) => {
            assert_eq!(len, 100);
            assert!(pos_padding != 0, "Should report non-zero pos padding");
        }
        other => panic!("Expected NonZeroPadding error, got {:?}", other),
    }
}

// ============================================================================
// STRESS TESTS
// ============================================================================

/// Stress test with many sequential operations.
#[test]
fn test_operation_chain_validity() {
    let dim = 1000;
    
    // Create initial vectors
    let sparse_a = SparseVec { 
        pos: (0..50).map(|i| i * 20).collect(), 
        neg: (0..50).map(|i| i * 20 + 10).collect() 
    };
    let sparse_b = SparseVec { 
        pos: (0..50).map(|i| i * 20 + 5).collect(), 
        neg: (0..50).map(|i| i * 20 + 15).collect() 
    };
    
    let mut a = BitslicedTritVec::from_sparse(&sparse_a, dim);
    let b = BitslicedTritVec::from_sparse(&sparse_b, dim);
    
    // Chain of operations
    for i in 0..100 {
        match i % 4 {
            0 => a = a.bind(&b),
            1 => a = a.bundle(&b),
            2 => a = a.permute(i),
            3 => a = a.negate(),
            _ => unreachable!(),
        }
        
        assert!(
            is_valid(&a),
            "Operation chain failed at step {} (op={})", 
            i, 
            ["bind", "bundle", "permute", "negate"][i % 4]
        );
    }
}

/// Test all trit combinations for correctness.
#[test]
fn test_exhaustive_small_vector() {
    // For a 3-trit vector, test all 3^3 = 27 combinations
    for a0 in [Trit::Z, Trit::P, Trit::N] {
        for a1 in [Trit::Z, Trit::P, Trit::N] {
            for a2 in [Trit::Z, Trit::P, Trit::N] {
                let mut v = BitslicedTritVec::new_zero(3);
                v.set(0, a0);
                v.set(1, a1);
                v.set(2, a2);
                
                assert!(
                    is_valid(&v),
                    "Vector [{:?}, {:?}, {:?}] should be valid",
                    a0, a1, a2
                );
                
                // Verify roundtrip
                assert_eq!(v.get(0), a0);
                assert_eq!(v.get(1), a1);
                assert_eq!(v.get(2), a2);
            }
        }
    }
}

/// Test validity across typical dimension values.
#[test]
fn test_standard_dimensions() {
    // Common dimensions used in practice
    let dims = [
        64,      // Single word
        256,     // 4 words
        1024,    // 16 words
        10000,   // Default DIM
        65536,   // Power of 2
    ];
    
    for &dim in &dims {
        // Create random-ish sparse vector
        let nnz = dim / 100 + 10;
        let pos: Vec<usize> = (0..nnz).map(|i| (i * 97) % dim).collect();
        let neg: Vec<usize> = (0..nnz).map(|i| ((i * 101) + 50) % dim).collect();
        
        // Remove duplicates between pos and neg
        let pos: Vec<usize> = pos.into_iter().filter(|x| !neg.contains(x)).collect();
        
        let sparse = SparseVec { pos, neg };
        let v = BitslicedTritVec::from_sparse(&sparse, dim);
        
        assert!(is_valid(&v), "Standard dimension {} failed validity", dim);
        
        // Test operations
        let v2 = v.clone();
        assert!(is_valid(&v.bind(&v2)), "bind failed for dim={}", dim);
        assert!(is_valid(&v.bundle(&v2)), "bundle failed for dim={}", dim);
        assert!(is_valid(&v.permute(1)), "permute failed for dim={}", dim);
        assert!(is_valid(&v.negate()), "negate failed for dim={}", dim);
    }
}

// ============================================================================
// ADDITIONAL COVERAGE: QA-IDENTIFIED GAPS
// ============================================================================

/// Test full u64 saturation - every bit in a word is used.
/// This ensures operations work correctly at maximum density within a word.
#[test]
fn test_full_saturation_word() {
    let dim = 64;
    
    // Create a fully saturated word: all 64 trits are non-zero
    // Pattern: alternating +1/-1 to maximize bit usage without overlap
    let mut v = BitslicedTritVec::new_zero(dim);
    for i in 0..64 {
        v.set(i, if i % 2 == 0 { Trit::P } else { Trit::N });
    }
    
    assert!(is_valid(&v), "Full saturation word should be valid");
    assert_eq!(v.nnz(), 64, "All 64 trits should be non-zero");
    
    // Verify bit patterns: pos=0x5555..., neg=0xAAAA...
    let pos = v.pos_plane()[0];
    let neg = v.neg_plane()[0];
    assert_eq!(pos, 0x5555_5555_5555_5555, "pos should be even bits");
    assert_eq!(neg, 0xAAAA_AAAA_AAAA_AAAA, "neg should be odd bits");
    assert_eq!(pos | neg, !0u64, "union should cover all bits");
    assert_eq!(pos & neg, 0, "no overlap");
    
    // Test operations on saturated word
    let v2 = v.clone();
    
    // Bind: +1*+1=+1, -1*-1=+1, so result should be all +1
    let bound = v.bind(&v2);
    assert!(is_valid(&bound), "bind of saturated should be valid");
    for i in 0..64 {
        assert_eq!(bound.get(i), Trit::P, "bind result at {} should be P", i);
    }
    
    // Bundle: +1++1=+1, -1+-1=-1 (saturating), pattern preserved
    let bundled = v.bundle(&v2);
    assert!(is_valid(&bundled), "bundle of saturated should be valid");
    for i in 0..64 {
        let expected = if i % 2 == 0 { Trit::P } else { Trit::N };
        assert_eq!(bundled.get(i), expected, "bundle result at {} wrong", i);
    }
}

/// Test multi-word full saturation.
#[test]
fn test_full_saturation_multi_word() {
    let dim = 256; // 4 words
    
    // All +1 in first 128, all -1 in second 128
    let mut v = BitslicedTritVec::new_zero(dim);
    for i in 0..128 {
        v.set(i, Trit::P);
    }
    for i in 128..256 {
        v.set(i, Trit::N);
    }
    
    assert!(is_valid(&v), "Multi-word saturation should be valid");
    assert_eq!(v.nnz(), 256, "All 256 trits should be non-zero");
    
    // Verify bit patterns per word
    assert_eq!(v.pos_plane()[0], !0u64, "word 0 pos should be full");
    assert_eq!(v.pos_plane()[1], !0u64, "word 1 pos should be full");
    assert_eq!(v.neg_plane()[2], !0u64, "word 2 neg should be full");
    assert_eq!(v.neg_plane()[3], !0u64, "word 3 neg should be full");
}

/// Test permute with shifts that cross word boundaries.
/// This validates bit-carry correctness across multiple words.
#[test]
fn test_permute_cross_word_carry() {
    let dim = 192; // 3 words
    
    // Set single bit at position 0
    let mut v = BitslicedTritVec::new_zero(dim);
    v.set(0, Trit::P);
    
    assert!(is_valid(&v), "Initial vector should be valid");
    assert_eq!(v.nnz(), 1);
    
    // Permute by 65 (crosses into word 1)
    let p65 = v.permute(65);
    assert!(is_valid(&p65), "permute(65) should be valid");
    assert_eq!(p65.nnz(), 1, "nnz should be preserved");
    assert_eq!(p65.get(65), Trit::P, "bit should be at position 65");
    assert_eq!(p65.get(0), Trit::Z, "original position should be zero");
    
    // Permute by 127 (near word boundary)
    let p127 = v.permute(127);
    assert!(is_valid(&p127), "permute(127) should be valid");
    assert_eq!(p127.nnz(), 1);
    assert_eq!(p127.get(127), Trit::P);
    
    // Permute by 128 (exactly at word 2 start)
    let p128 = v.permute(128);
    assert!(is_valid(&p128), "permute(128) should be valid");
    assert_eq!(p128.nnz(), 1);
    assert_eq!(p128.get(128), Trit::P);
}

/// Test permute wrap-around at dimension boundary.
#[test]
fn test_permute_wrap_around() {
    let dim = 100; // Non-power-of-2 with partial last word
    
    // Set bit at position 99 (last valid)
    let mut v = BitslicedTritVec::new_zero(dim);
    v.set(99, Trit::N);
    
    assert!(is_valid(&v), "Initial should be valid");
    
    // Permute by 1: should wrap 99 -> 0
    let p1 = v.permute(1);
    assert!(is_valid(&p1), "permute(1) wrap should be valid");
    assert_eq!(p1.nnz(), 1, "nnz preserved");
    assert_eq!(p1.get(0), Trit::N, "wrapped bit should be at 0");
    assert_eq!(p1.get(99), Trit::Z, "original position should be zero");
    
    // Permute by dim-1: should move 99 -> 98
    let pdm1 = v.permute(dim - 1);
    assert!(is_valid(&pdm1), "permute(dim-1) should be valid");
    assert_eq!(pdm1.get(98), Trit::N);
}

/// Test nnz invariant: operations preserve non-zero count correctly.
#[test]
fn test_nnz_invariant_preservation() {
    let dim = 256;
    
    // Create vector with known nnz
    let mut v = BitslicedTritVec::new_zero(dim);
    for i in (0..256).step_by(4) {
        v.set(i, Trit::P);
    }
    for i in (2..256).step_by(4) {
        v.set(i, Trit::N);
    }
    
    let initial_nnz = v.nnz();
    assert_eq!(initial_nnz, 128, "64 pos + 64 neg = 128 nnz");
    
    // Negate: swaps pos/neg but preserves nnz
    let negated = v.negate();
    assert!(is_valid(&negated));
    assert_eq!(negated.nnz(), initial_nnz, "negate preserves nnz");
    
    // Permute: always preserves nnz
    let permuted = v.permute(37);
    assert!(is_valid(&permuted));
    assert_eq!(permuted.nnz(), initial_nnz, "permute preserves nnz");
    
    // Bind: nnz can change (zero * nonzero = zero)
    // But bind(v, v) = v for ternary (since x*x = |x|)
    // Actually: +1*+1=+1, -1*-1=+1, so bind(v,v) has all +1 where v was nonzero
    let self_bound = v.bind(&v);
    assert!(is_valid(&self_bound));
    assert_eq!(self_bound.nnz(), initial_nnz, "bind(v,v) preserves nnz");
}

/// Test bind_into and bundle_into validity (in-place operations).
#[test]
fn test_into_operations_validity() {
    let dim = 128;
    
    // Create test vectors
    let pos1: Vec<usize> = (0..20).map(|i| i * 5).collect();
    let neg1: Vec<usize> = (0..20).map(|i| i * 5 + 2).collect();
    let sparse1 = SparseVec { pos: pos1, neg: neg1 };
    let v1 = BitslicedTritVec::from_sparse(&sparse1, dim);
    
    let pos2: Vec<usize> = (0..20).map(|i| i * 6).collect();
    let neg2: Vec<usize> = (0..20).map(|i| i * 6 + 3).collect();
    let sparse2 = SparseVec { pos: pos2, neg: neg2 };
    let v2 = BitslicedTritVec::from_sparse(&sparse2, dim);
    
    // Test bind_into
    let mut out = BitslicedTritVec::new_zero(dim);
    v1.bind_into(&v2, &mut out);
    assert!(is_valid(&out), "bind_into result should be valid");
    
    // Verify it matches non-in-place bind
    let expected = v1.bind(&v2);
    for i in 0..dim {
        assert_eq!(out.get(i), expected.get(i), "bind_into mismatch at {}", i);
    }
    
    // Test bundle_into
    let mut out2 = BitslicedTritVec::new_zero(dim);
    v1.bundle_into(&v2, &mut out2);
    assert!(is_valid(&out2), "bundle_into result should be valid");
    
    let expected2 = v1.bundle(&v2);
    for i in 0..dim {
        assert_eq!(out2.get(i), expected2.get(i), "bundle_into mismatch at {}", i);
    }
}
