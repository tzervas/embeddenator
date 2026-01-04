//! Block-Sparse TritVec Invariant Tests
//!
//! This module provides comprehensive property-based tests for `BlockSparseTritVec`
//! to ensure invariants are maintained through all operations. These tests are
//! critical for validating the correctness of massive-dimension sparse vector
//! operations (1M-1B dimensions).
//!
//! # Invariants Tested
//!
//! 1. **Sorted Blocks**: Blocks must be sorted by `block_id` in strictly ascending order
//! 2. **No Overlap**: Every block must satisfy `(pos & neg) == 0`
//! 3. **No Zero Blocks**: No zero blocks should be stored (garbage collection)
//! 4. **Dimension Consistency**: Operations preserve dimension semantics
//!
//! # Coverage
//!
//! - Construction: `new`, `from_sparse`, `from_bitsliced`
//! - Operations: `bind`, `bundle`, `negate`, `bundle_many`
//! - Conversions: `to_sparse`, `to_bitsliced` roundtrips
//! - Edge cases: Empty, single block, billion-dimension, disjoint/overlapping blocks

#![cfg(feature = "proptest")]

use embeddenator::{BitslicedTritVec, Block, BlockSparseTritVec, SparseVec};
use proptest::prelude::*;
use std::collections::BTreeMap;

// ============================================================================
// INVARIANT HELPER FUNCTIONS
// ============================================================================

/// Check that blocks are sorted by block_id in strictly ascending order.
///
/// This invariant is critical for O(n+m) merge operations - unsorted blocks
/// would produce incorrect results in bind/bundle/dot operations.
fn is_sorted(blocks: &[(u32, Block)]) -> bool {
    blocks.windows(2).all(|w| w[0].0 < w[1].0)
}

/// Check that no zero blocks are stored.
///
/// Zero blocks waste memory and should be garbage collected. Storing them
/// violates the sparse representation contract.
fn has_no_zero_blocks(blocks: &[(u32, Block)]) -> bool {
    blocks.iter().all(|(_, b)| !b.is_zero())
}

/// Check that no block has overlapping pos and neg bits.
///
/// This is the fundamental ternary invariant: a trit position cannot
/// simultaneously be +1 and -1.
fn has_no_overlap(blocks: &[(u32, Block)]) -> bool {
    blocks.iter().all(|(_, b)| b.is_valid())
}

/// Full validity check for BlockSparseTritVec.
///
/// A BlockSparseTritVec is valid if and only if:
/// 1. Blocks are sorted by block_id
/// 2. No block has overlapping pos/neg bits
/// 3. No zero blocks are stored
fn is_valid(v: &BlockSparseTritVec) -> bool {
    let blocks = v.blocks();
    is_sorted(blocks) && has_no_overlap(blocks) && has_no_zero_blocks(blocks)
}

/// Detailed validity errors for debugging.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ValidityError {
    /// Blocks are not sorted at the given index.
    UnsortedAt { index: usize, id_a: u32, id_b: u32 },
    /// A block has overlapping pos/neg bits.
    Overlap { block_id: u32, overlap: u64 },
    /// A zero block is stored.
    ZeroBlock { block_id: u32 },
}

/// Detailed validity check with error information.
fn check_validity_detailed(v: &BlockSparseTritVec) -> Result<(), ValidityError> {
    let blocks = v.blocks();

    // Check sorted order
    for (i, window) in blocks.windows(2).enumerate() {
        if window[0].0 >= window[1].0 {
            return Err(ValidityError::UnsortedAt {
                index: i + 1,
                id_a: window[0].0,
                id_b: window[1].0,
            });
        }
    }

    // Check each block
    for &(block_id, block) in blocks {
        // Check overlap
        let overlap = block.pos & block.neg;
        if overlap != 0 {
            return Err(ValidityError::Overlap { block_id, overlap });
        }

        // Check zero block
        if block.is_zero() {
            return Err(ValidityError::ZeroBlock { block_id });
        }
    }

    Ok(())
}

// ============================================================================
// PROPTEST STRATEGIES
// ============================================================================

/// Strategy for generating valid SparseVec instances.
fn sparse_vec_strategy(max_nnz: usize, dim: usize) -> impl Strategy<Value = SparseVec> {
    prop::collection::vec((0usize..dim, prop_oneof![Just(1i8), Just(-1i8)]), 0..max_nnz).prop_map(
        |pairs| {
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
        },
    )
}

/// Strategy for generating valid BlockSparseTritVec via SparseVec conversion.
#[allow(dead_code)]
fn block_sparse_vec_strategy(
    max_nnz: usize,
    dim: usize,
) -> impl Strategy<Value = BlockSparseTritVec> {
    sparse_vec_strategy(max_nnz, dim)
        .prop_map(move |sparse| BlockSparseTritVec::from_sparse(&sparse, dim))
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
        Just(512),
        Just(1000),
        // Random moderate dimensions
        64..10_000usize,
    ]
}

/// Strategy for large dimensions (without excessive memory).
fn large_dimension_strategy() -> impl Strategy<Value = usize> {
    prop_oneof![
        Just(1_000_000usize),
        Just(10_000_000),
        Just(100_000_000),
        Just(1_000_000_000), // 1 billion
    ]
}

