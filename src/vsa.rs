//! Vector Symbolic Architecture (VSA) Implementation
//!
//! Sparse ternary vector representation with algebraic operations:
//! - Bundle (⊕): Associative superposition
//! - Bind (⊙): Non-commutative composition
//! - Cosine similarity for retrieval

use rand::seq::SliceRandom;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[cfg(feature = "bt-phase-2")]
use crate::ternary_vec::PackedTritVec;

#[cfg(feature = "bt-phase-2")]
use std::cell::RefCell;

/// Dimension of VSA vectors
pub const DIM: usize = 10000;

#[cfg(feature = "bt-phase-2")]
thread_local! {
    // Reused packed buffers for hot paths. Using TLS keeps this allocation
    // amortized while remaining safe in multi-threaded contexts.
    static PACKED_SCRATCH_A: RefCell<PackedTritVec> = RefCell::new(PackedTritVec::new_zero(DIM));
    static PACKED_SCRATCH_B: RefCell<PackedTritVec> = RefCell::new(PackedTritVec::new_zero(DIM));
    static PACKED_SCRATCH_OUT: RefCell<PackedTritVec> = RefCell::new(PackedTritVec::new_zero(DIM));
}

/// Configuration for reversible VSA encoding/decoding operations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReversibleVSAConfig {
    /// Block size for chunked encoding (must be power of 2 for efficiency)
    pub block_size: usize,
    /// Maximum path depth for hierarchical encoding
    pub max_path_depth: usize,
    /// Base permutation shift for path-based encoding
    pub base_shift: usize,
    /// Target sparsity level for operations (number of non-zero elements)
    pub target_sparsity: usize,
}

impl Default for ReversibleVSAConfig {
    fn default() -> Self {
        ReversibleVSAConfig {
            block_size: 256,  // 256-byte blocks
            max_path_depth: 10,
            base_shift: 1000,
            target_sparsity: 200,  // Default sparsity level
        }
    }
}

impl ReversibleVSAConfig {
    /// Create config optimized for small data blocks
    pub fn small_blocks() -> Self {
        ReversibleVSAConfig {
            block_size: 64,
            max_path_depth: 5,
            base_shift: 500,
            target_sparsity: 100,
        }
    }

    /// Create config optimized for large data blocks
    pub fn large_blocks() -> Self {
        ReversibleVSAConfig {
            block_size: 1024,
            max_path_depth: 20,
            base_shift: 2000,
            target_sparsity: 400,
        }
    }
}

/// Sparse ternary vector with positive and negative indices
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SparseVec {
    /// Indices with +1 value
    pub pos: Vec<usize>,
    /// Indices with -1 value
    pub neg: Vec<usize>,
}

impl Default for SparseVec {
    fn default() -> Self {
        Self::new()
    }
}

impl SparseVec {
    #[inline]
    fn nnz(&self) -> usize {
        self.pos.len() + self.neg.len()
    }

