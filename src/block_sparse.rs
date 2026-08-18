//! Block-Sparse Trit Vector Representation
//!
//! This module provides a memory-efficient representation for sparse ternary vectors
//! by grouping dimensions into 64-trit blocks with bitmask encoding.
//!
//! # Overview
//!
//! Instead of storing individual indices for each non-zero trit, block-sparse
//! representation groups consecutive dimensions into blocks of 64, using two
//! 64-bit masks per block:
//! - `pos`: Bitmask of positions with +1 value
//! - `neg`: Bitmask of positions with -1 value
//!
//! This provides:
//! - ~50% memory reduction for clustered non-zeros
//! - SIMD-friendly operations via bitwise logic
//! - Cache-efficient iteration over dense regions
//!
//! # Invariants
//!
//! A valid `BlockSparseTritVec` maintains these invariants:
//! 1. Blocks are sorted by `block_id` in ascending order
//! 2. No overlap: `pos & neg == 0` for each block
//! 3. No zero blocks: `pos | neg != 0` for each block
//!
//! # Example
//!
//! ```rust
//! use embeddenator_vsa::block_sparse::{Block, BlockSparseTritVec};
//! use embeddenator_vsa::SparseVec;
//!
//! // Convert from SparseVec
//! let sparse = SparseVec::random();
//! let block_sparse = BlockSparseTritVec::from_sparse_vec(&sparse, 10000);
//!
//! // Convert back - should be lossless
//! let recovered = block_sparse.to_sparse_vec();
//! assert_eq!(sparse.pos, recovered.pos);
//! assert_eq!(sparse.neg, recovered.neg);
//! ```

use serde::{Deserialize, Serialize};

use crate::vsa::SparseVec;

/// Number of trits per block (using u64 bitmasks)
pub const BLOCK_SIZE: usize = 64;

/// A single block of 64 trits represented as two bitmasks.
///
/// Each bit position `i` in `pos`/`neg` corresponds to dimension `block_id * 64 + i`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    /// Block identifier (block covers dimensions `block_id * 64` to `block_id * 64 + 63`)
    pub block_id: u32,
    /// Bitmask of positions with +1 value
    pub pos: u64,
    /// Bitmask of positions with -1 value
    pub neg: u64,
}

impl Block {
    /// Create a new block with given masks.
    ///
    /// # Panics
    /// Panics if `pos & neg != 0` (overlap) or `pos | neg == 0` (zero block).
    #[inline]
    pub fn new(block_id: u32, pos: u64, neg: u64) -> Self {
        assert!(
            pos & neg == 0,
            "Block invariant violated: pos and neg overlap"
        );
        assert!(
            pos | neg != 0,
            "Block invariant violated: zero block not allowed"
        );
        Self { block_id, pos, neg }
    }

    /// Create a new block, returning None if invariants would be violated.
    #[inline]
    pub fn try_new(block_id: u32, pos: u64, neg: u64) -> Option<Self> {
        if pos & neg != 0 || pos | neg == 0 {
            None
        } else {
            Some(Self { block_id, pos, neg })
        }
    }

    /// Number of non-zero trits in this block.
    #[inline]
    pub fn nnz(&self) -> usize {
        (self.pos.count_ones() + self.neg.count_ones()) as usize
    }