/// Strategy for valid Block instances.
fn block_strategy() -> impl Strategy<Value = Block> {
    (any::<u64>(), any::<u64>()).prop_map(|(p, n)| {
        // Ensure no overlap
        Block::new(p & !n, n & !p)
    })
}

/// Strategy for a vector of (block_id, Block) pairs in sorted order.
fn sorted_blocks_strategy(
    max_blocks: usize,
    max_block_id: u32,
) -> impl Strategy<Value = Vec<(u32, Block)>> {
    prop::collection::vec((0u32..max_block_id, block_strategy()), 0..max_blocks).prop_map(
        |mut blocks| {
            // Sort and deduplicate by block_id
            blocks.sort_by_key(|(id, _)| *id);
            blocks.dedup_by_key(|(id, _)| *id);
            // Remove zero blocks
            blocks.retain(|(_, b)| !b.is_zero());
            blocks
        },
    )
}

// ============================================================================
// PROPERTY TESTS: CONSTRUCTION
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 512,
        max_shrink_iters: 1000,
        .. ProptestConfig::default()
    })]

    /// Property: `new()` always produces a valid empty vector.
    #[test]
    fn prop_new_produces_valid_empty(dim in dimension_strategy()) {
        let v = BlockSparseTritVec::new(dim);

        prop_assert!(is_valid(&v), "new({}) produced invalid vector", dim);
        prop_assert_eq!(v.dim(), dim);
        prop_assert_eq!(v.block_count(), 0);
        prop_assert_eq!(v.nnz(), 0);
    }

    /// Property: `with_capacity()` produces a valid empty vector.
    #[test]
    fn prop_with_capacity_produces_valid(dim in dimension_strategy(), capacity in 0usize..1000) {
        let v = BlockSparseTritVec::with_capacity(dim, capacity);

        prop_assert!(is_valid(&v), "with_capacity({}, {}) produced invalid vector", dim, capacity);
        prop_assert_eq!(v.dim(), dim);
        prop_assert_eq!(v.block_count(), 0);
    }

    /// Property: `from_sparse()` always produces a valid vector.
    #[test]
    fn prop_from_sparse_produces_valid(
        dim in dimension_strategy(),
        sparse in sparse_vec_strategy(256, 10_000)
    ) {
        let v = BlockSparseTritVec::from_sparse(&sparse, dim);

        prop_assert!(is_valid(&v), "from_sparse produced invalid vector");
        prop_assert_eq!(v.dim(), dim);

        // Validate detailed
        match check_validity_detailed(&v) {
            Ok(()) => {}
            Err(e) => prop_assert!(false, "from_sparse validity error: {:?}", e),
        }
    }

    /// Property: `from_bitsliced()` always produces a valid vector.
    #[test]
    fn prop_from_bitsliced_produces_valid(
        dim in dimension_strategy(),
        sparse in sparse_vec_strategy(256, 10_000)
    ) {
        let bitsliced = BitslicedTritVec::from_sparse(&sparse, dim);
        let v = BlockSparseTritVec::from_bitsliced(&bitsliced);

        prop_assert!(is_valid(&v), "from_bitsliced produced invalid vector");
        prop_assert_eq!(v.dim(), dim);
    }
}

// ============================================================================
// PROPERTY TESTS: OPERATIONS PRESERVE VALIDITY
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 512,
        max_shrink_iters: 1000,
        .. ProptestConfig::default()
    })]

    /// Property: `bind()` preserves validity.
    ///
    /// The bind operation (elementwise multiply) must:
    /// - Produce sorted blocks
    /// - Have no overlapping pos/neg bits
    /// - Remove any resulting zero blocks
    #[test]
    fn prop_bind_preserves_validity(
        dim in 1usize..10_000,
        sparse_a in sparse_vec_strategy(128, 10_000),
        sparse_b in sparse_vec_strategy(128, 10_000)
    ) {
        let a = BlockSparseTritVec::from_sparse(&sparse_a, dim);
        let b = BlockSparseTritVec::from_sparse(&sparse_b, dim);

        // Pre-condition: inputs are valid
        prop_assume!(is_valid(&a));
        prop_assume!(is_valid(&b));

        // Operation under test
        let result = a.bind(&b);

        // Post-condition: result is valid
        prop_assert!(
            is_valid(&result),
            "bind produced invalid vector: {:?}",
            check_validity_detailed(&result)
        );
        prop_assert_eq!(result.dim(), dim);
    }

    /// Property: `bundle()` preserves validity.
    ///
    /// The bundle operation (saturating add) must:
    /// - Produce sorted blocks
    /// - Have no overlapping pos/neg bits
    /// - Remove any resulting zero blocks (from cancellation)
    #[test]
    fn prop_bundle_preserves_validity(
        dim in 1usize..10_000,
        sparse_a in sparse_vec_strategy(128, 10_000),
        sparse_b in sparse_vec_strategy(128, 10_000)
    ) {
        let a = BlockSparseTritVec::from_sparse(&sparse_a, dim);
        let b = BlockSparseTritVec::from_sparse(&sparse_b, dim);

        prop_assume!(is_valid(&a));
        prop_assume!(is_valid(&b));

        let result = a.bundle(&b);

        prop_assert!(
            is_valid(&result),
            "bundle produced invalid vector: {:?}",
            check_validity_detailed(&result)
        );
        prop_assert_eq!(result.dim(), dim);
    }

    /// Property: `negate()` preserves validity.
    #[test]
    fn prop_negate_preserves_validity(
        dim in 1usize..10_000,
        sparse in sparse_vec_strategy(256, 10_000)
    ) {
        let v = BlockSparseTritVec::from_sparse(&sparse, dim);
        prop_assume!(is_valid(&v));

        let negated = v.negate();

        prop_assert!(
            is_valid(&negated),
            "negate produced invalid vector: {:?}",
            check_validity_detailed(&negated)
        );
        prop_assert_eq!(negated.dim(), dim);
        prop_assert_eq!(negated.block_count(), v.block_count());
        prop_assert_eq!(negated.nnz(), v.nnz());
    }

    /// Property: Double negation is identity.
    #[test]
    fn prop_double_negate_identity(
        dim in 1usize..10_000,
        sparse in sparse_vec_strategy(128, 10_000)
    ) {
        let v = BlockSparseTritVec::from_sparse(&sparse, dim);
        prop_assume!(is_valid(&v));

        let result = v.negate().negate();

        prop_assert_eq!(v, result, "double negate should be identity");
    }
}

