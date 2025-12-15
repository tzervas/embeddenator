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

impl SparseVec {
    /// Create an empty sparse vector
    pub fn new() -> Self {
        SparseVec {
            pos: Vec::new(),
            neg: Vec::new(),
        }
    }

    /// Generate a random sparse vector with ~1% density
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
    /// Performs element-wise multiplication
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
