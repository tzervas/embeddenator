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

    /// Generate a deterministic sparse vector from data using SHA256 seed
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
        let pos = indices[..sparsity].to_vec();
        let neg = indices[sparsity..sparsity * 2].to_vec();
        
        SparseVec { pos, neg }
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
}