// ============================================================================
// PROPERTY TESTS: ROUNDTRIP CONVERSIONS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        max_shrink_iters: 1000,
        .. ProptestConfig::default()
    })]

    /// Property: `from_sparse/to_sparse` roundtrip produces identical data.
    #[test]
    fn prop_sparse_roundtrip(
        dim in 1usize..10_000,
        sparse in sparse_vec_strategy(256, 10_000)
    ) {
        // Filter sparse to dimension
        let filtered = SparseVec {
            pos: sparse.pos.into_iter().filter(|&i| i < dim).collect(),
            neg: sparse.neg.into_iter().filter(|&i| i < dim).collect(),
        };

        let block = BlockSparseTritVec::from_sparse(&filtered, dim);
        let recovered = block.to_sparse();

        // Recovered should match filtered (same indices)
        let mut expected_pos = filtered.pos.clone();
        let mut expected_neg = filtered.neg.clone();
        expected_pos.sort();
        expected_neg.sort();

        let mut got_pos = recovered.pos.clone();
        let mut got_neg = recovered.neg.clone();
        got_pos.sort();
        got_neg.sort();

        prop_assert_eq!(expected_pos, got_pos, "pos indices mismatch in sparse roundtrip");
        prop_assert_eq!(expected_neg, got_neg, "neg indices mismatch in sparse roundtrip");
    }

    /// Property: `from_bitsliced/to_bitsliced` roundtrip produces identical data.
    #[test]
    fn prop_bitsliced_roundtrip(
        dim in 1usize..5_000,
        sparse in sparse_vec_strategy(256, 5_000)
    ) {
        // Build original bitsliced
        let filtered = SparseVec {
            pos: sparse.pos.into_iter().filter(|&i| i < dim).collect(),
            neg: sparse.neg.into_iter().filter(|&i| i < dim).collect(),
        };
        let original = BitslicedTritVec::from_sparse(&filtered, dim);

        // Convert to block-sparse and back
        let block = BlockSparseTritVec::from_bitsliced(&original);
        let recovered = block.to_bitsliced();

        // Compare planes
        prop_assert_eq!(original.len(), recovered.len(), "dimension mismatch in bitsliced roundtrip");
        prop_assert_eq!(original.pos_plane(), recovered.pos_plane(), "pos plane mismatch");
        prop_assert_eq!(original.neg_plane(), recovered.neg_plane(), "neg plane mismatch");
    }

    /// Property: `from_sparse → to_bitsliced` equals direct `BitslicedTritVec::from_sparse`.
    #[test]
    fn prop_sparse_to_bitsliced_equivalence(
        dim in 1usize..5_000,
        sparse in sparse_vec_strategy(256, 5_000)
    ) {
        let filtered = SparseVec {
            pos: sparse.pos.into_iter().filter(|&i| i < dim).collect(),
            neg: sparse.neg.into_iter().filter(|&i| i < dim).collect(),
        };

        // Direct conversion
        let direct = BitslicedTritVec::from_sparse(&filtered, dim);

        // Via block-sparse
        let via_block = BlockSparseTritVec::from_sparse(&filtered, dim).to_bitsliced();

        prop_assert_eq!(direct.pos_plane(), via_block.pos_plane());
        prop_assert_eq!(direct.neg_plane(), via_block.neg_plane());
    }
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

/// Tests for empty vectors.
mod empty_vectors {
    use super::*;

    #[test]
    fn empty_vector_is_valid() {
        let v = BlockSparseTritVec::new(1000);
        assert!(is_valid(&v));
        assert_eq!(v.block_count(), 0);
        assert_eq!(v.nnz(), 0);
    }