    /// Check if this block is valid (no overlap, non-zero).
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.pos & self.neg == 0 && self.pos | self.neg != 0
    }

    /// Negate this block (swap pos and neg).
    #[inline]
    pub fn negate(&self) -> Self {
        Self {
            block_id: self.block_id,
            pos: self.neg,
            neg: self.pos,
        }
    }

    /// Bind two blocks at the same block_id (element-wise multiplication).
    ///
    /// For trits: (+1)*(+1) = +1, (-1)*(-1) = +1, (+1)*(-1) = -1, 0*x = 0
    #[inline]
    pub fn bind(&self, other: &Self) -> Option<Self> {
        debug_assert_eq!(
            self.block_id, other.block_id,
            "Block IDs must match for bind"
        );

        // Result is +1 where signs match (both pos or both neg)
        // Result is -1 where signs differ
        let result_pos = (self.pos & other.pos) | (self.neg & other.neg);
        let result_neg = (self.pos & other.neg) | (self.neg & other.pos);

        Block::try_new(self.block_id, result_pos, result_neg)
    }

    /// Bundle two blocks at the same block_id (element-wise majority vote).
    ///
    /// For two vectors: same sign => keep, opposite signs => cancel to 0
    #[inline]
    pub fn bundle(&self, other: &Self) -> Option<Self> {
        debug_assert_eq!(
            self.block_id, other.block_id,
            "Block IDs must match for bundle"
        );

        // Same sign keeps the sign
        let both_pos = self.pos & other.pos;
        let both_neg = self.neg & other.neg;

        // Opposite signs cancel
        let a_pos_b_neg = self.pos & other.neg;
        let a_neg_b_pos = self.neg & other.pos;

        // Positions where only one is set (no overlap)
        let only_a_pos = self.pos & !other.pos & !other.neg;
        let only_b_pos = other.pos & !self.pos & !self.neg;
        let only_a_neg = self.neg & !other.pos & !other.neg;
        let only_b_neg = other.neg & !self.pos & !self.neg;

        // Result: keep matching signs, keep singles, cancel opposites
        let result_pos = both_pos | only_a_pos | only_b_pos;
        let result_neg = both_neg | only_a_neg | only_b_neg;

        // Ensure no overlap (opposite signs should have cancelled)
        let final_pos = result_pos & !(a_pos_b_neg | a_neg_b_pos);
        let final_neg = result_neg & !(a_pos_b_neg | a_neg_b_pos);

        Block::try_new(self.block_id, final_pos, final_neg)
    }
}

/// Block-sparse representation of a ternary vector.
///
/// Dimensions are grouped into blocks of 64, with each block storing
/// two bitmasks for positive and negative trits.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockSparseTritVec {
    /// Sorted blocks (by block_id)
    blocks: Vec<Block>,
    /// Total dimension of the vector space
    dimension: usize,
}

impl BlockSparseTritVec {
    /// Create an empty block-sparse vector with given dimension.
    pub fn new(dimension: usize) -> Self {
        Self {
            blocks: Vec::new(),
            dimension,
        }
    }

    /// Create from a list of blocks. Validates and sorts.
    ///
    /// # Errors
    /// Returns None if any block violates invariants.
    pub fn from_blocks(mut blocks: Vec<Block>, dimension: usize) -> Option<Self> {
        // Validate all blocks
        for block in &blocks {
            if !block.is_valid() {
                return None;
            }
            // Check block_id is in range
            let max_block_id = dimension.div_ceil(BLOCK_SIZE);
            if block.block_id as usize >= max_block_id {
                return None;
            }
        }

        // Sort by block_id
        blocks.sort_unstable_by_key(|b| b.block_id);

        // Check for duplicate block_ids
        for window in blocks.windows(2) {
            if window[0].block_id == window[1].block_id {
                return None;
            }
        }

        Some(Self { blocks, dimension })
    }

    /// Convert from SparseVec representation.
    pub fn from_sparse_vec(sparse: &SparseVec, dimension: usize) -> Self {
        use std::collections::BTreeMap;

        let mut block_map: BTreeMap<u32, (u64, u64)> = BTreeMap::new();

        // Add positive indices
        for &idx in &sparse.pos {
            if idx >= dimension {
                continue;
            }
            let block_id = (idx / BLOCK_SIZE) as u32;
            let bit_pos = idx % BLOCK_SIZE;
            let entry = block_map.entry(block_id).or_insert((0u64, 0u64));
            entry.0 |= 1u64 << bit_pos;
        }

        // Add negative indices
        for &idx in &sparse.neg {
            if idx >= dimension {
                continue;
            }
            let block_id = (idx / BLOCK_SIZE) as u32;
            let bit_pos = idx % BLOCK_SIZE;
            let entry = block_map.entry(block_id).or_insert((0u64, 0u64));
            entry.1 |= 1u64 << bit_pos;
        }

        // Convert to blocks, skipping zero blocks
        let blocks: Vec<Block> = block_map
            .into_iter()
            .filter_map(|(block_id, (pos, neg))| {
                // Handle any overlap by canceling
                let clean_pos = pos & !neg;
                let clean_neg = neg & !pos;
                Block::try_new(block_id, clean_pos, clean_neg)
            })
            .collect();

        Self { blocks, dimension }
    }

