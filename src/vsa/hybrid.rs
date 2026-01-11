//! Hybrid Ternary Vector — Automatic Representation Selection
//!
//! This module provides a unified interface that automatically selects between
//! sparse and bitsliced representations based on vector density. This achieves
//! optimal performance across all sparsity levels:
//!
//! - **Sparse** (ρ < 0.5%): O(nnz) operations via sorted index merge
//! - **Bitsliced** (ρ ≥ 0.5%): O(D/64) operations via SIMD-friendly bit-planes
//!
//! # Mathematical Basis
//!
//! The crossover point is derived from complexity analysis:
//!
//! ```text
//! Sparse cost:    c_s · ρD    (branch-heavy sorted merge)
//! Bitsliced cost: c_b · D/64  (branchless ALU ops)
//!
//! Crossover: ρ < c_b / (64 · c_s) ≈ 0.5%
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use embeddenator::{HybridTritVec, SparseVec, DIM};
//!
//! let sparse = SparseVec::random();
//! let hybrid = HybridTritVec::from_sparse(sparse, DIM);
//!
//! // Operations auto-dispatch to optimal representation
//! let bound = hybrid.bind(&other_hybrid, DIM);
//! let bundled = hybrid.bundle(&other_hybrid, DIM);
//! ```

use crate::bitsliced::BitslicedTritVec;
use crate::block_sparse::BlockSparseTritVec;
use crate::vsa::{SparseVec, DIM};
use serde::{Deserialize, Serialize};

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Density threshold for sparse → bitsliced transition.
///
/// Below this threshold, sparse operations (O(nnz)) outperform bitsliced (O(D/64)).
/// Derived from empirical benchmarks: sparse wins when nnz < ~50 at D=10K.
pub const DENSITY_THRESHOLD: f64 = 0.005; // 0.5%

/// Minimum dimension for bitsliced to be worthwhile.
/// Below this, the conversion overhead exceeds operation savings.
pub const MIN_BITSLICED_DIM: usize = 256;

/// Minimum dimension for block-sparse to be worthwhile (> 100K).
///
/// Block-sparse representation excels at massive dimensions where dense
/// bitsliced would require prohibitive memory.
pub const MIN_BLOCK_SPARSE_DIM: usize = 100_000;

/// Density threshold below which block-sparse beats bitsliced for large dims.
///
/// At 1% density and 1M dimensions, block-sparse uses ~160KB vs ~31MB for dense.
pub const BLOCK_SPARSE_DENSITY_THRESHOLD: f64 = 0.01; // 1%

// ============================================================================
// HYBRID REPRESENTATION
// ============================================================================

/// Hybrid ternary vector with automatic representation selection.
///
/// Transparently wraps either `SparseVec` or `BitslicedTritVec`, selecting
/// the optimal representation based on density and operation patterns.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum HybridTritVec {
    /// Sparse representation for low-density vectors (ρ < 0.5%)
    Sparse(SparseVec),
    /// Bitsliced representation for dense vectors (ρ ≥ 0.5%)
    Bitsliced(BitslicedTritVec),
    /// Block-sparse representation for massive dimensions (D ≥ 100K) with low density
    BlockSparse(BlockSparseTritVec),
}

impl HybridTritVec {
    // ========================================================================
    // CONSTRUCTION
    // ========================================================================

    /// Create from sparse vector, auto-selecting representation.
    ///
    /// # Arguments
    /// * `sparse` - Source sparse vector
    /// * `dim` - Total dimension (needed for density calculation)
    ///
    /// # Selection Logic
    /// - If `dim < MIN_BITSLICED_DIM`: Always sparse
    /// - If `dim >= MIN_BLOCK_SPARSE_DIM` AND `density < 1%`: Block-sparse
    /// - If `nnz/dim < DENSITY_THRESHOLD`: Sparse
    /// - Otherwise: Convert to bitsliced
    pub fn from_sparse(sparse: SparseVec, dim: usize) -> Self {
        let nnz = sparse.pos.len() + sparse.neg.len();
        let density = nnz as f64 / dim as f64;

        if dim < MIN_BITSLICED_DIM {
            HybridTritVec::Sparse(sparse)
        } else if dim >= MIN_BLOCK_SPARSE_DIM && density < BLOCK_SPARSE_DENSITY_THRESHOLD {
            HybridTritVec::BlockSparse(BlockSparseTritVec::from_sparse(&sparse, dim))
        } else if density < DENSITY_THRESHOLD {
            HybridTritVec::Sparse(sparse)
        } else {
            HybridTritVec::Bitsliced(BitslicedTritVec::from_sparse(&sparse, dim))
        }
    }