    #[test]
    fn empty_bind_empty() {
        let a = BlockSparseTritVec::new(1000);
        let b = BlockSparseTritVec::new(1000);
        let result = a.bind(&b);

        assert!(is_valid(&result));
        assert_eq!(result.block_count(), 0);
    }

    #[test]
    fn empty_bundle_empty() {
        let a = BlockSparseTritVec::new(1000);
        let b = BlockSparseTritVec::new(1000);
        let result = a.bundle(&b);

        assert!(is_valid(&result));
        assert_eq!(result.block_count(), 0);
    }

    #[test]
    fn non_empty_bind_empty() {
        let sparse = SparseVec {
            pos: vec![0, 64, 128],
            neg: vec![1, 65],
        };
        let a = BlockSparseTritVec::from_sparse(&sparse, 1000);
        let b = BlockSparseTritVec::new(1000);

        // Bind with empty should be empty (multiply by zero)
        let result = a.bind(&b);
        assert!(is_valid(&result));
        assert_eq!(result.block_count(), 0);
    }

    #[test]
    fn non_empty_bundle_empty() {
        let sparse = SparseVec {
            pos: vec![0, 64, 128],
            neg: vec![1, 65],
        };
        let a = BlockSparseTritVec::from_sparse(&sparse, 1000);
        let b = BlockSparseTritVec::new(1000);

        // Bundle with empty should preserve original
        let result = a.bundle(&b);
        assert!(is_valid(&result));
        assert_eq!(result.to_sparse().pos, a.to_sparse().pos);
        assert_eq!(result.to_sparse().neg, a.to_sparse().neg);
    }

    #[test]
    fn empty_sparse_roundtrip() {
        let sparse = SparseVec {
            pos: vec![],
            neg: vec![],
        };
        let block = BlockSparseTritVec::from_sparse(&sparse, 1000);
        let recovered = block.to_sparse();

        assert!(is_valid(&block));
        assert!(recovered.pos.is_empty());
        assert!(recovered.neg.is_empty());
    }
}

/// Tests for single-block vectors.
mod single_block {
    use super::*;

    #[test]
    fn single_block_construction() {
        let sparse = SparseVec {
            pos: vec![0, 5, 10, 63],
            neg: vec![1, 2, 62],
        };
        let v = BlockSparseTritVec::from_sparse(&sparse, 64);

        assert!(is_valid(&v));
        assert_eq!(v.block_count(), 1);
        assert_eq!(v.nnz(), 7);
    }

    #[test]
    fn single_block_bind_same() {
        let sparse = SparseVec {
            pos: vec![0, 1, 2],
            neg: vec![60, 61, 62],
        };
        let v = BlockSparseTritVec::from_sparse(&sparse, 64);
        let result = v.bind(&v);

        assert!(is_valid(&result));
        // Bind with self: (+1)*(+1)=+1, (-1)*(-1)=+1
        // All become positive
        let recovered = result.to_sparse();
        assert_eq!(recovered.pos.len(), 6);
        assert!(recovered.neg.is_empty());
    }

    #[test]
    fn single_block_negate() {
        let sparse = SparseVec {
            pos: vec![0, 1, 2],
            neg: vec![60, 61, 62],
        };
        let v = BlockSparseTritVec::from_sparse(&sparse, 64);
        let negated = v.negate();

        assert!(is_valid(&negated));
        let recovered = negated.to_sparse();
        // pos and neg should swap
        assert_eq!(recovered.neg, vec![0, 1, 2]);
        assert_eq!(recovered.pos, vec![60, 61, 62]);
    }
}

/// Tests for billion-dimension vectors (construction only, minimal ops).
mod billion_dimension {
    use super::*;

    #[test]
    fn billion_dim_empty_construction() {
        let v = BlockSparseTritVec::new(1_000_000_000);
        assert!(is_valid(&v));
        assert_eq!(v.dim(), 1_000_000_000);
        assert_eq!(v.block_count(), 0);
    }

    #[test]
    fn billion_dim_with_sparse_blocks() {
        // Insert a few blocks at high indices
        let sparse = SparseVec {
            pos: vec![999_999_936, 999_999_937], // Block 15_624_999
            neg: vec![0, 1],                      // Block 0
        };
        let v = BlockSparseTritVec::from_sparse(&sparse, 1_000_000_000);

        assert!(is_valid(&v));
        assert_eq!(v.dim(), 1_000_000_000);
        assert_eq!(v.block_count(), 2); // Block 0 and block 15_624_999
        assert_eq!(v.nnz(), 4);
    }