    /// Convert to SparseVec representation.
    pub fn to_sparse_vec(&self) -> SparseVec {
        let mut pos = Vec::new();
        let mut neg = Vec::new();

        for block in &self.blocks {
            let base = block.block_id as usize * BLOCK_SIZE;

            // Extract positive indices
            let mut pos_mask = block.pos;
            while pos_mask != 0 {
                let bit = pos_mask.trailing_zeros() as usize;
                let idx = base + bit;
                if idx < self.dimension {
                    pos.push(idx);
                }
                pos_mask &= pos_mask - 1; // Clear lowest set bit
            }

            // Extract negative indices
            let mut neg_mask = block.neg;
            while neg_mask != 0 {
                let bit = neg_mask.trailing_zeros() as usize;
                let idx = base + bit;
                if idx < self.dimension {
                    neg.push(idx);
                }
                neg_mask &= neg_mask - 1;
            }
        }

        // Indices are already sorted due to sorted blocks
        SparseVec { pos, neg }
    }

    /// Get the dimension of this vector.
    #[inline]
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Get the number of blocks.
    #[inline]
    pub fn num_blocks(&self) -> usize {
        self.blocks.len()
    }

    /// Get total number of non-zero elements.
    pub fn nnz(&self) -> usize {
        self.blocks.iter().map(|b| b.nnz()).sum()
    }

    /// Get iterator over blocks.
    pub fn blocks(&self) -> impl Iterator<Item = &Block> {
        self.blocks.iter()
    }

    /// Validate all invariants are maintained.
    pub fn validate(&self) -> Result<(), BlockSparseError> {
        // Check blocks are sorted
        for window in self.blocks.windows(2) {
            if window[0].block_id >= window[1].block_id {
                return Err(BlockSparseError::UnsortedBlocks);
            }
        }

        // Check each block is valid
        for (i, block) in self.blocks.iter().enumerate() {
            if block.pos & block.neg != 0 {
                return Err(BlockSparseError::OverlappingMasks { block_index: i });
            }
            if block.pos | block.neg == 0 {
                return Err(BlockSparseError::ZeroBlock { block_index: i });
            }
            let max_block_id = self.dimension.div_ceil(BLOCK_SIZE);
            if block.block_id as usize >= max_block_id {
                return Err(BlockSparseError::BlockIdOutOfRange {
                    block_id: block.block_id,
                    max: max_block_id as u32,
                });
            }
        }

        Ok(())
    }

    /// Negate all trits (swap positive and negative).
    pub fn negate(&self) -> Self {
        let blocks = self.blocks.iter().map(|b| b.negate()).collect();
        Self {
            blocks,
            dimension: self.dimension,
        }
    }

