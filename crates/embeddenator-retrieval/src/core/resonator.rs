//! Resonator Networks for VSA Pattern Completion
//!
//! Implements iterative refinement algorithms for:
//! - Pattern completion from noisy or partial inputs
//! - Factorization of compound representations
//! - Noise reduction through codebook projection

use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};
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
    /// use embeddenator_retrieval::resonator::Resonator;
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
    /// use embeddenator_retrieval::resonator::Resonator;
    /// use embeddenator_vsa::SparseVec;
    ///
    /// let codebook = vec![SparseVec::from_data(b"pattern1"), SparseVec::from_data(b"pattern2")];
    /// let resonator = Resonator::with_params(codebook, 20, 0.0001);
    /// assert_eq!(resonator.max_iterations, 20);
    /// ```
    pub fn with_params(
        codebook: Vec<SparseVec>,
        max_iterations: usize,
        convergence_threshold: f64,
    ) -> Self {
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
    /// use embeddenator_retrieval::resonator::Resonator;
    /// use embeddenator_vsa::SparseVec;
    ///
    /// let clean = SparseVec::from_data(b"hello");
    /// let codebook = vec![clean.clone(), SparseVec::from_data(b"world")];
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
    /// use embeddenator_retrieval::resonator::Resonator;
    /// use embeddenator_vsa::SparseVec;
    ///
    /// let factor1 = SparseVec::from_data(b"hello");
    /// let factor2 = SparseVec::from_data(b"world");
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
        let mut factors: Vec<SparseVec> = (0..num_factors).map(|_| SparseVec::random()).collect();
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
    /// use embeddenator_retrieval::resonator::Resonator;
    /// use embeddenator_vsa::{SparseVec, ReversibleVSAConfig};
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
    pub fn recover_data(
        &self,
        encoded: &SparseVec,
        config: &ReversibleVSAConfig,
        path: Option<&str>,
        expected_size: usize,
    ) -> Vec<u8> {
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
    /// use embeddenator_retrieval::resonator::Resonator;
    /// use embeddenator_vsa::ReversibleVSAConfig;
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
    pub fn recover_chunks(
        &self,
        available_chunks: &std::collections::HashMap<usize, Vec<u8>>,
        missing_chunk_ids: &[usize],
        config: &ReversibleVSAConfig,
    ) -> std::collections::HashMap<usize, Vec<u8>> {
        let mut recovered = std::collections::HashMap::new();

        if missing_chunk_ids.is_empty() {
            return recovered;
        }

        // Determine typical chunk size from available chunks
        let chunk_size = available_chunks
            .values()
            .next()
            .map(|c| c.len())
            .unwrap_or(4096);

        for &chunk_id in missing_chunk_ids {
            // Find neighboring chunks for context
            let prev_chunk = if chunk_id > 0 {
                available_chunks.get(&(chunk_id - 1))
            } else {
                None
            };
            let next_chunk = available_chunks.get(&(chunk_id + 1));

            // Try recovery strategies in order of preference
            let recovered_data = if let (Some(prev), Some(next)) = (prev_chunk, next_chunk) {
                // Strategy 1: Interpolation from neighbors
                // Average byte values from prev and next chunks
                self.interpolate_chunk(prev, next, chunk_size)
            } else if let Some(neighbor) = prev_chunk.or(next_chunk) {
                // Strategy 2: Pattern completion from single neighbor
                self.complete_from_neighbor(neighbor, chunk_size, config)
            } else if !self.codebook.is_empty() {
                // Strategy 3: Codebook projection (find most common pattern)
                self.recover_from_codebook(chunk_size, config)
            } else {
                // Strategy 4: Zero-fill as last resort
                vec![0u8; chunk_size]
            };

            recovered.insert(chunk_id, recovered_data);
        }

        recovered
    }

    /// Interpolate a chunk from its neighbors by averaging byte values
    fn interpolate_chunk(&self, prev: &[u8], next: &[u8], target_size: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(target_size);

        for i in 0..target_size {
            let prev_byte = prev.get(i).copied().unwrap_or(0);
            let next_byte = next.get(i).copied().unwrap_or(0);

            // Simple average - works well for smooth data transitions
            let interpolated = ((prev_byte as u16 + next_byte as u16) / 2) as u8;
            result.push(interpolated);
        }

        result
    }

    /// Complete a chunk using patterns from a single neighbor
    fn complete_from_neighbor(
        &self,
        neighbor: &[u8],
        target_size: usize,
        config: &ReversibleVSAConfig,
    ) -> Vec<u8> {
        if !self.codebook.is_empty() {
            // Encode neighbor and project onto codebook for pattern match
            let neighbor_vec = SparseVec::encode_data(neighbor, config, None);
            let projected = self.project(&neighbor_vec);

            // Decode the projected vector to get cleaned pattern
            let decoded = projected.decode_data(config, None, target_size);
            if !decoded.is_empty() {
                return decoded;
            }
        }

        // Fallback: return a copy of neighbor (sequential chunks often similar)
        let mut result = Vec::with_capacity(target_size);
        for i in 0..target_size {
            result.push(neighbor.get(i).copied().unwrap_or(0));
        }
        result
    }

    /// Recover a chunk using the most representative codebook pattern
    fn recover_from_codebook(&self, target_size: usize, config: &ReversibleVSAConfig) -> Vec<u8> {
        if self.codebook.is_empty() {
            return vec![0u8; target_size];
        }

        // Find the codebook entry with highest average similarity to all others
        // (most "central" pattern that represents typical content)
        let mut best_pattern = &self.codebook[0];
        let mut best_centrality = f64::NEG_INFINITY;

        for pattern in &self.codebook {
            let mut total_sim = 0.0;
            for other in &self.codebook {
                total_sim += pattern.cosine(other);
            }
            let centrality = total_sim / self.codebook.len() as f64;
            if centrality > best_centrality {
                best_centrality = centrality;
                best_pattern = pattern;
            }
        }

        // Decode the most central pattern
        let decoded = best_pattern.decode_data(config, None, target_size);
        if !decoded.is_empty() {
            decoded
        } else {
            vec![0u8; target_size]
        }
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
    /// use embeddenator_retrieval::resonator::Resonator;
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