    #[test]
    fn billion_dim_sparse_roundtrip() {
        let sparse = SparseVec {
            pos: vec![500_000_000, 999_999_999],
            neg: vec![0, 250_000_000],
        };
        let v = BlockSparseTritVec::from_sparse(&sparse, 1_000_000_000);
        let recovered = v.to_sparse();

        assert!(is_valid(&v));

        let mut expected_pos = sparse.pos.clone();
        let mut expected_neg = sparse.neg.clone();
        expected_pos.sort();
        expected_neg.sort();

        let mut got_pos = recovered.pos.clone();
        let mut got_neg = recovered.neg.clone();
        got_pos.sort();
        got_neg.sort();

        assert_eq!(expected_pos, got_pos);
        assert_eq!(expected_neg, got_neg);
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 16, // Fewer cases for expensive tests
            .. ProptestConfig::default()
        })]

        /// Property: Billion-dim construction with random sparse indices is valid.
        #[test]
        fn prop_billion_dim_construction(dim in large_dimension_strategy()) {
            // Generate sparse indices spread across the dimension space
            let pos: Vec<usize> = (0..10)
                .map(|i| (i * (dim / 10)).min(dim - 1))
                .collect();
            let neg: Vec<usize> = (0..10)
                .map(|i| ((i * (dim / 10)) + dim / 20).min(dim - 1))
                .filter(|x| !pos.contains(x))
                .collect();

            let sparse = SparseVec { pos, neg };
            let v = BlockSparseTritVec::from_sparse(&sparse, dim);

            prop_assert!(is_valid(&v), "billion-dim construction invalid");
            prop_assert_eq!(v.dim(), dim);
        }
    }
}

/// Tests for operations between vectors with disjoint blocks.
mod disjoint_blocks {
    use super::*;

    #[test]
    fn disjoint_bind_produces_empty() {
        // a has blocks 0, 1; b has blocks 100, 101
        let sparse_a = SparseVec {
            pos: vec![0, 64],
            neg: vec![1, 65],
        };
        let sparse_b = SparseVec {
            pos: vec![6400, 6464],
            neg: vec![6401, 6465],
        };
        let a = BlockSparseTritVec::from_sparse(&sparse_a, 10000);
        let b = BlockSparseTritVec::from_sparse(&sparse_b, 10000);

        // Bind with disjoint blocks = empty (all multiply by zero)
        let result = a.bind(&b);

        assert!(is_valid(&result));
        assert_eq!(result.block_count(), 0);
        assert_eq!(result.nnz(), 0);
    }

    #[test]
    fn disjoint_bundle_preserves_all() {
        let sparse_a = SparseVec {
            pos: vec![0, 64],
            neg: vec![1, 65],
        };
        let sparse_b = SparseVec {
            pos: vec![6400, 6464],
            neg: vec![6401, 6465],
        };
        let a = BlockSparseTritVec::from_sparse(&sparse_a, 10000);
        let b = BlockSparseTritVec::from_sparse(&sparse_b, 10000);

        // Bundle with disjoint blocks = union
        let result = a.bundle(&b);

        assert!(is_valid(&result));
        assert_eq!(result.block_count(), 4); // All blocks preserved
        assert_eq!(result.nnz(), 8);
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 128,
            .. ProptestConfig::default()
        })]

        /// Property: Bundling disjoint vectors gives union of nnz.
        #[test]
        fn prop_disjoint_bundle_union(
            nnz_a in 1usize..50,
            nnz_b in 1usize..50
        ) {
            // Create vectors in non-overlapping index ranges
            let dim = 100_000;
            let pos_a: Vec<usize> = (0..nnz_a).map(|i| i * 64).collect();
            let neg_b: Vec<usize> = (0..nnz_b).map(|i| 50_000 + i * 64).collect();

            let a = BlockSparseTritVec::from_sparse(&SparseVec { pos: pos_a.clone(), neg: vec![] }, dim);
            let b = BlockSparseTritVec::from_sparse(&SparseVec { pos: vec![], neg: neg_b.clone() }, dim);

            let result = a.bundle(&b);

            prop_assert!(is_valid(&result));
            prop_assert_eq!(result.nnz(), a.nnz() + b.nnz());
        }
    }
}

/// Tests for operations between vectors with all overlapping blocks.
mod overlapping_blocks {
    use super::*;

    #[test]
    fn identical_bind_all_positive() {
        let sparse = SparseVec {
            pos: vec![0, 1, 2],
            neg: vec![3, 4, 5],
        };
        let v = BlockSparseTritVec::from_sparse(&sparse, 64);
        let result = v.bind(&v);

        assert!(is_valid(&result));
        // (+1)*( +1) = +1, (-1)*(-1) = +1
        let recovered = result.to_sparse();
        assert_eq!(recovered.pos.len(), 6);
        assert!(recovered.neg.is_empty());
    }

    #[test]
    fn opposite_bind_all_negative() {
        let sparse = SparseVec {
            pos: vec![0, 1, 2],
            neg: vec![3, 4, 5],
        };
        let v = BlockSparseTritVec::from_sparse(&sparse, 64);
        let negated = v.negate();
        let result = v.bind(&negated);

        assert!(is_valid(&result));
        // (+1)*(-1) = -1, (-1)*(+1) = -1
        let recovered = result.to_sparse();
        assert!(recovered.pos.is_empty());
        assert_eq!(recovered.neg.len(), 6);
    }

    #[test]
    fn identical_bundle_saturates() {
        let sparse = SparseVec {
            pos: vec![0, 1, 2],
            neg: vec![3, 4, 5],
        };
        let v = BlockSparseTritVec::from_sparse(&sparse, 64);
        let result = v.bundle(&v);

        assert!(is_valid(&result));
        // Saturating: +1 + +1 = +1, -1 + -1 = -1
        let recovered = result.to_sparse();
        assert_eq!(recovered.pos, vec![0, 1, 2]);
        assert_eq!(recovered.neg, vec![3, 4, 5]);
    }

