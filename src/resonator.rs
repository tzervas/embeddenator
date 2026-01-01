//! Resonator Networks for VSA Pattern Completion
//!
//! Implements iterative refinement algorithms for:
//! - Pattern completion from noisy or partial inputs
//! - Factorization of compound representations
//! - Noise reduction through codebook projection

use crate::vsa::SparseVec;
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
    /// use embeddenator::SparseVec;
    ///
    /// let codebook = vec![SparseVec::from_data(b"pattern1"), SparseVec::from_data(b"pattern2")];
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
    /// use embeddenator::SparseVec;
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
    /// use embeddenator::resonator::Resonator;
    /// use embeddenator::SparseVec;
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