    /// Bind with another block-sparse vector (element-wise multiplication).
    pub fn bind(&self, other: &Self) -> Self {
        debug_assert_eq!(
            self.dimension, other.dimension,
            "Dimensions must match for bind"
        );

        let mut result_blocks = Vec::new();
        let mut i = 0;
        let mut j = 0;

        // Merge-join on block_id (only matching blocks contribute)
        while i < self.blocks.len() && j < other.blocks.len() {
            let a = &self.blocks[i];
            let b = &other.blocks[j];

            match a.block_id.cmp(&b.block_id) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    if let Some(bound) = a.bind(b) {
                        result_blocks.push(bound);
                    }
                    i += 1;
                    j += 1;
                }
            }
        }

        Self {
            blocks: result_blocks,
            dimension: self.dimension,
        }
    }

    /// Bundle with another block-sparse vector (element-wise majority vote).
    pub fn bundle(&self, other: &Self) -> Self {
        debug_assert_eq!(
            self.dimension, other.dimension,
            "Dimensions must match for bundle"
        );

        let mut result_blocks = Vec::new();
        let mut i = 0;
        let mut j = 0;

        // Merge-join on block_id
        while i < self.blocks.len() || j < other.blocks.len() {
            let a = self.blocks.get(i);
            let b = other.blocks.get(j);

            match (a, b) {
                (Some(a), Some(b)) => match a.block_id.cmp(&b.block_id) {
                    std::cmp::Ordering::Less => {
                        result_blocks.push(*a);
                        i += 1;
                    }
                    std::cmp::Ordering::Greater => {
                        result_blocks.push(*b);
                        j += 1;
                    }
                    std::cmp::Ordering::Equal => {
                        if let Some(bundled) = a.bundle(b) {
                            result_blocks.push(bundled);
                        }
                        i += 1;
                        j += 1;
                    }
                },
                (Some(a), None) => {
                    result_blocks.push(*a);
                    i += 1;
                }
                (None, Some(b)) => {
                    result_blocks.push(*b);
                    j += 1;
                }
                (None, None) => break,
            }
        }

        Self {
            blocks: result_blocks,
            dimension: self.dimension,
        }
    }

    /// Bundle multiple vectors together using sum-then-threshold.
    ///
    /// This is associative (order-independent) unlike pairwise bundling.
    pub fn bundle_many<'a, I>(vectors: I, dimension: usize) -> Self
    where
        I: IntoIterator<Item = &'a BlockSparseTritVec>,
    {
        use std::collections::BTreeMap;

        // Accumulate contributions per block
        let mut block_sums: BTreeMap<u32, Vec<i32>> = BTreeMap::new();

        for vec in vectors {
            for block in &vec.blocks {
                let sums = block_sums
                    .entry(block.block_id)
                    .or_insert_with(|| vec![0i32; BLOCK_SIZE]);

                #[allow(clippy::needless_range_loop)]
                for bit in 0..BLOCK_SIZE {
                    // bit is used both for array indexing AND bit operations
                    if block.pos & (1u64 << bit) != 0 {
                        sums[bit] += 1;
                    } else if block.neg & (1u64 << bit) != 0 {
                        sums[bit] -= 1;
                    }
                }
            }
        }

        // Threshold to ternary
        let blocks: Vec<Block> = block_sums
            .into_iter()
            .filter_map(|(block_id, sums)| {
                let mut pos = 0u64;
                let mut neg = 0u64;

                for (bit, &sum) in sums.iter().enumerate() {
                    if sum > 0 {
                        pos |= 1u64 << bit;
                    } else if sum < 0 {
                        neg |= 1u64 << bit;
                    }
                }

                Block::try_new(block_id, pos, neg)
            })
            .collect();

        Self { blocks, dimension }
    }

    /// Calculate memory usage in bytes.
    ///
    /// This accounts for the struct itself plus heap allocation for blocks.
    /// Uses capacity (not len) since that's the actual heap allocation size.
    pub fn memory_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.blocks.capacity() * std::mem::size_of::<Block>()
    }

    /// Calculate memory usage of equivalent SparseVec.
    pub fn sparse_vec_memory_bytes(&self) -> usize {
        let nnz = self.nnz();
        // SparseVec uses Vec<usize> for pos and neg
        std::mem::size_of::<SparseVec>() + nnz * std::mem::size_of::<usize>()
    }

    /// Memory efficiency ratio (< 1.0 means block-sparse uses less memory).
    pub fn memory_efficiency(&self) -> f64 {
        let block_mem = self.memory_bytes() as f64;
        let sparse_mem = self.sparse_vec_memory_bytes() as f64;
        if sparse_mem == 0.0 {
            1.0
        } else {
            block_mem / sparse_mem
        }
    }
}