    #[test]
    fn opposite_bundle_cancels() {
        let sparse = SparseVec {
            pos: vec![0, 1, 2],
            neg: vec![3, 4, 5],
        };
        let v = BlockSparseTritVec::from_sparse(&sparse, 64);
        let negated = v.negate();
        let result = v.bundle(&negated);

        assert!(is_valid(&result));
        // +1 + -1 = 0, -1 + +1 = 0
        assert_eq!(result.nnz(), 0);
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 128,
            .. ProptestConfig::default()
        })]

        /// Property: v.bind(v) produces all-positive output.
        #[test]
        fn prop_self_bind_all_positive(
            dim in 64usize..5_000,
            sparse in sparse_vec_strategy(100, 5_000)
        ) {
            let filtered = SparseVec {
                pos: sparse.pos.into_iter().filter(|&i| i < dim).collect(),
                neg: sparse.neg.into_iter().filter(|&i| i < dim).collect(),
            };
            let v = BlockSparseTritVec::from_sparse(&filtered, dim);

            let result = v.bind(&v);

            prop_assert!(is_valid(&result));
            let recovered = result.to_sparse();
            prop_assert!(recovered.neg.is_empty(), "self-bind should produce no negatives");
            prop_assert_eq!(recovered.pos.len(), v.nnz());
        }

        /// Property: v.bundle(v.negate()) produces empty.
        #[test]
        fn prop_bundle_with_negate_cancels(
            dim in 64usize..5_000,
            sparse in sparse_vec_strategy(100, 5_000)
        ) {
            let filtered = SparseVec {
                pos: sparse.pos.into_iter().filter(|&i| i < dim).collect(),
                neg: sparse.neg.into_iter().filter(|&i| i < dim).collect(),
            };
            let v = BlockSparseTritVec::from_sparse(&filtered, dim);

            let result = v.bundle(&v.negate());

            prop_assert!(is_valid(&result));
            prop_assert_eq!(result.nnz(), 0, "bundle with negate should cancel");
        }
    }
}

// ============================================================================
// MEMORY EFFICIENCY TESTS
// ============================================================================

/// Tests that verify block count stays proportional to density, not dimension.
mod memory_efficiency {
    use super::*;

    #[test]
    fn block_count_proportional_to_density() {
        // Same number of non-zero elements, different dimensions
        let nnz = 1000;
        let dims = [10_000, 100_000, 1_000_000, 10_000_000];

        for dim in dims {
            let pos: Vec<usize> = (0..nnz / 2)
                .map(|i| (i * dim / (nnz / 2)).min(dim - 1))
                .collect();
            let neg: Vec<usize> = (0..nnz / 2)
                .map(|i| ((i * dim / (nnz / 2)) + 1).min(dim - 1))
                .filter(|x| !pos.contains(x))
                .collect();

            let sparse = SparseVec { pos, neg };
            let v = BlockSparseTritVec::from_sparse(&sparse, dim);

            assert!(is_valid(&v));

            // Block count should be O(nnz/64) not O(dim/64)
            let expected_max_blocks = nnz; // Upper bound: 1 block per element
            assert!(
                v.block_count() <= expected_max_blocks,
                "dim={}, block_count={} exceeds expected_max={}",
                dim,
                v.block_count(),
                expected_max_blocks
            );
        }
    }

    #[test]
    fn memory_size_independent_of_dimension() {
        // Same density (0.01%), different dimensions
        let density = 0.0001;

        let small_dim = 100_000;
        let small_nnz = (small_dim as f64 * density) as usize;

        let large_dim = 10_000_000;
        let large_nnz = (large_dim as f64 * density) as usize;

        let make_vec = |dim: usize, nnz: usize| {
            let pos: Vec<usize> = (0..nnz / 2)
                .map(|i| (i * dim / (nnz / 2)).min(dim - 1))
                .collect();
            let neg: Vec<usize> = (0..nnz / 2)
                .map(|i| ((i * dim / (nnz / 2)) + 1).min(dim - 1))
                .filter(|x| !pos.contains(x))
                .collect();
            BlockSparseTritVec::from_sparse(&SparseVec { pos, neg }, dim)
        };

        let small_vec = make_vec(small_dim, small_nnz);
        let large_vec = make_vec(large_dim, large_nnz);

        // Both should have similar block counts (proportional to their nnz)
        let small_blocks = small_vec.block_count();
        let large_blocks = large_vec.block_count();

        // The ratio of block counts should be similar to ratio of nnz
        // (within some tolerance for block alignment effects)
        let nnz_ratio = large_nnz as f64 / small_nnz as f64;
        let block_ratio = large_blocks as f64 / small_blocks as f64;

        // Block ratio should be within 2x of nnz ratio
        assert!(
            block_ratio < nnz_ratio * 2.0 && block_ratio > nnz_ratio / 2.0,
            "Block count not proportional to density: nnz_ratio={}, block_ratio={}",
            nnz_ratio,
            block_ratio
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 32,
            .. ProptestConfig::default()
        })]

        /// Property: Block count is bounded by ceil(nnz / 64) * some_constant.
        #[test]
        fn prop_block_count_bounded_by_nnz(
            dim in 1_000usize..1_000_000,
            nnz in 10usize..1_000
        ) {
            // Generate nnz random indices
            let pos: Vec<usize> = (0..nnz / 2)
                .map(|i| (i * 7919) % dim) // Pseudo-random spread
                .collect();
            let neg: Vec<usize> = (0..nnz / 2)
                .map(|i| ((i * 7919) + 1) % dim)
                .filter(|x| !pos.contains(x))
                .collect();

            let sparse = SparseVec { pos, neg };
            let v = BlockSparseTritVec::from_sparse(&sparse, dim);

            // Block count should never exceed nnz (worst case: 1 element per block)
            let actual_nnz = v.nnz();
            prop_assert!(
                v.block_count() <= actual_nnz,
                "block_count={} > nnz={}",
                v.block_count(),
                actual_nnz
            );
        }
    }
}

