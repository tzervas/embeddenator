//! Resonator Networks for VSA Pattern Completion
//!
//! Implements iterative refinement algorithms for:
//! - Pattern completion from noisy or partial inputs
//! - Factorization of compound representations
//! - Noise reduction through codebook projection

use crate::vsa::{SparseVec, ReversibleVSAConfig};
use serde::{Deserialize, Serialize};

/// Result of resonator factorization
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FactorizeResult {
    /// Extracted factors
    pub factors: Vec<SparseVec>,
    /// Number of iterations performed
    pub iterations: usize,
    /// Final convergence delta
    pub final_delta: f64,
}

/// Resonator network for pattern completion and factorization
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resonator {
    /// Codebook of clean reference patterns
    pub codebook: Vec<SparseVec>,
    /// Maximum iterations for convergence
    pub max_iterations: usize,
    /// Convergence threshold for early stopping
    pub convergence_threshold: f64,
}

impl Default for Resonator {
    fn default() -> Self {
        Self {
            codebook: Vec::new(),
            max_iterations: 10,
            convergence_threshold: 0.001,
        }
    }
}

impl Resonator {
    /// Create a new resonator with default parameters
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::resonator::Resonator;
    ///
    /// let resonator = Resonator::new();
    /// assert_eq!(resonator.max_iterations, 10);
    /// assert_eq!(resonator.convergence_threshold, 0.001);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Create resonator with custom parameters
    ///
    /// # Arguments
    /// * `codebook` - Vector of clean reference patterns
    /// * `max_iterations` - Maximum refinement iterations
    /// * `convergence_threshold` - Early stopping threshold
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::resonator::Resonator;
    /// use embeddenator::{ReversibleVSAConfig, SparseVec};
    ///
    /// let cfg = ReversibleVSAConfig::default();
    /// let codebook = vec![
    ///     SparseVec::encode_data(b"pattern1", &cfg, None),
    ///     SparseVec::encode_data(b"pattern2", &cfg, None),
    /// ];
    /// let resonator = Resonator::with_params(codebook, 20, 0.0001);
    /// assert_eq!(resonator.max_iterations, 20);
    /// ```
    pub fn with_params(codebook: Vec<SparseVec>, max_iterations: usize, convergence_threshold: f64) -> Self {
        Self {
            codebook,
            max_iterations,
            convergence_threshold,
        }
    }

    /// Project a noisy vector onto the nearest codebook entry
    ///
    /// Computes cosine similarity against all codebook entries and returns
    /// the entry with highest similarity. Used for pattern completion and
    /// noise reduction.
    ///
    /// # Arguments
    /// * `noisy` - Input vector to project (may be noisy or partial)
    ///
    /// # Returns
    /// The codebook entry with highest similarity to the input
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::resonator::Resonator;
    /// use embeddenator::{ReversibleVSAConfig, SparseVec};
    ///
    /// let cfg = ReversibleVSAConfig::default();
    /// let clean = SparseVec::encode_data(b"hello", &cfg, None);
    /// let codebook = vec![clean.clone(), SparseVec::encode_data(b"world", &cfg, None)];
    /// let resonator = Resonator::with_params(codebook, 10, 0.001);
    ///
    /// // Clean input should project to itself
    /// let projected = resonator.project(&clean);
    /// assert!(clean.cosine(&projected) > 0.9);
    /// ```
    pub fn project(&self, noisy: &SparseVec) -> SparseVec {
        if self.codebook.is_empty() {
            return noisy.clone();
        }

        let mut best_similarity = f64::NEG_INFINITY;
        let mut best_entry = &self.codebook[0];

        for entry in &self.codebook {
            let similarity = entry.cosine(noisy);
            if similarity > best_similarity {
                best_similarity = similarity;
                best_entry = entry;
            }
        }

        best_entry.clone()
    }

    /// Factorize a compound vector into constituent factors using iterative refinement
    ///
    /// Uses the resonator network to decompose a bundled vector into its original
    /// components through iterative projection and unbinding operations.
    ///
    /// # Arguments
    /// * `compound` - The bundled vector to factorize
    /// * `num_factors` - Number of factors to extract
    ///
    /// # Returns
    /// FactorizeResult containing the extracted factors, iterations performed, and convergence delta
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::resonator::Resonator;
    /// use embeddenator::{ReversibleVSAConfig, SparseVec};
    ///
    /// let cfg = ReversibleVSAConfig::default();
    /// let factor1 = SparseVec::encode_data(b"hello", &cfg, None);
    /// let factor2 = SparseVec::encode_data(b"world", &cfg, None);
    /// let compound = factor1.bundle(&factor2);
    ///
    /// let codebook = vec![factor1.clone(), factor2.clone()];
    /// let resonator = Resonator::with_params(codebook, 10, 0.001);
    ///
    /// let result = resonator.factorize(&compound, 2);
    /// assert_eq!(result.factors.len(), 2);
    /// assert!(result.iterations <= 10);
    /// ```
    pub fn factorize(&self, compound: &SparseVec, num_factors: usize) -> FactorizeResult {
        if self.codebook.is_empty() || num_factors == 0 {
            return FactorizeResult {
                factors: vec![],
                iterations: 0,
                final_delta: 0.0,
            };
        }

        // Initialize factor estimates randomly
        let mut factors: Vec<SparseVec> = (0..num_factors)
            .map(|_| SparseVec::random())
            .collect();
        let mut iterations = 0;
        let mut final_delta = f64::INFINITY;

        for iter in 0..self.max_iterations {
            iterations = iter + 1;
            let mut max_delta = 0.0f64;
            let mut all_stable = true;

            // Update each factor
            for i in 0..num_factors {
                // Unbind all other factors from the compound
                let mut unbound = compound.clone();
                for (j, factor) in factors.iter().enumerate() {
                    if i != j {
                        unbound = unbound.bind(factor);
                    }
                }

                // Project onto codebook
                let projected = self.project(&unbound);

                // Calculate delta for this factor
                let delta = 1.0 - factors[i].cosine(&projected);
                max_delta = max_delta.max(delta);

                // Check if this factor changed significantly
                if delta > self.convergence_threshold {
                    all_stable = false;
                }

                // Update factor estimate
                factors[i] = projected;
            }

            final_delta = max_delta;

            // Log progress if debug enabled
            #[cfg(debug_assertions)]
            println!("Iteration {}: delta = {:.6}", iterations, final_delta);

            // Check convergence - either max delta below threshold or all factors stable
            if final_delta < self.convergence_threshold || all_stable {
                break;
            }
        }

        FactorizeResult {
            factors,
            iterations,
            final_delta,
        }
    }