/// Errors that can occur with block-sparse vectors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockSparseError {
    /// Blocks are not sorted by block_id.
    UnsortedBlocks,
    /// A block has overlapping pos and neg masks.
    OverlappingMasks { block_index: usize },
    /// A block has both pos and neg as zero.
    ZeroBlock { block_index: usize },
    /// Block ID exceeds maximum for dimension.
    BlockIdOutOfRange { block_id: u32, max: u32 },
}

impl std::fmt::Display for BlockSparseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsortedBlocks => write!(f, "Blocks are not sorted by block_id"),
            Self::OverlappingMasks { block_index } => {
                write!(f, "Block {} has overlapping pos and neg masks", block_index)
            }
            Self::ZeroBlock { block_index } => {
                write!(f, "Block {} has both pos and neg as zero", block_index)
            }
            Self::BlockIdOutOfRange { block_id, max } => {
                write!(
                    f,
                    "Block ID {} exceeds maximum {} for dimension",
                    block_id, max
                )
            }
        }
    }
}

impl std::error::Error for BlockSparseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_creation() {
        // Valid block
        let block = Block::new(0, 0b1010, 0b0101);
        assert_eq!(block.nnz(), 4);
        assert!(block.is_valid());

        // try_new with valid input
        assert!(Block::try_new(0, 0b1010, 0b0101).is_some());

        // try_new with overlap should fail
        assert!(Block::try_new(0, 0b1111, 0b1111).is_none());