    /// Create from bitsliced vector (always stays bitsliced).
    pub fn from_bitsliced(bitsliced: BitslicedTritVec) -> Self {
        HybridTritVec::Bitsliced(bitsliced)
    }

    /// Create from block-sparse vector (always stays block-sparse).
    pub fn from_block_sparse(block_sparse: BlockSparseTritVec) -> Self {
        HybridTritVec::BlockSparse(block_sparse)
    }

    /// Create empty/zero vector as sparse.
    pub fn new_empty() -> Self {
        HybridTritVec::Sparse(SparseVec::new())
    }

    /// Create zero vector with specified dimension as bitsliced.
    pub fn new_zero(dim: usize) -> Self {
        if dim < MIN_BITSLICED_DIM {
            HybridTritVec::Sparse(SparseVec::new())
        } else if dim >= MIN_BLOCK_SPARSE_DIM {
            HybridTritVec::BlockSparse(BlockSparseTritVec::new(dim))
        } else {
            HybridTritVec::Bitsliced(BitslicedTritVec::new_zero(dim))
        }
    }

    // ========================================================================
    // REPRESENTATION ACCESS
    // ========================================================================

    /// Check if currently using sparse representation.
    #[inline]
    pub fn is_sparse(&self) -> bool {
        matches!(self, HybridTritVec::Sparse(_))
    }

    /// Check if currently using bitsliced representation.
    #[inline]
    pub fn is_bitsliced(&self) -> bool {
        matches!(self, HybridTritVec::Bitsliced(_))
    }

    /// Check if currently using block-sparse representation.
    #[inline]
    pub fn is_block_sparse(&self) -> bool {
        matches!(self, HybridTritVec::BlockSparse(_))
    }

    /// Get current density estimate.
    pub fn density(&self, dim: usize) -> f64 {
        match self {
            HybridTritVec::Sparse(s) => (s.pos.len() + s.neg.len()) as f64 / dim as f64,
            HybridTritVec::Bitsliced(b) => b.nnz() as f64 / b.len() as f64,
            HybridTritVec::BlockSparse(bs) => bs.nnz() as f64 / bs.dim() as f64,
        }
    }

    /// Get number of non-zero elements.
    pub fn nnz(&self, _dim: usize) -> usize {
        match self {
            HybridTritVec::Sparse(s) => s.pos.len() + s.neg.len(),
            HybridTritVec::Bitsliced(b) => b.nnz(),
            HybridTritVec::BlockSparse(bs) => bs.nnz(),
        }
    }

    /// Force conversion to bitsliced representation.
    ///
    /// Use this before batch operations where bitsliced is always faster.
    pub fn to_bitsliced(&self, dim: usize) -> BitslicedTritVec {
        match self {
            HybridTritVec::Sparse(s) => BitslicedTritVec::from_sparse(s, dim),
            HybridTritVec::Bitsliced(b) => b.clone(),
            HybridTritVec::BlockSparse(bs) => bs.to_bitsliced(),
        }
    }

    /// Force conversion to sparse representation.
    ///
    /// Use this for serialization or when memory is constrained.
    pub fn to_sparse(&self) -> SparseVec {
        match self {
            HybridTritVec::Sparse(s) => s.clone(),
            HybridTritVec::Bitsliced(b) => b.to_sparse(),
            HybridTritVec::BlockSparse(bs) => bs.to_sparse(),
        }
    }

    /// Force conversion to block-sparse representation.
    ///
    /// Use this for massive dimensions with low density.
    pub fn to_block_sparse(&self, dim: usize) -> BlockSparseTritVec {
        match self {
            HybridTritVec::Sparse(s) => BlockSparseTritVec::from_sparse(s, dim),
            HybridTritVec::Bitsliced(b) => BlockSparseTritVec::from_bitsliced(b),
            HybridTritVec::BlockSparse(bs) => bs.clone(),
        }
    }

    /// Get reference to inner bitsliced (if applicable).
    pub fn as_bitsliced_ref(&self) -> Option<&BitslicedTritVec> {
        match self {
            HybridTritVec::Bitsliced(b) => Some(b),
            _ => None,
        }
    }

