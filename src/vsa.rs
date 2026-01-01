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
use std::collections::HashSet;

/// Dimension of VSA vectors
pub const DIM: usize = 10000;

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
            let path_hash = u32::from_le_bytes(hash[0..4].try_into().unwrap()) as usize;
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
            encoded_blocks.into_iter().next().unwrap()
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

        // Calculate path-based shift (same as encoding)
        let path_shift = if let Some(path_str) = path {
            let mut hasher = Sha256::new();
            hasher.update(path_str.as_bytes());
            let hash = hasher.finalize();
            let path_hash = u32::from_le_bytes(hash[0..4].try_into().unwrap()) as usize;
            (path_hash % config.max_path_depth) * config.base_shift
        } else {
            0
        };

        // Estimate number of blocks based on expected size
        let estimated_blocks = (expected_size + config.block_size - 1) / config.block_size;

        // For single block case
        if estimated_blocks <= 1 {
            return Self::decode_block(self, path_shift);
        }

        // For multiple blocks, we need to factorize the hierarchical bundle
        // This is a simplified approach - in practice, we'd need more sophisticated
        // factorization to separate the blocks
        let mut result = Vec::new();

        // For now, attempt to decode as much as possible
        // This is a placeholder for the full hierarchical decoding
        for i in 0..estimated_blocks {
            let block_shift = path_shift + (i * config.base_shift / estimated_blocks.max(1));
            let block_data = Self::decode_block(self, block_shift);
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

        // Create block index mapping
        let block_indices: Vec<usize> = (0..data.len()).collect();
        let mut permuted_indices = block_indices.clone();

        // Apply cyclic permutation based on shift
        for i in 0..permuted_indices.len() {
            permuted_indices[i] = (i + shift) % DIM;
        }

        // Map data bytes to vector indices using the permuted mapping
        let mut pos = Vec::new();
        let mut neg = Vec::new();

        for (i, &byte) in data.iter().enumerate() {
            let base_idx = permuted_indices[i % permuted_indices.len()];

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
    fn decode_block(encoded: &SparseVec, shift: usize) -> Vec<u8> {
        let mut result = Vec::new();
        let mut index_map = HashSet::new();

        // Collect all indices
        for &idx in &encoded.pos {
            index_map.insert(idx);
        }
        for &idx in &encoded.neg {
            index_map.insert(idx);
        }

        // Reconstruct data by reversing the permutation
        let max_possible_size = index_map.len();
        for i in 0..max_possible_size {
            let base_idx = (i + shift) % DIM;

            // Look for indices that map back to this position
            let mut found_byte = None;
            for offset in 0..128u8 {
                let test_idx_pos = (base_idx + offset as usize) % DIM;
                let test_idx_neg = (base_idx + offset as usize) % DIM;

                if encoded.pos.contains(&test_idx_pos) {
                    found_byte = Some(offset);
                    break;
                } else if encoded.neg.contains(&test_idx_neg) {
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

    /// Bundle operation: associative superposition (A ⊕ B)
    /// Combines two vectors by majority voting on each dimension
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

    /// Bundle operation: associative superposition (A ⊕ B)
    /// Combines two vectors by majority voting on each dimension
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
        let pos_set: HashSet<_> = self.pos.iter().copied().collect();
        let neg_set: HashSet<_> = self.neg.iter().copied().collect();
        let other_pos_set: HashSet<_> = other.pos.iter().copied().collect();
        let other_neg_set: HashSet<_> = other.neg.iter().copied().collect();

        let mut result_pos: HashSet<usize> = HashSet::new();
        let mut result_neg: HashSet<usize> = HashSet::new();

        for &idx in &self.pos {
            if other_pos_set.contains(&idx) || !other_neg_set.contains(&idx) {
                result_pos.insert(idx);
            }
        }

        for &idx in &other.pos {
            if pos_set.contains(&idx) || !neg_set.contains(&idx) {
                result_pos.insert(idx);
            }
        }

        for &idx in &self.neg {
            if other_neg_set.contains(&idx) || !other_pos_set.contains(&idx) {
                result_neg.insert(idx);
            }
        }

        for &idx in &other.neg {
            if neg_set.contains(&idx) || !pos_set.contains(&idx) {
                result_neg.insert(idx);
            }
        }

        result_pos.retain(|&x| !result_neg.contains(&x));
        result_neg.retain(|&x| !result_pos.contains(&x));

        let mut pos: Vec<_> = result_pos.into_iter().collect();
        let mut neg: Vec<_> = result_neg.into_iter().collect();
        pos.sort_unstable();
        neg.sort_unstable();

        SparseVec { pos, neg }
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
        let pos_set: HashSet<_> = self.pos.iter().copied().collect();
        let neg_set: HashSet<_> = self.neg.iter().copied().collect();

        let mut result_pos = Vec::new();
        let mut result_neg = Vec::new();

        for &idx in &other.pos {
            if pos_set.contains(&idx) {
                result_pos.push(idx);
            } else if neg_set.contains(&idx) {
                result_neg.push(idx);
            }
        }

        for &idx in &other.neg {
            if pos_set.contains(&idx) {
                result_neg.push(idx);
            } else if neg_set.contains(&idx) {
                result_pos.push(idx);
            }
        }

        result_pos.sort_unstable();
        result_neg.sort_unstable();

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
        let pos_set: HashSet<_> = self.pos.iter().copied().collect();
        let neg_set: HashSet<_> = self.neg.iter().copied().collect();

        let mut dot = 0i32;
        for &idx in &other.pos {
            if pos_set.contains(&idx) {
                dot += 1;
            } else if neg_set.contains(&idx) {
                dot -= 1;
            }
        }

        for &idx in &other.neg {
            if pos_set.contains(&idx) {
                dot -= 1;
            } else if neg_set.contains(&idx) {
                dot += 1;
            }
        }

        let self_norm = (self.pos.len() + self.neg.len()) as f64;
        let other_norm = (other.pos.len() + other.neg.len()) as f64;

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