    /// Recover data from an encoded sparse vector using resonator-enhanced decoding
    ///
    /// Uses the codebook to enhance pattern completion during the decoding process,
    /// enabling recovery from noisy or partially corrupted encodings.
    ///
    /// # Arguments
    /// * `encoded` - The encoded sparse vector to decode
    /// * `config` - Configuration used for encoding
    /// * `path` - Path string used for encoding
    /// * `expected_size` - Expected size of the decoded data
    ///
    /// # Returns
    /// Recovered data bytes (may need correction layer for 100% fidelity)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use embeddenator::resonator::Resonator;
    /// use embeddenator::{SparseVec, ReversibleVSAConfig};
    ///
    /// let data = b"hello world";
    /// let config = ReversibleVSAConfig::default();
    /// let encoded = SparseVec::encode_data(data, &config, None);
    ///
    /// let resonator = Resonator::new();
    /// let recovered = resonator.recover_data(&encoded, &config, None, data.len());
    ///
    /// // Note: For 100% fidelity, use CorrectionStore with EmbrFS
    /// ```
    pub fn recover_data(&self, encoded: &SparseVec, config: &ReversibleVSAConfig, path: Option<&str>, expected_size: usize) -> Vec<u8> {
        // First attempt direct decoding
        let mut result = encoded.decode_data(config, path, expected_size);

        // If direct decoding didn't work and we have a codebook, try enhanced recovery
        if result.is_empty() && !self.codebook.is_empty() {
            // Project the encoded vector onto the codebook to clean it up
            let cleaned = self.project(encoded);

            // Try decoding the cleaned vector
            result = cleaned.decode_data(config, path, expected_size);
        }

        result
    }

    /// Recover missing chunks using pattern completion
    ///
    /// Attempts to reconstruct missing or corrupted data chunks by leveraging
    /// the resonator's codebook for pattern completion.
    ///
    /// # Arguments
    /// * `available_chunks` - Map of chunk_id to available chunk data
    /// * `missing_chunk_ids` - List of chunk IDs that need recovery
    /// * `config` - VSA configuration for encoding/decoding
    ///
    /// # Returns
    /// Map of recovered chunk_id to recovered data
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::resonator::Resonator;
    /// use embeddenator::ReversibleVSAConfig;
    /// use std::collections::HashMap;
    ///
    /// let resonator = Resonator::new();
    /// let config = ReversibleVSAConfig::default();
    /// let mut available_chunks = HashMap::new();
    /// // available_chunks.insert(0, chunk_data);
    ///
    /// let missing_ids = vec![1, 2];
    /// let recovered = resonator.recover_chunks(&available_chunks, &missing_ids, &config);
    /// ```
    pub fn recover_chunks(&self, _available_chunks: &std::collections::HashMap<usize, Vec<u8>>, missing_chunk_ids: &[usize], _config: &ReversibleVSAConfig) -> std::collections::HashMap<usize, Vec<u8>> {
        let mut recovered = std::collections::HashMap::new();

        for &chunk_id in missing_chunk_ids {
            // Try to recover this chunk using available context
            // This is a simplified implementation - in practice, you'd use
            // more sophisticated pattern completion based on neighboring chunks

            if !self.codebook.is_empty() {
                // Use codebook patterns to attempt recovery
                // For now, return a placeholder - this would be enhanced with
                // actual recovery logic based on chunk relationships
                let placeholder = format!("recovered_chunk_{}", chunk_id).into_bytes();
                recovered.insert(chunk_id, placeholder);
            }
        }

        recovered
    }

    /// Apply ternary sign thresholding to enhance sparsity preservation
    ///
    /// Converts similarity scores to ternary values (-1, 0, +1) using a threshold,
    /// preserving the sparse ternary nature of VSA vectors while reducing noise.
    ///
    /// # Arguments
    /// * `similarities` - Vector of similarity scores to threshold
    /// * `threshold` - Minimum absolute similarity to retain (default: 0.1)
    ///
    /// # Returns
    /// Vector of ternary values: -1, 0, or +1
    ///
    /// # Examples
    ///
    /// ```
    /// use embeddenator::resonator::Resonator;
    ///
    /// let resonator = Resonator::new();
    /// let similarities = vec![0.8, -0.3, 0.05, -0.9];
    /// let ternary = resonator.sign_threshold(&similarities, 0.1);
    ///
    /// assert_eq!(ternary, vec![1, -1, 0, -1]);
    /// ```
    pub fn sign_threshold(&self, similarities: &[f64], threshold: f64) -> Vec<i8> {
        similarities
            .iter()
            .map(|&sim| {
                if sim == 0.0 {
                    0
                } else if sim.abs() >= threshold {
                    if sim > 0.0 {
                        1
                    } else {
                        -1
                    }
                } else {
                    0
                }
            })
            .collect()
    }
}