    /// Get reference to inner block-sparse (if applicable).
    pub fn as_block_sparse_ref(&self) -> Option<&BlockSparseTritVec> {
        match self {
            HybridTritVec::BlockSparse(bs) => Some(bs),
            _ => None,
        }
    }

    // ========================================================================
    // VSA OPERATIONS
    // ========================================================================

    /// Bind operation (⊙): Element-wise multiplication.
    ///
    /// # Mathematical Definition
    /// $(a \odot b)_i = a_i \cdot b_i$
    ///
    /// # Dispatch Logic
    /// - Both sparse: Use sparse bind, then re-evaluate density
    /// - Both block-sparse: Use SIMD-dispatched block-sparse bind
    /// - BlockSparse × Other: Convert based on dimension
    /// - Otherwise: Convert to bitsliced and use SIMD-optimized path
    pub fn bind(&self, other: &Self, dim: usize) -> Self {
        match (self, other) {
            (HybridTritVec::Sparse(a), HybridTritVec::Sparse(b)) => {
                let result = a.bind(b);
                HybridTritVec::from_sparse(result, dim)
            }
            (HybridTritVec::BlockSparse(a), HybridTritVec::BlockSparse(b)) => {
                HybridTritVec::BlockSparse(a.bind_dispatch(b))
            }
            (HybridTritVec::BlockSparse(_), _) | (_, HybridTritVec::BlockSparse(_)) => {
                // Mixed with block-sparse: choose representation by dimension
                if dim >= MIN_BLOCK_SPARSE_DIM {
                    let a_bs = self.to_block_sparse(dim);
                    let b_bs = other.to_block_sparse(dim);
                    HybridTritVec::BlockSparse(a_bs.bind_dispatch(&b_bs))
                } else {
                    let a_bit = self.to_bitsliced(dim);
                    let b_bit = other.to_bitsliced(dim);
                    HybridTritVec::Bitsliced(a_bit.bind(&b_bit))
                }
            }
            _ => {
                // At least one is bitsliced, use bitsliced path
                let a_bs = self.to_bitsliced(dim);
                let b_bs = other.to_bitsliced(dim);
                HybridTritVec::Bitsliced(a_bs.bind(&b_bs))
            }
        }
    }

    /// Bundle operation (⊕): Element-wise saturating addition.
    ///
    /// # Mathematical Definition
    /// $(a \oplus b)_i = \text{clamp}(a_i + b_i, -1, +1)$
    ///
    /// Conflict-cancel semantics: +1 + (-1) = 0
    pub fn bundle(&self, other: &Self, dim: usize) -> Self {
        match (self, other) {
            (HybridTritVec::Sparse(a), HybridTritVec::Sparse(b)) => {
                let result = a.bundle(b);
                HybridTritVec::from_sparse(result, dim)
            }
            (HybridTritVec::BlockSparse(a), HybridTritVec::BlockSparse(b)) => {
                HybridTritVec::BlockSparse(a.bundle_dispatch(b))
            }
            (HybridTritVec::BlockSparse(_), _) | (_, HybridTritVec::BlockSparse(_)) => {
                if dim >= MIN_BLOCK_SPARSE_DIM {
                    let a_bs = self.to_block_sparse(dim);
                    let b_bs = other.to_block_sparse(dim);
                    HybridTritVec::BlockSparse(a_bs.bundle_dispatch(&b_bs))
                } else {
                    let a_bit = self.to_bitsliced(dim);
                    let b_bit = other.to_bitsliced(dim);
                    HybridTritVec::Bitsliced(a_bit.bundle(&b_bit))
                }
            }
            _ => {
                let a_bs = self.to_bitsliced(dim);
                let b_bs = other.to_bitsliced(dim);
                HybridTritVec::Bitsliced(a_bs.bundle(&b_bs))
            }
        }
    }

    /// Dot product: $\langle a, b \rangle = \sum_i a_i \cdot b_i$
    pub fn dot(&self, other: &Self, dim: usize) -> i64 {
        match (self, other) {
            (HybridTritVec::BlockSparse(a), HybridTritVec::BlockSparse(b)) => a.dot_dispatch(b),
            (HybridTritVec::BlockSparse(_), _) | (_, HybridTritVec::BlockSparse(_)) => {
                if dim >= MIN_BLOCK_SPARSE_DIM {
                    let a_bs = self.to_block_sparse(dim);
                    let b_bs = other.to_block_sparse(dim);
                    a_bs.dot_dispatch(&b_bs)
                } else {
                    let a_bit = self.to_bitsliced(dim);
                    let b_bit = other.to_bitsliced(dim);
                    a_bit.dot(&b_bit) as i64
                }
            }
            _ => {
                let a_bs = self.to_bitsliced(dim);
                let b_bs = other.to_bitsliced(dim);
                a_bs.dot(&b_bs) as i64
            }
        }
    }