        // try_new with zero block should fail
        assert!(Block::try_new(0, 0, 0).is_none());
    }

    #[test]
    fn test_block_negate() {
        let block = Block::new(0, 0b1010, 0b0101);
        let negated = block.negate();
        assert_eq!(negated.pos, 0b0101);
        assert_eq!(negated.neg, 0b1010);
    }

    #[test]
    fn test_block_bind() {
        // (+1, +1) -> +1
        // (-1, -1) -> +1
        // (+1, -1) -> -1
        let a = Block::new(0, 0b1100, 0b0011);
        let b = Block::new(0, 0b1010, 0b0101);

        let bound = a.bind(&b).unwrap();
        // pos: (1100 & 1010) | (0011 & 0101) = 1000 | 0001 = 1001
        // neg: (1100 & 0101) | (0011 & 1010) = 0100 | 0010 = 0110
        assert_eq!(bound.pos, 0b1001);
        assert_eq!(bound.neg, 0b0110);
    }

    #[test]
    fn test_block_bundle() {
        // Same sign keeps, opposite cancels
        let a = Block::new(0, 0b1100, 0b0011);
        let b = Block::new(0, 0b1010, 0b0101);

        let bundled = a.bundle(&b).unwrap();
        // Both pos at bit 3: keep pos
        // a.pos at bit 2, b.neg at bit 2: cancel
        // a.neg at bit 1, b.pos at bit 1: cancel
        // Both neg at bit 0: keep neg
        assert_eq!(bundled.pos, 0b1000);
        assert_eq!(bundled.neg, 0b0001);
    }

    #[test]
    fn test_sparse_vec_roundtrip() {
        let sparse = SparseVec::random();
        let block_sparse = BlockSparseTritVec::from_sparse_vec(&sparse, 10000);

        // Validate invariants
        assert!(block_sparse.validate().is_ok());

        // Roundtrip
        let recovered = block_sparse.to_sparse_vec();
        assert_eq!(sparse.pos, recovered.pos);
        assert_eq!(sparse.neg, recovered.neg);
    }

    #[test]
    fn test_block_sparse_bundle() {
        let a = SparseVec::random();
        let b = SparseVec::random();

        let a_block = BlockSparseTritVec::from_sparse_vec(&a, 10000);
        let b_block = BlockSparseTritVec::from_sparse_vec(&b, 10000);

        // Bundle in both representations
        let sparse_bundled = a.bundle(&b);
        let block_bundled = a_block.bundle(&b_block);

        // Convert block result to sparse and compare
        let block_as_sparse = block_bundled.to_sparse_vec();
        assert_eq!(sparse_bundled.pos, block_as_sparse.pos);
        assert_eq!(sparse_bundled.neg, block_as_sparse.neg);
    }

    #[test]
    fn test_block_sparse_bind() {
        let a = SparseVec::random();
        let b = SparseVec::random();

        let a_block = BlockSparseTritVec::from_sparse_vec(&a, 10000);
        let b_block = BlockSparseTritVec::from_sparse_vec(&b, 10000);

        // Bind in both representations
        let sparse_bound = a.bind(&b);
        let block_bound = a_block.bind(&b_block);

        // Convert block result to sparse and compare
        let block_as_sparse = block_bound.to_sparse_vec();
        assert_eq!(sparse_bound.pos, block_as_sparse.pos);
        assert_eq!(sparse_bound.neg, block_as_sparse.neg);
    }

    #[test]
    fn test_block_sparse_negate() {
        let sparse = SparseVec::random();
        let block_sparse = BlockSparseTritVec::from_sparse_vec(&sparse, 10000);

        let negated = block_sparse.negate();
        let as_sparse = negated.to_sparse_vec();

        // Negation swaps pos and neg
        assert_eq!(sparse.pos, as_sparse.neg);
        assert_eq!(sparse.neg, as_sparse.pos);
    }

    #[test]
    fn test_bundle_many_associativity() {
        let a = SparseVec::random();
        let b = SparseVec::random();
        let c = SparseVec::random();

        let a_block = BlockSparseTritVec::from_sparse_vec(&a, 10000);
        let b_block = BlockSparseTritVec::from_sparse_vec(&b, 10000);
        let c_block = BlockSparseTritVec::from_sparse_vec(&c, 10000);

        // bundle_many should be order-independent
        let result1 = BlockSparseTritVec::bundle_many([&a_block, &b_block, &c_block], 10000);
        let result2 = BlockSparseTritVec::bundle_many([&c_block, &a_block, &b_block], 10000);

        assert_eq!(result1.to_sparse_vec().pos, result2.to_sparse_vec().pos);
        assert_eq!(result1.to_sparse_vec().neg, result2.to_sparse_vec().neg);
    }

    #[test]
    fn test_memory_efficiency() {
        // Create a vector with clustered non-zeros (should be efficient)
        let mut pos = Vec::new();
        let mut neg = Vec::new();

        // Cluster 1: indices 0-63 (block 0)
        for i in 0..32 {
            pos.push(i);
        }
        for i in 32..64 {
            neg.push(i);
        }

        // Cluster 2: indices 1024-1087 (block 16) - aligned to 64-bit boundary
        for i in 1024..1056 {
            pos.push(i);
        }
        for i in 1056..1088 {
            neg.push(i);
        }

        let sparse = SparseVec { pos, neg };
        let block_sparse = BlockSparseTritVec::from_sparse_vec(&sparse, 10000);

        // Should use only 2 blocks for 128 non-zeros
        assert_eq!(block_sparse.num_blocks(), 2);
        assert_eq!(block_sparse.nnz(), 128);

        // Block-sparse should be more efficient for clustered data
        let efficiency = block_sparse.memory_efficiency();
        assert!(
            efficiency < 1.0,
            "Block-sparse should use less memory for clustered data"
        );
    }

    #[test]
    fn test_validate_catches_errors() {
        // Manually create invalid block-sparse vector
        let invalid = BlockSparseTritVec {
            blocks: vec![
                Block {
                    block_id: 1,
                    pos: 1,
                    neg: 0,
                },
                Block {
                    block_id: 0,
                    pos: 1,
                    neg: 0,
                }, // Out of order
            ],
            dimension: 10000,
        };
        assert!(matches!(
            invalid.validate(),
            Err(BlockSparseError::UnsortedBlocks)
        ));

        // Block with overlap (manually bypassing constructor)
        let invalid2 = BlockSparseTritVec {
            blocks: vec![Block {
                block_id: 0,
                pos: 0b11,
                neg: 0b11,
            }],
            dimension: 10000,
        };
        assert!(matches!(
            invalid2.validate(),
            Err(BlockSparseError::OverlappingMasks { .. })
        ));
    }
}