// ============================================================================
// VALIDATION METHOD TESTS
// ============================================================================

/// Tests for the validation methods on BlockSparseTritVec.
mod validation_methods {
    use super::*;

    #[test]
    fn is_valid_returns_true_for_valid_vec() {
        let sparse = SparseVec {
            pos: vec![0, 64, 128],
            neg: vec![1, 65, 129],
        };
        let v = BlockSparseTritVec::from_sparse(&sparse, 1000);

        assert!(v.is_valid());
        assert!(v.validate().is_ok());
    }

    #[test]
    fn validate_returns_ok_for_valid_vec() {
        let v = BlockSparseTritVec::new(1000);
        assert!(v.validate().is_ok());
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 128,
            .. ProptestConfig::default()
        })]

        /// Property: is_valid() agrees with validate().is_ok()
        #[test]
        fn prop_is_valid_agrees_with_validate(
            dim in 64usize..10_000,
            sparse in sparse_vec_strategy(128, 10_000)
        ) {
            let filtered = SparseVec {
                pos: sparse.pos.into_iter().filter(|&i| i < dim).collect(),
                neg: sparse.neg.into_iter().filter(|&i| i < dim).collect(),
            };
            let v = BlockSparseTritVec::from_sparse(&filtered, dim);

            prop_assert_eq!(
                v.is_valid(),
                v.validate().is_ok(),
                "is_valid() and validate().is_ok() disagree"
            );
        }
    }
}

// ============================================================================
// BUNDLE_MANY TESTS
// ============================================================================

/// Tests for bundle_many operation.
mod bundle_many {
    use super::*;

    #[test]
    fn bundle_many_empty_returns_none() {
        let result = BlockSparseTritVec::bundle_many(&[]);
        assert!(result.is_none());
    }

    #[test]
    fn bundle_many_single_returns_clone() {
        let sparse = SparseVec {
            pos: vec![0, 1, 2],
            neg: vec![3, 4, 5],
        };
        let v = BlockSparseTritVec::from_sparse(&sparse, 64);
        let result = BlockSparseTritVec::bundle_many(&[v.clone()]).unwrap();

        assert!(is_valid(&result));
        assert_eq!(result, v);
    }

    #[test]
    fn bundle_many_preserves_validity() {
        let vecs: Vec<BlockSparseTritVec> = (0..10)
            .map(|i| {
                let sparse = SparseVec {
                    pos: vec![i * 64],
                    neg: vec![i * 64 + 1],
                };
                BlockSparseTritVec::from_sparse(&sparse, 1000)
            })
            .collect();

        let result = BlockSparseTritVec::bundle_many(&vecs).unwrap();

        assert!(is_valid(&result));
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        /// Property: bundle_many produces valid output.
        #[test]
        fn prop_bundle_many_valid(
            dim in 64usize..5_000,
            count in 2usize..20
        ) {
            let vecs: Vec<BlockSparseTritVec> = (0..count)
                .map(|i| {
                    let sparse = SparseVec {
                        pos: vec![(i * 64) % dim],
                        neg: vec![((i * 64) + 1) % dim],
                    };
                    BlockSparseTritVec::from_sparse(&sparse, dim)
                })
                .collect();

            let result = BlockSparseTritVec::bundle_many(&vecs).unwrap();

            prop_assert!(is_valid(&result));
            prop_assert_eq!(result.dim(), dim);
        }
    }
}

// ============================================================================
// DOT PRODUCT INVARIANTS
// ============================================================================

/// Tests for dot product correctness.
mod dot_product {
    use super::*;

    #[test]
    fn dot_with_self_equals_nnz() {
        let sparse = SparseVec {
            pos: vec![0, 1, 2],
            neg: vec![3, 4, 5],
        };
        let v = BlockSparseTritVec::from_sparse(&sparse, 64);

        // dot(v, v) = sum of (+1)*(+1) + (-1)*(-1) = count of non-zeros
        assert_eq!(v.dot(&v), v.nnz() as i64);
    }