    /// Cosine similarity: $\cos(a, b) = \frac{\langle a, b \rangle}{\sqrt{|a|_0 \cdot |b|_0}}$
    pub fn cosine(&self, other: &Self, dim: usize) -> f64 {
        match (self, other) {
            (HybridTritVec::Sparse(a), HybridTritVec::Sparse(b)) => a.cosine(b),
            (HybridTritVec::BlockSparse(a), HybridTritVec::BlockSparse(b)) => a.cosine_dispatch(b),
            (HybridTritVec::BlockSparse(_), _) | (_, HybridTritVec::BlockSparse(_)) => {
                if dim >= MIN_BLOCK_SPARSE_DIM {
                    let a_bs = self.to_block_sparse(dim);
                    let b_bs = other.to_block_sparse(dim);
                    a_bs.cosine_dispatch(&b_bs)
                } else {
                    let a_bit = self.to_bitsliced(dim);
                    let b_bit = other.to_bitsliced(dim);
                    a_bit.cosine(&b_bit)
                }
            }
            _ => {
                let a_bs = self.to_bitsliced(dim);
                let b_bs = other.to_bitsliced(dim);
                a_bs.cosine(&b_bs)
            }
        }
    }

    /// Permute (cyclic shift): $\pi_k(v)_i = v_{(i-k) \mod D}$
    ///
    /// Uses optimized word-level rotation for bitsliced vectors.
    pub fn permute(&self, shift: usize, dim: usize) -> Self {
        match self {
            HybridTritVec::Sparse(s) => {
                // SparseVec.permute uses global DIM, so we use it directly
                let result = s.permute(shift);
                HybridTritVec::from_sparse(result, dim)
            }
            HybridTritVec::Bitsliced(b) => HybridTritVec::Bitsliced(b.permute_optimized(shift)),
            HybridTritVec::BlockSparse(bs) => {
                // Block-sparse doesn't have optimized permute; convert to sparse
                let sparse = bs.to_sparse();
                let permuted = sparse.permute(shift);
                HybridTritVec::from_sparse(permuted, dim)
            }
        }
    }

    /// Negate all trits: $-v_i$ for all $i$
    pub fn negate(&self) -> Self {
        match self {
            HybridTritVec::Sparse(s) => {
                // Swap pos and neg indices
                HybridTritVec::Sparse(SparseVec {
                    pos: s.neg.clone(),
                    neg: s.pos.clone(),
                })
            }
            HybridTritVec::Bitsliced(b) => HybridTritVec::Bitsliced(b.negate()),
            HybridTritVec::BlockSparse(bs) => HybridTritVec::BlockSparse(bs.negate()),
        }
    }
}

// ============================================================================
// BATCH OPERATIONS
// ============================================================================

impl HybridTritVec {
    /// Bundle multiple vectors using carry-save accumulation.
    ///
    /// More accurate than sequential bundling for N > 2 vectors
    /// because it tracks vote counts before final majority decision.
    ///
    /// # Mathematical Basis
    /// For N vectors, each position accumulates votes:
    /// - pos_votes[i] = count of vectors with +1 at position i
    /// - neg_votes[i] = count of vectors with -1 at position i
    /// - Result: +1 if pos > neg, -1 if neg > pos, 0 otherwise
    pub fn bundle_many<'a>(vecs: impl IntoIterator<Item = &'a Self>, dim: usize) -> Self {
        use crate::bitsliced::CarrySaveBundle;

        let mut acc = CarrySaveBundle::new(dim);

        for v in vecs {
            let bs = v.to_bitsliced(dim);
            acc.accumulate(&bs);
        }

        HybridTritVec::Bitsliced(acc.finalize())
    }

    /// Bind a sequence of vectors.
    ///
    /// Since bind is associative: $(a \odot b) \odot c = a \odot (b \odot c)$
    pub fn bind_many<'a>(vecs: impl IntoIterator<Item = &'a Self>, dim: usize) -> Self {
        let mut iter = vecs.into_iter();

        let first = match iter.next() {
            Some(v) => v.to_bitsliced(dim),
            None => return HybridTritVec::new_zero(dim),
        };

        let result = iter.fold(first, |acc: BitslicedTritVec, v: &Self| {
            let v_bs = v.to_bitsliced(dim);
            acc.bind(&v_bs)
        });

        HybridTritVec::Bitsliced(result)
    }
}