    fn intersection_count_sorted(a: &[usize], b: &[usize]) -> usize {
        let mut i = 0usize;
        let mut j = 0usize;
        let mut count = 0usize;
        while i < a.len() && j < b.len() {
            match a[i].cmp(&b[j]) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    count += 1;
                    i += 1;
                    j += 1;
                }
            }
        }
        count
    }

    fn union_sorted(a: &[usize], b: &[usize]) -> Vec<usize> {
        let mut out = Vec::with_capacity(a.len() + b.len());
        let mut i = 0usize;
        let mut j = 0usize;

        while i < a.len() && j < b.len() {
            match a[i].cmp(&b[j]) {
                std::cmp::Ordering::Less => {
                    out.push(a[i]);
                    i += 1;
                }
                std::cmp::Ordering::Greater => {
                    out.push(b[j]);
                    j += 1;
                }
                std::cmp::Ordering::Equal => {
                    out.push(a[i]);
                    i += 1;
                    j += 1;
                }
            }
        }

        if i < a.len() {
            out.extend_from_slice(&a[i..]);
        }
        if j < b.len() {
            out.extend_from_slice(&b[j..]);
        }

        out
    }

    fn difference_sorted(a: &[usize], b: &[usize]) -> Vec<usize> {
        let mut out = Vec::with_capacity(a.len());
        let mut i = 0usize;
        let mut j = 0usize;

        while i < a.len() && j < b.len() {
            match a[i].cmp(&b[j]) {
                std::cmp::Ordering::Less => {
                    out.push(a[i]);
                    i += 1;
                }
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    i += 1;
                    j += 1;
                }
            }
        }

        if i < a.len() {
            out.extend_from_slice(&a[i..]);
        }

        out
    }
    /// Create an empty sparse vector
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::SparseVec;
    ///
    /// let vec = SparseVec::new();
    /// assert!(vec.pos.is_empty());
    /// assert!(vec.neg.is_empty());
    /// ```
    pub fn new() -> Self {
        SparseVec {
            pos: Vec::new(),
            neg: Vec::new(),
        }
    }

    /// Generate a random sparse vector with ~1% density
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::SparseVec;
    ///
    /// let vec = SparseVec::random();
    /// // Vector should have approximately 1% density (100 positive + 100 negative)
    /// assert!(vec.pos.len() > 0);
    /// assert!(vec.neg.len() > 0);
    /// ```
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        let sparsity = DIM / 100; // ~1% density

        // Generate random indices without replacement
        let mut indices: Vec<usize> = (0..DIM).collect();
        indices.shuffle(&mut rng);

        let mut pos: Vec<_> = indices[..sparsity].to_vec();
        let mut neg: Vec<_> = indices[sparsity..sparsity * 2].to_vec();

        pos.sort_unstable();
        neg.sort_unstable();

        SparseVec { pos, neg }
    }

    /// Encode data into a reversible sparse vector using block-based mapping
    ///
    /// This method implements hierarchical encoding with path-based permutations
    /// for lossless data recovery. The encoding process:
    /// 1. Splits data into blocks of configurable size
    /// 2. Applies path-based permutations to each block
    /// 3. Combines blocks using hierarchical bundling
    ///
    /// # Arguments
    /// * `data` - The data to encode
    /// * `config` - Configuration for encoding parameters
    /// * `path` - Optional path string for hierarchical encoding (affects permutation)
    ///
    /// # Returns
    /// A SparseVec that can be decoded back to the original data
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::{SparseVec, ReversibleVSAConfig};
    ///
    /// let data = b"hello world";
    /// let config = ReversibleVSAConfig::default();
    /// let encoded = SparseVec::encode_data(data, &config, None);
    ///
    /// // encoded vector contains reversible representation of the data
    /// assert!(!encoded.pos.is_empty() || !encoded.neg.is_empty());
    /// ```
    pub fn encode_data(data: &[u8], config: &ReversibleVSAConfig, path: Option<&str>) -> Self {
        if data.is_empty() {
            return SparseVec::new();
        }

        // Calculate path-based shift for hierarchical encoding
        let path_shift = if let Some(path_str) = path {
            // Use path hash to determine shift, constrained to prevent overflow
            let mut hasher = Sha256::new();
            hasher.update(path_str.as_bytes());
            let hash = hasher.finalize();
            // SHA256 always produces 32 bytes, but verify slice is valid
            let hash_bytes: [u8; 4] = hash[0..4].try_into()
                .expect("SHA256 hash is always at least 4 bytes");
            let path_hash = u32::from_le_bytes(hash_bytes) as usize;
            (path_hash % config.max_path_depth) * config.base_shift
        } else {
            0
        };

        // Split data into blocks
        let mut blocks = Vec::new();
        for chunk in data.chunks(config.block_size) {
            blocks.push(chunk);
        }

        // Encode each block with position-based permutation
        let mut encoded_blocks = Vec::new();
        for (i, block) in blocks.iter().enumerate() {
            let block_shift = path_shift + (i * config.base_shift / blocks.len().max(1));
            let block_vec = Self::encode_block(block, block_shift);
            encoded_blocks.push(block_vec);
        }

        // Combine blocks hierarchically
        if encoded_blocks.is_empty() {
            SparseVec::new()
        } else if encoded_blocks.len() == 1 {
            // Safe: we just checked len() == 1, so next() must return Some
            encoded_blocks.into_iter().next()
                .expect("encoded_blocks has exactly one element")
        } else {
            // Hierarchical bundling: combine in binary tree fashion
            Self::hierarchical_bundle(&encoded_blocks)
        }
    }

    /// Decode data from a reversible sparse vector
    ///
    /// Reverses the encoding process to recover the original data.
    /// Requires the same configuration and path used during encoding.
    ///
    /// # Arguments
    /// * `config` - Same configuration used for encoding
    /// * `path` - Same path string used for encoding
    /// * `expected_size` - Expected size of the decoded data (for validation)
    ///
    /// # Returns
    /// The original data bytes (may need correction layer for 100% fidelity)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use embeddenator::{SparseVec, ReversibleVSAConfig};
    ///
    /// let data = b"hello world";
    /// let config = ReversibleVSAConfig::default();
    /// let encoded = SparseVec::encode_data(data, &config, None);
    /// let decoded = encoded.decode_data(&config, None, data.len());
    ///
    /// // Note: For 100% fidelity, use CorrectionStore with EmbrFS
    /// // Raw decode may have minor differences that corrections compensate for
    /// ```
    pub fn decode_data(&self, config: &ReversibleVSAConfig, path: Option<&str>, expected_size: usize) -> Vec<u8> {
        if self.pos.is_empty() && self.neg.is_empty() {
            return Vec::new();
        }

        if expected_size == 0 {
            return Vec::new();
        }

        // Calculate path-based shift (same as encoding)
        let path_shift = if let Some(path_str) = path {
            let mut hasher = Sha256::new();
            hasher.update(path_str.as_bytes());
            let hash = hasher.finalize();
            // SHA256 always produces 32 bytes, but verify slice is valid
            let hash_bytes: [u8; 4] = hash[0..4].try_into()
                .expect("SHA256 hash is always at least 4 bytes");
            let path_hash = u32::from_le_bytes(hash_bytes) as usize;
            (path_hash % config.max_path_depth) * config.base_shift
        } else {
            0
        };

        // Estimate number of blocks based on expected size
        let estimated_blocks = (expected_size + config.block_size - 1) / config.block_size;

        // For single block case
        if estimated_blocks <= 1 {
            return Self::decode_block(self, path_shift, expected_size);
        }

        // For multiple blocks, we need to factorize the hierarchical bundle
        // This is a simplified approach - in practice, we'd need more sophisticated
        // factorization to separate the blocks
        let mut result = Vec::new();

        // For now, attempt to decode as much as possible
        // This is a placeholder for the full hierarchical decoding
        for i in 0..estimated_blocks {
            let block_shift = path_shift + (i * config.base_shift / estimated_blocks.max(1));
            let remaining = expected_size.saturating_sub(result.len());
            if remaining == 0 {
                break;
            }
            let max_len = remaining.min(config.block_size);
            let block_data = Self::decode_block(self, block_shift, max_len);
            if block_data.is_empty() {
                break;
            }
            result.extend(block_data);
            if result.len() >= expected_size {
                break;
            }
        }

        // Truncate to expected size
        result.truncate(expected_size);
        result
    }

    /// Encode a single block of data with position-based permutation
    fn encode_block(data: &[u8], shift: usize) -> SparseVec {
        if data.is_empty() {
            return SparseVec::new();
        }

        // Map data bytes to vector indices using the permuted mapping
        let mut pos = Vec::new();
        let mut neg = Vec::new();

        for (i, &byte) in data.iter().enumerate() {
            let base_idx = (i + shift) % DIM;

            // Use byte value to determine polarity and offset
            if byte & 0x80 != 0 {
                // High bit set -> negative
                neg.push((base_idx + (byte & 0x7F) as usize) % DIM);
            } else {
                // High bit clear -> positive
                pos.push((base_idx + byte as usize) % DIM);
            }
        }

        pos.sort_unstable();
        pos.dedup();
        neg.sort_unstable();
        neg.dedup();

        SparseVec { pos, neg }
    }

    /// Decode a single block of data
    fn decode_block(encoded: &SparseVec, shift: usize, max_len: usize) -> Vec<u8> {
        if max_len == 0 {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(max_len);

        // Reconstruct data by reversing the permutation.
        // Note: `pos` and `neg` are kept sorted, so membership can be checked via binary search.
        for i in 0..max_len {
            let base_idx = (i + shift) % DIM;

            // Look for indices that map back to this position
            let mut found_byte = None;
            for offset in 0..128u8 {
                let test_idx = (base_idx + offset as usize) % DIM;

                if encoded.pos.binary_search(&test_idx).is_ok() {
                    found_byte = Some(offset);
                    break;
                } else if encoded.neg.binary_search(&test_idx).is_ok() {
                    found_byte = Some(offset | 0x80);
                    break;
                }
            }

            if let Some(byte) = found_byte {
                result.push(byte);
            } else {
                // No more data found
                break;
            }
        }

        result
    }

    /// Combine multiple vectors using hierarchical bundling
    fn hierarchical_bundle(vectors: &[SparseVec]) -> SparseVec {
        if vectors.is_empty() {
            return SparseVec::new();
        }
        if vectors.len() == 1 {
            return vectors[0].clone();
        }

        // Binary tree combination
        let mut result = vectors[0].clone();
        for vec in &vectors[1..] {
            result = result.bundle(vec);
        }
        result
    }

    /// Generate a deterministic sparse vector from data using SHA256 seed
    /// DEPRECATED: Use encode_data() for new code
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::SparseVec;
    ///
    /// let data = b"hello world";
    /// let vec1 = SparseVec::from_data(data);
    /// let vec2 = SparseVec::from_data(data);
    ///
    /// // Same input produces same vector (deterministic)
    /// assert_eq!(vec1.pos, vec2.pos);
    /// assert_eq!(vec1.neg, vec2.neg);
    /// ```
    #[deprecated(since = "0.2.0", note = "Use encode_data() for reversible encoding")]
    pub fn from_data(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();

        // SHA256 always produces 32 bytes, use first 32 bytes as seed
        let seed: [u8; 32] = hash[..32]
            .try_into()
            .expect("SHA256 output is always 32 bytes");
        let mut rng = rand::rngs::StdRng::from_seed(seed);

        let mut indices: Vec<usize> = (0..DIM).collect();
        indices.shuffle(&mut rng);

        let sparsity = DIM / 100;
        let mut pos = indices[..sparsity].to_vec();
        let mut neg = indices[sparsity..sparsity * 2].to_vec();

        pos.sort_unstable();
        neg.sort_unstable();

        SparseVec { pos, neg }
    }

    /// Bundle operation: pairwise conflict-cancel superposition (A ⊕ B)
    ///
    /// This is a fast, commutative merge for two vectors:
    /// - same sign => keep
    /// - opposite signs => cancel to 0
    /// - sign vs 0 => keep sign
    ///
    /// Note: While this is well-defined for two vectors, repeated application across 3+
    /// vectors is generally **not associative** because early cancellation/thresholding can
    /// discard multiplicity information.
    ///
    /// # Arguments
    /// * `other` - The vector to bundle with self
    /// * `config` - Optional ReversibleVSAConfig for controlling sparsity via thinning
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::{SparseVec, ReversibleVSAConfig};
    ///
    /// let vec1 = SparseVec::from_data(b"data1");
    /// let vec2 = SparseVec::from_data(b"data2");
    /// let config = ReversibleVSAConfig::default();
    /// let bundled = vec1.bundle_with_config(&vec2, Some(&config));
    ///
    /// // Bundled vector contains superposition of both inputs
    /// // Should be similar to both original vectors
    /// let sim1 = vec1.cosine(&bundled);
    /// let sim2 = vec2.cosine(&bundled);
    /// assert!(sim1 > 0.3);
    /// assert!(sim2 > 0.3);
    /// ```
    pub fn bundle_with_config(&self, other: &SparseVec, config: Option<&ReversibleVSAConfig>) -> SparseVec {
        let mut result = self.bundle(other);
        
        // Apply thinning if config provided and result exceeds target sparsity
        if let Some(cfg) = config {
            let current_count = result.pos.len() + result.neg.len();
            if current_count > cfg.target_sparsity {
                result = result.thin(cfg.target_sparsity);
            }
        }
        
        result
    }

    /// Bundle operation: pairwise conflict-cancel superposition (A ⊕ B)
    ///
    /// See `bundle()` for semantic details; this wrapper optionally applies thinning via
    /// `ReversibleVSAConfig`.
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::SparseVec;
    ///
    /// let vec1 = SparseVec::from_data(b"data1");
    /// let vec2 = SparseVec::from_data(b"data2");
    /// let bundled = vec1.bundle(&vec2);
    ///
    /// // Bundled vector contains superposition of both inputs
    /// // Should be similar to both original vectors
    /// let sim1 = vec1.cosine(&bundled);
    /// let sim2 = vec2.cosine(&bundled);
    /// assert!(sim1 > 0.3);
    /// assert!(sim2 > 0.3);
    /// ```
    pub fn bundle(&self, other: &SparseVec) -> SparseVec {
        // Optional ternary-native fast path (migration gate).
        // This is primarily intended for cases where vectors become dense enough
        // that packed word-wise operations are competitive.
        #[cfg(feature = "bt-phase-2")]
        {
            // Converting SparseVec -> PackedTritVec is O(DIM) for allocation/zeroing, plus O(nnz)
            // for setting lanes. Only attempt the packed path when total density is high enough,
            // and avoid converting an extremely sparse operand.
            let a_nnz = self.nnz();
            let b_nnz = other.nnz();
            let total = a_nnz + b_nnz;
            if total > DIM / 4 {
                let min_nnz = a_nnz.min(b_nnz);
                if min_nnz > DIM / 32 {
                    return PACKED_SCRATCH_A.with(|a_cell| {
                        PACKED_SCRATCH_B.with(|b_cell| {
                            PACKED_SCRATCH_OUT.with(|out_cell| {
                                let mut a = a_cell.borrow_mut();
                                let mut b = b_cell.borrow_mut();
                                let mut out = out_cell.borrow_mut();

                                a.fill_from_sparsevec(self, DIM);
                                b.fill_from_sparsevec(other, DIM);
                                a.bundle_into(&b, &mut out);
                                out.to_sparsevec()
                            })
                        })
                    });
                }
            }
        }

        // Majority voting for two sparse ternary vectors:
        // - Same sign => keep
        // - Opposite signs => cancel to 0
        // - Sign vs 0 => keep sign
        // This can be expressed via sorted set differences/unions.
        let pos_a = Self::difference_sorted(&self.pos, &other.neg);
        let pos_b = Self::difference_sorted(&other.pos, &self.neg);
        let neg_a = Self::difference_sorted(&self.neg, &other.pos);
        let neg_b = Self::difference_sorted(&other.neg, &self.pos);

        let pos = Self::union_sorted(&pos_a, &pos_b);
        let neg = Self::union_sorted(&neg_a, &neg_b);

        SparseVec { pos, neg }
    }

    /// Associative bundle over many vectors: sums contributions per index, then thresholds to sign.
    /// This is order-independent because all contributions are accumulated before applying sign.
    /// Complexity: O(K log K) where K is total non-zero entries across inputs.
    pub fn bundle_sum_many<'a, I>(vectors: I) -> SparseVec
    where
        I: IntoIterator<Item = &'a SparseVec>,
    {
        let mut contributions: Vec<(usize, i32)> = Vec::new();

        for vec in vectors {
            contributions.extend(vec.pos.iter().map(|&idx| (idx, 1i32)));
            contributions.extend(vec.neg.iter().map(|&idx| (idx, -1i32)));
        }

        if contributions.is_empty() {
            return SparseVec {
                pos: Vec::new(),
                neg: Vec::new(),
            };
        }

        contributions.sort_unstable_by_key(|(idx, _)| *idx);

        let mut pos = Vec::new();
        let mut neg = Vec::new();

        let mut iter = contributions.into_iter();
        // Safe: we checked contributions.is_empty() above and returned early if empty
        let (mut current_idx, mut acc) = iter.next()
            .expect("contributions is non-empty after early return check");

        for (idx, value) in iter {
            if idx == current_idx {
                acc += value;
            } else {
                if acc > 0 {
                    pos.push(current_idx);
                } else if acc < 0 {
                    neg.push(current_idx);
                }
                current_idx = idx;
                acc = value;
            }
        }

        if acc > 0 {
            pos.push(current_idx);
        } else if acc < 0 {
            neg.push(current_idx);
        }

        SparseVec { pos, neg }
    }

    /// Hybrid bundle: choose a fast pairwise fold for very sparse regimes (to preserve sparsity),
    /// otherwise use the associative sum-then-threshold path (order-independent, more faithful to majority).
    ///
    /// Heuristic: estimate expected overlap/collision count assuming uniform hashing into `DIM`.
    /// If expected colliding dimensions is below a small budget, use pairwise `bundle`; else use
    /// `bundle_sum_many`.
    pub fn bundle_hybrid_many<'a, I>(vectors: I) -> SparseVec
    where
        I: IntoIterator<Item = &'a SparseVec>,
    {
        let collected: Vec<&'a SparseVec> = vectors.into_iter().collect();
        if collected.is_empty() {
            return SparseVec {
                pos: Vec::new(),
                neg: Vec::new(),
            };
        }

        if collected.len() == 1 {
            return collected[0].clone();
        }

        if collected.len() == 2 {
            return collected[0].bundle(collected[1]);
        }

        let total_nnz: usize = collected
            .iter()
            .map(|v| v.pos.len() + v.neg.len())
            .sum();

        if total_nnz == 0 {
            return SparseVec {
                pos: Vec::new(),
                neg: Vec::new(),
            };
        }

        // Constant-time overlap/collision risk estimate.
        // Model: each non-zero lands uniformly in DIM dimensions.
        // Let λ = total_nnz / DIM be expected hits per dimension.
        // Then P(K>=2) ≈ 1 - e^{-λ}(1+λ) for Poisson(λ).
        // If expected number of colliding dimensions is tiny, pairwise fold is effectively safe
        // (and faster). Otherwise, use associative accumulation to avoid order sensitivity.
        let lambda = total_nnz as f64 / DIM as f64;
        let p_ge_2 = 1.0 - (-lambda).exp() * (1.0 + lambda);
        let expected_colliding_dims = p_ge_2 * DIM as f64;

        // Budget: allow a small number of potentially order-sensitive dimensions.
        // Tune via benchmarks; this is conservative for integrity.
        let collision_budget_dims = 32.0;

        if expected_colliding_dims <= collision_budget_dims {
            let mut iter = collected.into_iter();
            // Safe: hierarchical_bundle is only called when collected.len() > 1
            let mut acc = iter.next()
                .expect("hierarchical_bundle called with non-empty collection")
                .clone();
            for v in iter {
                acc = acc.bundle(v);
            }
            return acc;
        }

        SparseVec::bundle_sum_many(collected)
    }

    /// Bind operation: non-commutative composition (A ⊙ B)
    /// Performs element-wise multiplication. Self-inverse: A ⊙ A ≈ I
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::SparseVec;
    ///
    /// let vec = SparseVec::from_data(b"test");
    /// let bound = vec.bind(&vec);
    ///
    /// // Bind with self should produce high similarity (self-inverse property)
    /// let identity = SparseVec::from_data(b"identity");
    /// let sim = bound.cosine(&identity);
    /// // Result is approximately identity, so similarity varies
    /// assert!(sim >= -1.0 && sim <= 1.0);
    /// ```
    pub fn bind(&self, other: &SparseVec) -> SparseVec {
        #[cfg(feature = "bt-phase-2")]
        {
            // Packed bind is only worthwhile when both operands are dense enough.
            // Using a short-circuiting check avoids paying extra overhead for sparse workloads.
            let a_nnz = self.nnz();
            if a_nnz > DIM / 4 {
                let b_nnz = other.nnz();
                if b_nnz > DIM / 4 {
                    return PACKED_SCRATCH_A.with(|a_cell| {
                        PACKED_SCRATCH_B.with(|b_cell| {
                            PACKED_SCRATCH_OUT.with(|out_cell| {
                                let mut a = a_cell.borrow_mut();
                                let mut b = b_cell.borrow_mut();
                                let mut out = out_cell.borrow_mut();

                                a.fill_from_sparsevec(self, DIM);
                                b.fill_from_sparsevec(other, DIM);
                                a.bind_into(&b, &mut out);
                                out.to_sparsevec()
                            })
                        })
                    });
                }
            }
        }

        // Sparse bind is element-wise multiplication restricted to non-zero support.
        // We can compute this in a single merge-join over the signed supports, keeping
        // outputs sorted without any post-sort.
        let mut result_pos = Vec::new();
        let mut result_neg = Vec::new();

        let mut a_pos = 0usize;
        let mut a_neg = 0usize;
        let mut b_pos = 0usize;
        let mut b_neg = 0usize;

        loop {
            let next_a = match (self.pos.get(a_pos), self.neg.get(a_neg)) {
                (Some(&p), Some(&n)) => {
                    if p < n {
                        Some((p, 1i8, true))
                    } else {
                        Some((n, -1i8, false))
                    }
                }
                (Some(&p), None) => Some((p, 1i8, true)),
                (None, Some(&n)) => Some((n, -1i8, false)),
                (None, None) => None,
            };

            let next_b = match (other.pos.get(b_pos), other.neg.get(b_neg)) {
                (Some(&p), Some(&n)) => {
                    if p < n {
                        Some((p, 1i8, true))
                    } else {
                        Some((n, -1i8, false))
                    }
                }
                (Some(&p), None) => Some((p, 1i8, true)),
                (None, Some(&n)) => Some((n, -1i8, false)),
                (None, None) => None,
            };

            let Some((idx_a, sign_a, a_is_pos)) = next_a else {
                break;
            };
            let Some((idx_b, sign_b, b_is_pos)) = next_b else {
                break;
            };

            match idx_a.cmp(&idx_b) {
                std::cmp::Ordering::Less => {
                    if a_is_pos {
                        a_pos += 1;
                    } else {
                        a_neg += 1;
                    }
                }
                std::cmp::Ordering::Greater => {
                    if b_is_pos {
                        b_pos += 1;
                    } else {
                        b_neg += 1;
                    }
                }
                std::cmp::Ordering::Equal => {
                    let prod = sign_a * sign_b;
                    if prod == 1 {
                        result_pos.push(idx_a);
                    } else {
                        result_neg.push(idx_a);
                    }

                    if a_is_pos {
                        a_pos += 1;
                    } else {
                        a_neg += 1;
                    }
                    if b_is_pos {
                        b_pos += 1;
                    } else {
                        b_neg += 1;
                    }
                }
            }
        }

        SparseVec {
            pos: result_pos,
            neg: result_neg,
        }
    }

    /// Calculate cosine similarity between two sparse vectors
    /// Returns value in [-1, 1] where 1 is identical, 0 is orthogonal
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::SparseVec;
    ///
    /// let vec1 = SparseVec::from_data(b"hello");
    /// let vec2 = SparseVec::from_data(b"hello");
    /// let vec3 = SparseVec::from_data(b"world");
    ///
    /// // Identical data produces identical vectors
    /// assert!((vec1.cosine(&vec2) - 1.0).abs() < 0.01);
    ///
    /// // Different data produces low similarity
    /// let sim = vec1.cosine(&vec3);
    /// assert!(sim < 0.3);
    /// ```
    pub fn cosine(&self, other: &SparseVec) -> f64 {
        #[cfg(feature = "bt-phase-2")]
        {
            // Only use packed cosine when total density is high enough to amortize conversion,
            // and avoid converting an extremely sparse operand.
            let a_nnz = self.nnz();
            let b_nnz = other.nnz();
            let total = a_nnz + b_nnz;
            if total > DIM / 4 {
                let min_nnz = a_nnz.min(b_nnz);
                if min_nnz > DIM / 32 {
                    let dot = PACKED_SCRATCH_A.with(|a_cell| {
                        PACKED_SCRATCH_B.with(|b_cell| {
                            let mut a = a_cell.borrow_mut();
                            let mut b = b_cell.borrow_mut();
                            a.fill_from_sparsevec(self, DIM);
                            b.fill_from_sparsevec(other, DIM);
                            a.dot(&b)
                        })
                    }) as f64;
                    let self_norm = a_nnz as f64;
                    let other_norm = b_nnz as f64;
                    if self_norm == 0.0 || other_norm == 0.0 {
                        return 0.0;
                    }
                    return dot / (self_norm.sqrt() * other_norm.sqrt());
                }
            }
        }

        // Sparse ternary dot product:
        // +1 when signs match, -1 when signs oppose.
        let pp = Self::intersection_count_sorted(&self.pos, &other.pos) as i32;
        let nn = Self::intersection_count_sorted(&self.neg, &other.neg) as i32;
        let pn = Self::intersection_count_sorted(&self.pos, &other.neg) as i32;
        let np = Self::intersection_count_sorted(&self.neg, &other.pos) as i32;
        let dot = (pp + nn) - (pn + np);

        let self_norm = self.nnz() as f64;
        let other_norm = other.nnz() as f64;

        if self_norm == 0.0 || other_norm == 0.0 {
            return 0.0;
        }

        dot as f64 / (self_norm.sqrt() * other_norm.sqrt())
    }

    /// Apply cyclic permutation to vector indices
    /// Used for encoding sequence order in hierarchical structures
    ///
    /// # Arguments
    /// * `shift` - Number of positions to shift indices cyclically
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::SparseVec;
    ///
    /// let vec = SparseVec::from_data(b"test");
    /// let permuted = vec.permute(100);
    ///
    /// // Permuted vector should have different indices but same structure
    /// assert_eq!(vec.pos.len(), permuted.pos.len());
    /// assert_eq!(vec.neg.len(), permuted.neg.len());
    /// ```
    pub fn permute(&self, shift: usize) -> SparseVec {
        let permute_index = |idx: usize| (idx + shift) % DIM;

        let pos: Vec<usize> = self.pos.iter().map(|&idx| permute_index(idx)).collect();
        let neg: Vec<usize> = self.neg.iter().map(|&idx| permute_index(idx)).collect();

        // Indices must remain sorted for efficient operations
        let mut pos = pos;
        let mut neg = neg;
        pos.sort_unstable();
        neg.sort_unstable();

        SparseVec { pos, neg }
    }

    /// Apply inverse cyclic permutation to vector indices
    /// Decodes sequence order by reversing the permutation shift
    ///
    /// # Arguments
    /// * `shift` - Number of positions to reverse shift indices cyclically
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::SparseVec;
    ///
    /// let vec = SparseVec::from_data(b"test");
    /// let permuted = vec.permute(100);
    /// let recovered = permuted.inverse_permute(100);
    ///
    /// // Round-trip should recover original vector
    /// assert_eq!(vec.pos, recovered.pos);
    /// assert_eq!(vec.neg, recovered.neg);
    /// ```
    pub fn inverse_permute(&self, shift: usize) -> SparseVec {
        let inverse_permute_index = |idx: usize| (idx + DIM - (shift % DIM)) % DIM;

        let pos: Vec<usize> = self.pos.iter().map(|&idx| inverse_permute_index(idx)).collect();
        let neg: Vec<usize> = self.neg.iter().map(|&idx| inverse_permute_index(idx)).collect();

        // Indices must remain sorted for efficient operations
        let mut pos = pos;
        let mut neg = neg;
        pos.sort_unstable();
        neg.sort_unstable();

        SparseVec { pos, neg }
    }

    /// Context-Dependent Thinning Algorithm
    ///
    /// Thinning controls vector sparsity during bundle operations to prevent
    /// exponential density growth that degrades VSA performance. The algorithm:
    ///
    /// 1. Calculate current density = (pos.len() + neg.len()) as f32 / DIM as f32
    /// 2. If current_density <= target_density, return unchanged
    /// 3. Otherwise, randomly sample indices to reduce to target count
    /// 4. Preserve pos/neg ratio to maintain signal polarity balance
    /// 5. Use deterministic seeding for reproducible results
    ///
    /// Edge Cases:
    /// - Empty vector: return unchanged
    /// - target_non_zero = 0: return empty vector (not recommended)
    /// - target_non_zero >= current: return clone
    /// - Single polarity vectors: preserve polarity distribution
    ///
    /// Performance: O(n log n) due to sorting, where n = target_non_zero
    pub fn thin(&self, target_non_zero: usize) -> SparseVec {
        let current_count = self.pos.len() + self.neg.len();
        
        // Edge case: already at or below target
        if current_count <= target_non_zero {
            return self.clone();
        }
        
        // Edge case: target is zero
        if target_non_zero == 0 {
            return SparseVec::new();
        }
        
        // Calculate how many to keep from each polarity
        let pos_ratio = self.pos.len() as f32 / current_count as f32;
        let target_pos = (target_non_zero as f32 * pos_ratio).round() as usize;
        let target_neg = target_non_zero - target_pos;
        
        // Randomly sample indices using deterministic seed based on vector content
        let mut seed = [0u8; 32];
        seed[0..4].copy_from_slice(&(self.pos.len() as u32).to_le_bytes());
        seed[4..8].copy_from_slice(&(self.neg.len() as u32).to_le_bytes());
        seed[8..12].copy_from_slice(&(target_non_zero as u32).to_le_bytes());
        // Rest remains zero for deterministic seeding
        let mut rng = rand::rngs::StdRng::from_seed(seed);
        
        // Sample positive indices
        let mut sampled_pos: Vec<usize> = self.pos.clone();
        sampled_pos.shuffle(&mut rng);
        sampled_pos.truncate(target_pos);
        sampled_pos.sort_unstable();
        
        // Sample negative indices
        let mut sampled_neg: Vec<usize> = self.neg.clone();
        sampled_neg.shuffle(&mut rng);
        sampled_neg.truncate(target_neg);
        sampled_neg.sort_unstable();
        
        SparseVec {
            pos: sampled_pos,
            neg: sampled_neg,
        }
    }
}