    #[test]
    fn dot_with_negate_equals_negative_nnz() {
        let sparse = SparseVec {
            pos: vec![0, 1, 2],
            neg: vec![3, 4, 5],
        };
        let v = BlockSparseTritVec::from_sparse(&sparse, 64);
        let negated = v.negate();

        // dot(v, -v) = sum of (+1)*(-1) + (-1)*(+1) = -nnz
        assert_eq!(v.dot(&negated), -(v.nnz() as i64));
    }

    #[test]
    fn dot_disjoint_is_zero() {
        let sparse_a = SparseVec {
            pos: vec![0, 1],
            neg: vec![],
        };
        let sparse_b = SparseVec {
            pos: vec![64, 65],
            neg: vec![],
        };
        let a = BlockSparseTritVec::from_sparse(&sparse_a, 1000);
        let b = BlockSparseTritVec::from_sparse(&sparse_b, 1000);

        assert_eq!(a.dot(&b), 0);
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 128,
            .. ProptestConfig::default()
        })]

        /// Property: dot(v, v) == nnz(v)
        #[test]
        fn prop_dot_self_equals_nnz(
            dim in 64usize..5_000,
            sparse in sparse_vec_strategy(100, 5_000)
        ) {
            let filtered = SparseVec {
                pos: sparse.pos.into_iter().filter(|&i| i < dim).collect(),
                neg: sparse.neg.into_iter().filter(|&i| i < dim).collect(),
            };
            let v = BlockSparseTritVec::from_sparse(&filtered, dim);

            prop_assert_eq!(v.dot(&v), v.nnz() as i64);
        }

        /// Property: dot is commutative.
        #[test]
        fn prop_dot_commutative(
            dim in 64usize..5_000,
            sparse_a in sparse_vec_strategy(50, 5_000),
            sparse_b in sparse_vec_strategy(50, 5_000)
        ) {
            let a = BlockSparseTritVec::from_sparse(&SparseVec {
                pos: sparse_a.pos.into_iter().filter(|&i| i < dim).collect(),
                neg: sparse_a.neg.into_iter().filter(|&i| i < dim).collect(),
            }, dim);
            let b = BlockSparseTritVec::from_sparse(&SparseVec {
                pos: sparse_b.pos.into_iter().filter(|&i| i < dim).collect(),
                neg: sparse_b.neg.into_iter().filter(|&i| i < dim).collect(),
            }, dim);

            prop_assert_eq!(a.dot(&b), b.dot(&a));
        }
    }
}

// ============================================================================
// INSERT/REMOVE BLOCK INVARIANTS
// ============================================================================

/// Tests for insert_block and remove_block operations.
mod insert_remove {
    use super::*;

    #[test]
    fn insert_block_maintains_sorted_order() {
        let mut v = BlockSparseTritVec::new(10000);

        // Insert in random order
        v.insert_block(100, Block::new(0xFF, 0));
        v.insert_block(50, Block::new(0xFF, 0));
        v.insert_block(150, Block::new(0xFF, 0));
        v.insert_block(75, Block::new(0xFF, 0));

        assert!(is_valid(&v));
        assert!(is_sorted(v.blocks()));
    }

    #[test]
    fn insert_zero_block_removes_existing() {
        let mut v = BlockSparseTritVec::new(10000);
        v.insert_block(100, Block::new(0xFF, 0));

        assert_eq!(v.block_count(), 1);

        // Insert zero block should remove
        v.insert_block(100, Block::ZERO);

        assert!(is_valid(&v));
        assert_eq!(v.block_count(), 0);
    }

    #[test]
    fn remove_block_maintains_validity() {
        let mut v = BlockSparseTritVec::new(10000);
        v.insert_block(50, Block::new(0xFF, 0));
        v.insert_block(100, Block::new(0xFF, 0));
        v.insert_block(150, Block::new(0xFF, 0));

        let removed = v.remove_block(100);

        assert!(is_valid(&v));
        assert_eq!(v.block_count(), 2);
        assert!(removed.is_some());
        assert!(v.get_block(100).is_none());
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 128,
            .. ProptestConfig::default()
        })]

        /// Property: insert_block always maintains sorted order.
        #[test]
        fn prop_insert_maintains_sorted(
            initial_blocks in sorted_blocks_strategy(20, 1000),
            new_id in 0u32..1000,
            block in block_strategy()
        ) {
            let dim = 100_000;
            let mut v = BlockSparseTritVec::new(dim);

            // Insert initial blocks
            for (id, b) in initial_blocks {
                v.insert_block(id, b);
            }

            prop_assume!(is_valid(&v));

            // Insert new block
            v.insert_block(new_id, block);

            prop_assert!(is_valid(&v), "insert_block broke validity");
        }

        /// Property: remove_block maintains validity.
        #[test]
        fn prop_remove_maintains_valid(
            initial_blocks in sorted_blocks_strategy(20, 1000),
            remove_id in 0u32..1000
        ) {
            let dim = 100_000;
            let mut v = BlockSparseTritVec::new(dim);

            for (id, b) in initial_blocks {
                v.insert_block(id, b);
            }

            prop_assume!(is_valid(&v));

            v.remove_block(remove_id);

            prop_assert!(is_valid(&v), "remove_block broke validity");
        }
    }
}