// ============================================================================
// CONVERSION TRAITS
// ============================================================================

impl From<SparseVec> for HybridTritVec {
    fn from(sparse: SparseVec) -> Self {
        // Without dimension info, default to DIM constant
        HybridTritVec::from_sparse(sparse, DIM)
    }
}

impl From<BitslicedTritVec> for HybridTritVec {
    fn from(bitsliced: BitslicedTritVec) -> Self {
        HybridTritVec::Bitsliced(bitsliced)
    }
}

impl From<BlockSparseTritVec> for HybridTritVec {
    fn from(block_sparse: BlockSparseTritVec) -> Self {
        HybridTritVec::BlockSparse(block_sparse)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_density_selection() {
        // Very sparse: should stay sparse
        let sparse = SparseVec {
            pos: vec![0, 100],
            neg: vec![50],
        };
        let hybrid = HybridTritVec::from_sparse(sparse, 10000);
        assert!(hybrid.is_sparse());

        // Dense: should convert to bitsliced
        let dense_pos: Vec<usize> = (0..500).collect();
        let dense_neg: Vec<usize> = (500..1000).collect();
        let dense = SparseVec {
            pos: dense_pos,
            neg: dense_neg,
        };
        let hybrid = HybridTritVec::from_sparse(dense, 10000);
        assert!(hybrid.is_bitsliced());
    }

    #[test]
    fn test_bind_equivalence() {
        let a = SparseVec {
            pos: vec![0, 10, 100],
            neg: vec![5, 50],
        };
        let b = SparseVec {
            pos: vec![0, 5, 100],
            neg: vec![10, 50],
        };

        // Sparse bind
        let sparse_result = a.bind(&b);

        // Hybrid bind (should use sparse path)
        let ha = HybridTritVec::from_sparse(a.clone(), DIM);
        let hb = HybridTritVec::from_sparse(b.clone(), DIM);
        let hybrid_result = ha.bind(&hb, DIM);

        // Results should match
        let sparse_from_hybrid = hybrid_result.to_sparse();
        assert_eq!(sparse_result.pos, sparse_from_hybrid.pos);
        assert_eq!(sparse_result.neg, sparse_from_hybrid.neg);
    }

    #[test]
    fn test_bundle_equivalence() {
        let a = SparseVec {
            pos: vec![0, 10, 100],
            neg: vec![5, 50],
        };
        let b = SparseVec {
            pos: vec![0, 5, 100],
            neg: vec![10, 50],
        };

        // Sparse bundle
        let sparse_result = a.bundle(&b);

        // Hybrid bundle
        let ha = HybridTritVec::from_sparse(a.clone(), DIM);
        let hb = HybridTritVec::from_sparse(b.clone(), DIM);
        let hybrid_result = ha.bundle(&hb, DIM);

        // Results should match
        let sparse_from_hybrid = hybrid_result.to_sparse();
        assert_eq!(sparse_result.pos, sparse_from_hybrid.pos);
        assert_eq!(sparse_result.neg, sparse_from_hybrid.neg);
    }

    #[test]
    fn test_cosine_equivalence() {
        let a = SparseVec {
            pos: vec![0, 10, 100],
            neg: vec![5, 50],
        };
        let b = SparseVec {
            pos: vec![0, 5, 100],
            neg: vec![10, 50],
        };

        let sparse_cos = a.cosine(&b);

        let ha = HybridTritVec::from_sparse(a, DIM);
        let hb = HybridTritVec::from_sparse(b, DIM);
        let hybrid_cos = ha.cosine(&hb, DIM);

        assert!((sparse_cos - hybrid_cos).abs() < 1e-10);
    }

    #[test]
    fn test_cross_representation_bind() {
        // Test bind between sparse and bitsliced representations
        let sparse_vec = SparseVec {
            pos: vec![0, 10, 100],
            neg: vec![5, 50],
        };

        // Create one sparse and one dense (bitsliced) hybrid
        let ha = HybridTritVec::from_sparse(sparse_vec.clone(), DIM);
        assert!(ha.is_sparse());

        // Force bitsliced for second operand
        let hb = HybridTritVec::Bitsliced(BitslicedTritVec::from_sparse(&sparse_vec, DIM));
        assert!(hb.is_bitsliced());

        // Cross-representation bind should work
        let result = ha.bind(&hb, DIM);

        // Bind with self should give all +1s at non-zero positions (self-inverse property)
        let result_sparse = result.to_sparse();
        // All positions should be positive (P * P = P, N * N = P)
        assert!(result_sparse.neg.is_empty());
        assert_eq!(
            result_sparse.pos.len(),
            sparse_vec.pos.len() + sparse_vec.neg.len()
        );
    }

    #[test]
    fn test_bundle_many() {
        let vecs: Vec<SparseVec> = (0..5)
            .map(|i| SparseVec {
                pos: vec![i * 10, i * 10 + 1],
                neg: vec![i * 10 + 5],
            })
            .collect();

        let hybrids: Vec<HybridTritVec> = vecs
            .iter()
            .map(|v| HybridTritVec::from_sparse(v.clone(), DIM))
            .collect();

        let result = HybridTritVec::bundle_many(hybrids.iter(), DIM);

        // Result should be bitsliced (bundle_many always uses carry-save)
        assert!(result.is_bitsliced());

        // Should have non-zero elements
        assert!(result.nnz(DIM) > 0);
    }

    #[test]
    fn test_small_dimension_stays_sparse() {
        // Dimensions below MIN_BITSLICED_DIM should always stay sparse
        let dense = SparseVec {
            pos: (0..100).collect(),
            neg: (100..200).collect(),
        };

        // Even though density is high, small dim should stay sparse
        let hybrid = HybridTritVec::from_sparse(dense, 200);
        assert!(
            hybrid.is_sparse(),
            "Small dim should stay sparse regardless of density"
        );
    }

    #[test]
    fn test_negate() {
        let sparse = SparseVec {
            pos: vec![0, 10],
            neg: vec![5, 50],
        };

        let hybrid = HybridTritVec::from_sparse(sparse.clone(), DIM);
        let negated = hybrid.negate();
        let negated_sparse = negated.to_sparse();

        // Pos and neg should be swapped
        assert_eq!(negated_sparse.pos, sparse.neg);
        assert_eq!(negated_sparse.neg, sparse.pos);
    }

    #[test]
    fn test_permute_hybrid() {
        let sparse = SparseVec {
            pos: vec![0, 100],
            neg: vec![50],
        };

        let hybrid = HybridTritVec::from_sparse(sparse.clone(), DIM);
        let permuted = hybrid.permute(100, DIM);

        // nnz should be preserved
        assert_eq!(permuted.nnz(DIM), hybrid.nnz(DIM));
    }

    // ========================================================================
    // BLOCK-SPARSE TESTS
    // ========================================================================

    #[test]
    fn test_block_sparse_selection_large_dim() {
        // At MIN_BLOCK_SPARSE_DIM with < 1% density → should select block-sparse
        let large_dim = MIN_BLOCK_SPARSE_DIM;
        let sparse = SparseVec {
            pos: vec![0, 1000, 50000],
            neg: vec![500, 75000],
        };
        // density = 5 / 100_000 = 0.00005 = 0.005% < 1%
        let hybrid = HybridTritVec::from_sparse(sparse, large_dim);
        assert!(
            hybrid.is_block_sparse(),
            "Large dim with low density should use block-sparse"
        );
    }

    #[test]
    fn test_block_sparse_not_selected_below_threshold() {
        // Below MIN_BLOCK_SPARSE_DIM should not use block-sparse
        let moderate_dim = 50_000; // < 100K
        let sparse = SparseVec {
            pos: vec![0, 100],
            neg: vec![50],
        };
        let hybrid = HybridTritVec::from_sparse(sparse, moderate_dim);
        assert!(
            !hybrid.is_block_sparse(),
            "Moderate dim should not use block-sparse"
        );
        // Should be sparse due to low density
        assert!(hybrid.is_sparse());
    }

    #[test]
    fn test_block_sparse_bind() {
        let large_dim = MIN_BLOCK_SPARSE_DIM;
        let a = SparseVec {
            pos: vec![0, 64, 1000],
            neg: vec![32, 500],
        };
        let b = SparseVec {
            pos: vec![0, 32, 1000],
            neg: vec![64, 500],
        };

        let ha = HybridTritVec::from_sparse(a.clone(), large_dim);
        let hb = HybridTritVec::from_sparse(b.clone(), large_dim);

        assert!(ha.is_block_sparse());
        assert!(hb.is_block_sparse());

        let result = ha.bind(&hb, large_dim);

        // Result should be block-sparse
        assert!(result.is_block_sparse());

        // Verify bind semantics: pos*pos=pos, neg*neg=pos, pos*neg=neg
        let result_sparse = result.to_sparse();
        // Position 0: +1 * +1 = +1
        assert!(result_sparse.pos.contains(&0));
        // Position 64: +1 * -1 = -1
        assert!(result_sparse.neg.contains(&64));
        // Position 32: -1 * +1 = -1
        assert!(result_sparse.neg.contains(&32));
        // Position 500: -1 * -1 = +1
        assert!(result_sparse.pos.contains(&500));
    }

    #[test]
    fn test_block_sparse_bundle() {
        let large_dim = MIN_BLOCK_SPARSE_DIM;
        let a = SparseVec {
            pos: vec![0, 100],
            neg: vec![200],
        };
        let b = SparseVec {
            pos: vec![0, 200], // pos 0: +1+1=+1, neg 200: -1+1=0
            neg: vec![100],    // pos 100: +1-1=0
        };

        let ha = HybridTritVec::from_sparse(a, large_dim);
        let hb = HybridTritVec::from_sparse(b, large_dim);

        let result = ha.bundle(&hb, large_dim);
        assert!(result.is_block_sparse());

        let result_sparse = result.to_sparse();
        // Position 0: both +1 → +1
        assert!(result_sparse.pos.contains(&0));
        // Position 100: +1 + (-1) = 0 (cancels)
        assert!(!result_sparse.pos.contains(&100));
        assert!(!result_sparse.neg.contains(&100));
        // Position 200: -1 + +1 = 0 (cancels)
        assert!(!result_sparse.pos.contains(&200));
        assert!(!result_sparse.neg.contains(&200));
    }

    #[test]
    fn test_block_sparse_cosine() {
        let large_dim = MIN_BLOCK_SPARSE_DIM;
        let sparse = SparseVec {
            pos: vec![0, 100, 1000],
            neg: vec![50, 500],
        };

        let ha = HybridTritVec::from_sparse(sparse.clone(), large_dim);
        let hb = HybridTritVec::from_sparse(sparse.clone(), large_dim);

        // Self-similarity should be 1.0
        let cos = ha.cosine(&hb, large_dim);
        assert!(
            (cos - 1.0).abs() < 1e-10,
            "Self-similarity should be 1.0, got {}",
            cos
        );
    }

    #[test]
    fn test_block_sparse_dot() {
        let large_dim = MIN_BLOCK_SPARSE_DIM;
        let a = SparseVec {
            pos: vec![0, 100],
            neg: vec![200],
        };
        let b = SparseVec {
            pos: vec![0, 200],
            neg: vec![100],
        };

        let ha = HybridTritVec::from_sparse(a, large_dim);
        let hb = HybridTritVec::from_sparse(b, large_dim);

        let dot = ha.dot(&hb, large_dim);
        // pos 0: +1 * +1 = +1
        // pos 100: +1 * -1 = -1
        // pos 200: -1 * +1 = -1
        // Total: +1 - 1 - 1 = -1
        assert_eq!(dot, -1);
    }

    #[test]
    fn test_block_sparse_negate() {
        let large_dim = MIN_BLOCK_SPARSE_DIM;
        let sparse = SparseVec {
            pos: vec![0, 100],
            neg: vec![50, 500],
        };

        let hybrid = HybridTritVec::from_sparse(sparse.clone(), large_dim);
        assert!(hybrid.is_block_sparse());

        let negated = hybrid.negate();
        assert!(negated.is_block_sparse());

        let negated_sparse = negated.to_sparse();
        // Pos and neg should be swapped
        assert_eq!(negated_sparse.pos, sparse.neg);
        assert_eq!(negated_sparse.neg, sparse.pos);
    }

    #[test]
    fn test_block_sparse_permute() {
        let large_dim = MIN_BLOCK_SPARSE_DIM;
        let sparse = SparseVec {
            pos: vec![0, 100],
            neg: vec![50],
        };

        let hybrid = HybridTritVec::from_sparse(sparse, large_dim);
        assert!(hybrid.is_block_sparse());

        let permuted = hybrid.permute(64, large_dim);

        // nnz should be preserved
        assert_eq!(permuted.nnz(large_dim), hybrid.nnz(large_dim));
    }

    #[test]
    fn test_cross_representation_block_sparse_and_sparse() {
        let large_dim = MIN_BLOCK_SPARSE_DIM;
        let sparse_vec = SparseVec {
            pos: vec![0, 100],
            neg: vec![50],
        };

        // Create block-sparse (large dim, low density)
        let ha = HybridTritVec::from_sparse(sparse_vec.clone(), large_dim);
        assert!(ha.is_block_sparse());

        // Force sparse by using small dim
        let hb = HybridTritVec::Sparse(sparse_vec.clone());
        assert!(hb.is_sparse());

        // Cross-representation bind should work
        let result = ha.bind(&hb, large_dim);

        // At large dim, should convert to block-sparse
        assert!(result.is_block_sparse());

        // Self-bind should give all positives
        let result_sparse = result.to_sparse();
        assert!(result_sparse.neg.is_empty());
    }

    #[test]
    fn test_cross_representation_block_sparse_and_bitsliced() {
        let large_dim = MIN_BLOCK_SPARSE_DIM;
        let sparse_vec = SparseVec {
            pos: vec![0, 100],
            neg: vec![50],
        };

        // Create block-sparse
        let ha = HybridTritVec::from_sparse(sparse_vec.clone(), large_dim);
        assert!(ha.is_block_sparse());

        // Force bitsliced
        let hb = HybridTritVec::Bitsliced(BitslicedTritVec::from_sparse(&sparse_vec, large_dim));
        assert!(hb.is_bitsliced());

        // Cross-representation bind at large dim → should use block-sparse
        let result = ha.bind(&hb, large_dim);
        assert!(result.is_block_sparse());

        // Cross-representation at small dim → should use bitsliced
        let small_dim = 1000;
        let result_small = ha.bind(&hb, small_dim);
        assert!(result_small.is_bitsliced());
    }

    #[test]
    fn test_to_block_sparse_conversion() {
        // Test conversion from all variants to block-sparse
        let dim = 10000;
        let sparse = SparseVec {
            pos: vec![0, 100],
            neg: vec![50],
        };

        // From Sparse
        let h_sparse = HybridTritVec::Sparse(sparse.clone());
        let bs1 = h_sparse.to_block_sparse(dim);
        assert_eq!(bs1.nnz(), 3);

        // From Bitsliced
        let h_bitsliced = HybridTritVec::Bitsliced(BitslicedTritVec::from_sparse(&sparse, dim));
        let bs2 = h_bitsliced.to_block_sparse(dim);
        assert_eq!(bs2.nnz(), 3);

        // From BlockSparse (should clone)
        let h_blocksparse =
            HybridTritVec::BlockSparse(BlockSparseTritVec::from_sparse(&sparse, dim));
        let bs3 = h_blocksparse.to_block_sparse(dim);
        assert_eq!(bs3.nnz(), 3);
    }

    #[test]
    fn test_from_block_sparse_constructor() {
        let dim = MIN_BLOCK_SPARSE_DIM;
        let sparse = SparseVec {
            pos: vec![0, 100],
            neg: vec![50],
        };

        let block_sparse = BlockSparseTritVec::from_sparse(&sparse, dim);
        let hybrid = HybridTritVec::from_block_sparse(block_sparse);

        assert!(hybrid.is_block_sparse());
        assert_eq!(hybrid.nnz(dim), 3);
    }

    #[test]
    fn test_block_sparse_from_trait() {
        let dim = MIN_BLOCK_SPARSE_DIM;
        let sparse = SparseVec {
            pos: vec![0, 100],
            neg: vec![50],
        };

        let block_sparse = BlockSparseTritVec::from_sparse(&sparse, dim);
        let hybrid: HybridTritVec = block_sparse.into();

        assert!(hybrid.is_block_sparse());
    }

    #[test]
    fn test_block_sparse_density_and_nnz() {
        let dim = MIN_BLOCK_SPARSE_DIM;
        let sparse = SparseVec {
            pos: vec![0, 100, 1000, 5000],
            neg: vec![50, 500],
        };

        let hybrid = HybridTritVec::from_sparse(sparse, dim);
        assert!(hybrid.is_block_sparse());

        assert_eq!(hybrid.nnz(dim), 6);

        let density = hybrid.density(dim);
        let expected_density = 6.0 / dim as f64;
        assert!((density - expected_density).abs() < 1e-15);
    }
}
