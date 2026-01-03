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
    /// - If `nnz/dim < DENSITY_THRESHOLD`: Sparse
    /// - Otherwise: Convert to bitsliced
    pub fn from_sparse(sparse: SparseVec, dim: usize) -> Self {
        let nnz = sparse.pos.len() + sparse.neg.len();
        let density = nnz as f64 / dim as f64;

        if dim < MIN_BITSLICED_DIM || density < DENSITY_THRESHOLD {
            HybridTritVec::Sparse(sparse)
        } else {
            HybridTritVec::Bitsliced(BitslicedTritVec::from_sparse(&sparse, dim))
        }
    }

    /// Create from bitsliced vector (always stays bitsliced).
    pub fn from_bitsliced(bitsliced: BitslicedTritVec) -> Self {
        HybridTritVec::Bitsliced(bitsliced)
    }

    /// Create empty/zero vector as sparse.
    pub fn new_empty() -> Self {
        HybridTritVec::Sparse(SparseVec::new())
    }

    /// Create zero vector with specified dimension as bitsliced.
    pub fn new_zero(dim: usize) -> Self {
        if dim < MIN_BITSLICED_DIM {
            HybridTritVec::Sparse(SparseVec::new())
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

    /// Get current density estimate.
    pub fn density(&self, dim: usize) -> f64 {
        match self {
            HybridTritVec::Sparse(s) => (s.pos.len() + s.neg.len()) as f64 / dim as f64,
            HybridTritVec::Bitsliced(b) => b.nnz() as f64 / b.len() as f64,
        }
    }

    /// Get number of non-zero elements.
    pub fn nnz(&self, _dim: usize) -> usize {
        match self {
            HybridTritVec::Sparse(s) => s.pos.len() + s.neg.len(),
            HybridTritVec::Bitsliced(b) => b.nnz(),
        }
    }

    /// Force conversion to bitsliced representation.
    ///
    /// Use this before batch operations where bitsliced is always faster.
    pub fn to_bitsliced(&self, dim: usize) -> BitslicedTritVec {
        match self {
            HybridTritVec::Sparse(s) => BitslicedTritVec::from_sparse(s, dim),
            HybridTritVec::Bitsliced(b) => b.clone(),
        }
    }

    /// Force conversion to sparse representation.
    ///
    /// Use this for serialization or when memory is constrained.
    pub fn to_sparse(&self) -> SparseVec {
        match self {
            HybridTritVec::Sparse(s) => s.clone(),
            HybridTritVec::Bitsliced(b) => b.to_sparse(),
        }
    }

    /// Get reference to inner bitsliced (if applicable).
    pub fn as_bitsliced_ref(&self) -> Option<&BitslicedTritVec> {
        match self {
            HybridTritVec::Bitsliced(b) => Some(b),
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
    /// - Otherwise: Convert to bitsliced and use SIMD-optimized path
    pub fn bind(&self, other: &Self, dim: usize) -> Self {
        match (self, other) {
            (HybridTritVec::Sparse(a), HybridTritVec::Sparse(b)) => {
                let result = a.bind(b);
                HybridTritVec::from_sparse(result, dim)
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
            _ => {
                let a_bs = self.to_bitsliced(dim);
                let b_bs = other.to_bitsliced(dim);
                HybridTritVec::Bitsliced(a_bs.bundle(&b_bs))
            }
        }
    }

    /// Dot product: $\langle a, b \rangle = \sum_i a_i \cdot b_i$
    pub fn dot(&self, other: &Self, dim: usize) -> i32 {
        // Always use bitsliced for dot - sparse doesn't have direct dot
        let a_bs = self.to_bitsliced(dim);
        let b_bs = other.to_bitsliced(dim);
        a_bs.dot(&b_bs)
    }

    /// Cosine similarity: $\cos(a, b) = \frac{\langle a, b \rangle}{\sqrt{|a|_0 \cdot |b|_0}}$
    pub fn cosine(&self, other: &Self, dim: usize) -> f64 {
        match (self, other) {
            (HybridTritVec::Sparse(a), HybridTritVec::Sparse(b)) => a.cosine(b),
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
            HybridTritVec::Bitsliced(b) => {
                HybridTritVec::Bitsliced(b.permute_optimized(shift))
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
        let hb = HybridTritVec::Bitsliced(
            BitslicedTritVec::from_sparse(&sparse_vec, DIM)
        );
        assert!(hb.is_bitsliced());

        // Cross-representation bind should work
        let result = ha.bind(&hb, DIM);
        
        // Bind with self should give all +1s at non-zero positions (self-inverse property)
        let result_sparse = result.to_sparse();
        // All positions should be positive (P * P = P, N * N = P)
        assert!(result_sparse.neg.is_empty());
        assert_eq!(result_sparse.pos.len(), sparse_vec.pos.len() + sparse_vec.neg.len());
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
        assert!(hybrid.is_sparse(), "Small dim should stay sparse regardless of density");
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
}
