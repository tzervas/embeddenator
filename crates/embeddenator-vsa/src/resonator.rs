//! Resonator Networks for Learned Codebooks
//!
//! This module implements resonator networks - a neural-inspired architecture for
//! iterative factorization and semantic variable inference in Vector Symbolic
//! Architectures (VSA).
//!
//! # Architecture
//!
//! Resonator networks solve the factorization problem: given a composite vector
//! that is the binding of multiple factors, recover the original factors.
//!
//! ```text
//! Input: x = f₁ ⊙ f₂ ⊙ f₃  (bound composite)
//!
//! Resonator Loop:
//!   estimate_f₁ = unbind(x, estimate_f₂, estimate_f₃)
//!   estimate_f₁ = cleanup(estimate_f₁, codebook_1)
//!   estimate_f₂ = unbind(x, estimate_f₁, estimate_f₃)
//!   estimate_f₂ = cleanup(estimate_f₂, codebook_2)
//!   estimate_f₃ = unbind(x, estimate_f₁, estimate_f₂)
//!   estimate_f₃ = cleanup(estimate_f₃, codebook_3)
//!
//! Repeat until convergence or max iterations
//! ```
//!
//! # Gradient Learning
//!
//! The codebook vectors can be optimized through gradient descent on a
//! reconstruction loss, enabling learned representations that better capture
//! the structure of training data.
//!
//! # Example
//!
//! ```rust,ignore
//! use embeddenator_vsa::{Resonator, ResonatorConfig, Codebook};
//!
//! let config = ResonatorConfig::default();
//! let mut resonator = Resonator::new(config);
//!
//! // Add codebooks for each factor type
//! resonator.add_codebook("type", type_codebook);
//! resonator.add_codebook("position", position_codebook);
//!
//! // Factorize a composite vector
//! let result = resonator.factorize(&composite_vec, 100)?;
//! println!("Type: {:?}", result.factors.get("type"));
//! println!("Position: {:?}", result.factors.get("position"));
//! ```

use std::collections::HashMap;

use crate::codebook::Codebook;
use crate::vsa::{SparseVec, DIM};

/// Configuration for resonator network
#[derive(Clone, Debug)]
pub struct ResonatorConfig {
    /// Maximum iterations before giving up
    pub max_iterations: usize,
    /// Convergence threshold (cosine similarity to previous estimate)
    pub convergence_threshold: f64,
    /// Learning rate for gradient updates during training
    pub learning_rate: f64,
    /// Momentum coefficient for gradient updates
    pub momentum: f64,
    /// Weight decay for regularization
    pub weight_decay: f64,
    /// Temperature for softmax cleanup (lower = sharper)
    pub temperature: f64,
    /// Whether to use soft cleanup (weighted average) vs hard (argmax)
    pub soft_cleanup: bool,
    /// Number of top candidates to consider in soft cleanup
    pub soft_cleanup_top_k: usize,
}

impl Default for ResonatorConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            convergence_threshold: 0.99,
            learning_rate: 0.01,
            momentum: 0.9,
            weight_decay: 1e-5,
            temperature: 0.1,
            soft_cleanup: true,
            soft_cleanup_top_k: 8,
        }
    }
}

/// Result of resonator factorization
#[derive(Clone, Debug)]
pub struct FactorizationResult {
    /// Recovered factors by codebook name
    pub factors: HashMap<String, RecoveredFactor>,
    /// Number of iterations used
    pub iterations: usize,
    /// Whether convergence was achieved
    pub converged: bool,
    /// Final reconstruction quality (cosine similarity to input)
    pub reconstruction_quality: f64,
    /// Per-iteration convergence metrics
    pub convergence_history: Vec<f64>,
}

/// A recovered factor from factorization
#[derive(Clone, Debug)]
pub struct RecoveredFactor {
    /// Best matching basis vector ID
    pub best_match_id: u32,
    /// Best matching basis vector (if available)
    pub best_match: Option<SparseVec>,
    /// Similarity to best match
    pub confidence: f64,
    /// The estimated factor vector
    pub estimate: SparseVec,
    /// Top-k candidate matches with similarities
    pub candidates: Vec<(u32, f64)>,
}

/// Gradient state for a single codebook during training
#[derive(Clone, Debug)]
struct CodebookGradient {
    /// Accumulated gradients for each basis vector (by ID)
    gradients: HashMap<u32, Vec<f64>>,
    /// Momentum terms for each basis vector
    momentum: HashMap<u32, Vec<f64>>,
}

impl CodebookGradient {
    fn new() -> Self {
        Self {
            gradients: HashMap::new(),
            momentum: HashMap::new(),
        }
    }

    fn zero_gradients(&mut self) {
        for grad in self.gradients.values_mut() {
            grad.fill(0.0);
        }
    }
}

/// Resonator network for iterative factorization and learning
pub struct Resonator {
    /// Configuration
    config: ResonatorConfig,
    /// Named codebooks for each factor type
    codebooks: HashMap<String, Codebook>,
    /// Order of factor estimation (determines unbinding order)
    factor_order: Vec<String>,
    /// Gradient accumulators for training
    gradients: HashMap<String, CodebookGradient>,
    /// Training statistics
    stats: ResonatorStats,
}

/// Statistics from resonator training
#[derive(Clone, Debug, Default)]
pub struct ResonatorStats {
    /// Total factorization attempts
    pub total_factorizations: u64,
    /// Successful convergences
    pub converged_count: u64,
    /// Average iterations to converge
    pub avg_iterations: f64,
    /// Average reconstruction quality
    pub avg_reconstruction_quality: f64,
    /// Total training steps
    pub training_steps: u64,
    /// Current average loss
    pub current_loss: f64,
}

impl Resonator {
    /// Create a new resonator network
    pub fn new(config: ResonatorConfig) -> Self {
        Self {
            config,
            codebooks: HashMap::new(),
            factor_order: Vec::new(),
            gradients: HashMap::new(),
            stats: ResonatorStats::default(),
        }
    }

    /// Add a codebook for a factor type
    pub fn add_codebook(&mut self, name: &str, codebook: Codebook) {
        self.codebooks.insert(name.to_string(), codebook);
        self.factor_order.push(name.to_string());
        self.gradients
            .insert(name.to_string(), CodebookGradient::new());
    }

    /// Get a codebook by name
    pub fn get_codebook(&self, name: &str) -> Option<&Codebook> {
        self.codebooks.get(name)
    }

    /// Get mutable codebook by name
    pub fn get_codebook_mut(&mut self, name: &str) -> Option<&mut Codebook> {
        self.codebooks.get_mut(name)
    }

    /// Get current statistics
    pub fn stats(&self) -> &ResonatorStats {
        &self.stats
    }

    /// Factorize a composite vector into its constituent factors
    ///
    /// The composite is assumed to be the binding (⊙) of factors from
    /// each codebook. The resonator iteratively estimates each factor.
    pub fn factorize(&mut self, composite: &SparseVec) -> FactorizationResult {
        self.factorize_with_iterations(composite, self.config.max_iterations)
    }

    /// Factorize with a specific maximum iteration count
    pub fn factorize_with_iterations(
        &mut self,
        composite: &SparseVec,
        max_iterations: usize,
    ) -> FactorizationResult {
        if self.factor_order.is_empty() {
            return FactorizationResult {
                factors: HashMap::new(),
                iterations: 0,
                converged: true,
                reconstruction_quality: 0.0,
                convergence_history: Vec::new(),
            };
        }

        // Initialize estimates randomly or from partial unbinding
        let mut estimates: HashMap<String, SparseVec> = self
            .factor_order
            .iter()
            .map(|name| (name.clone(), SparseVec::random()))
            .collect();

        let mut convergence_history = Vec::new();
        let mut prev_estimates = estimates.clone();
        let mut converged = false;

        for iteration in 0..max_iterations {
            // Update each factor estimate
            for name in &self.factor_order.clone() {
                // Compute the unbinding: x ⊙ f₂⁻¹ ⊙ f₃⁻¹ ... to estimate f₁
                let mut unbound = composite.clone();
                for (other_name, other_estimate) in &estimates {
                    if other_name != name {
                        // Unbind by binding with inverse (self-inverse property for sparse ternary)
                        unbound = unbound.bind(other_estimate);
                    }
                }

                // Cleanup: project onto codebook
                let cleaned = if let Some(codebook) = self.codebooks.get(name) {
                    self.cleanup(&unbound, codebook)
                } else {
                    unbound
                };

                estimates.insert(name.clone(), cleaned);
            }

            // Check convergence
            let mut min_similarity = 1.0f64;
            for name in &self.factor_order {
                let curr = estimates.get(name).unwrap();
                let prev = prev_estimates.get(name).unwrap();
                let sim = curr.cosine(prev);
                min_similarity = min_similarity.min(sim);
            }
            convergence_history.push(min_similarity);

            if min_similarity >= self.config.convergence_threshold {
                converged = true;
                self.stats.converged_count += 1;
            }

            prev_estimates = estimates.clone();

            if converged {
                // Compute final metrics
                let reconstruction = self.reconstruct(&estimates);
                let quality = reconstruction.cosine(composite);

                self.stats.total_factorizations += 1;
                self.stats.avg_reconstruction_quality = (self.stats.avg_reconstruction_quality
                    * (self.stats.total_factorizations - 1) as f64
                    + quality)
                    / self.stats.total_factorizations as f64;
                self.stats.avg_iterations = (self.stats.avg_iterations
                    * (self.stats.total_factorizations - 1) as f64
                    + (iteration + 1) as f64)
                    / self.stats.total_factorizations as f64;

                return self.build_result(
                    estimates,
                    iteration + 1,
                    true,
                    quality,
                    convergence_history,
                );
            }
        }

        // Did not converge
        self.stats.total_factorizations += 1;
        let reconstruction = self.reconstruct(&estimates);
        let quality = reconstruction.cosine(composite);

        self.build_result(
            estimates,
            max_iterations,
            false,
            quality,
            convergence_history,
        )
    }

    /// Cleanup operation: project vector onto codebook
    fn cleanup(&self, vec: &SparseVec, codebook: &Codebook) -> SparseVec {
        if codebook.basis_vectors.is_empty() {
            return vec.clone();
        }

        if self.config.soft_cleanup {
            // Soft cleanup: weighted average of top-k matches
            self.soft_cleanup(vec, codebook)
        } else {
            // Hard cleanup: return the best match
            self.hard_cleanup(vec, codebook)
        }
    }

    /// Hard cleanup: return best matching basis vector
    fn hard_cleanup(&self, vec: &SparseVec, codebook: &Codebook) -> SparseVec {
        let mut best_sim = f64::NEG_INFINITY;
        let mut best_vec = vec.clone();

        for basis in &codebook.basis_vectors {
            let sim = vec.cosine(&basis.vector);
            if sim > best_sim {
                best_sim = sim;
                best_vec = basis.vector.clone();
            }
        }

        best_vec
    }

    /// Soft cleanup: weighted average of top-k matches
    fn soft_cleanup(&self, vec: &SparseVec, codebook: &Codebook) -> SparseVec {
        // Compute similarities to all basis vectors
        let mut similarities: Vec<(usize, f64)> = codebook
            .basis_vectors
            .iter()
            .enumerate()
            .map(|(i, basis)| (i, vec.cosine(&basis.vector)))
            .collect();

        // Sort by similarity descending
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top-k
        let top_k: Vec<_> = similarities
            .into_iter()
            .take(self.config.soft_cleanup_top_k)
            .collect();

        if top_k.is_empty() {
            return vec.clone();
        }

        // Compute softmax weights
        let max_sim = top_k.first().map(|t| t.1).unwrap_or(0.0);
        let weights: Vec<f64> = top_k
            .iter()
            .map(|(_, sim)| ((sim - max_sim) / self.config.temperature).exp())
            .collect();
        let weight_sum: f64 = weights.iter().sum();

        if weight_sum == 0.0 {
            return codebook.basis_vectors[top_k[0].0].vector.clone();
        }

        // Weighted bundle of top-k vectors
        let weighted_vecs: Vec<_> = top_k
            .iter()
            .zip(weights.iter())
            .map(|((idx, _), w)| (codebook.basis_vectors[*idx].vector.clone(), *w / weight_sum))
            .collect();

        weighted_bundle(&weighted_vecs)
    }

    /// Reconstruct composite from factors
    fn reconstruct(&self, factors: &HashMap<String, SparseVec>) -> SparseVec {
        let mut result = SparseVec::random(); // Start with identity-ish
        let mut first = true;

        for name in &self.factor_order {
            if let Some(factor) = factors.get(name) {
                if first {
                    result = factor.clone();
                    first = false;
                } else {
                    result = result.bind(factor);
                }
            }
        }

        result
    }

    /// Build the factorization result
    fn build_result(
        &self,
        estimates: HashMap<String, SparseVec>,
        iterations: usize,
        converged: bool,
        quality: f64,
        convergence_history: Vec<f64>,
    ) -> FactorizationResult {
        let mut factors = HashMap::new();

        for (name, estimate) in estimates {
            if let Some(codebook) = self.codebooks.get(&name) {
                // Find best match and candidates
                let mut candidates: Vec<(u32, f64)> = codebook
                    .basis_vectors
                    .iter()
                    .map(|b| (b.id, estimate.cosine(&b.vector)))
                    .collect();
                candidates
                    .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                let best = candidates.first().cloned().unwrap_or((0, 0.0));
                let best_match = codebook
                    .basis_vectors
                    .iter()
                    .find(|b| b.id == best.0)
                    .map(|b| b.vector.clone());

                factors.insert(
                    name,
                    RecoveredFactor {
                        best_match_id: best.0,
                        best_match,
                        confidence: best.1,
                        estimate,
                        candidates: candidates.into_iter().take(10).collect(),
                    },
                );
            } else {
                factors.insert(
                    name,
                    RecoveredFactor {
                        best_match_id: 0,
                        best_match: None,
                        confidence: 0.0,
                        estimate,
                        candidates: Vec::new(),
                    },
                );
            }
        }

        FactorizationResult {
            factors,
            iterations,
            converged,
            reconstruction_quality: quality,
            convergence_history,
        }
    }

    /// Train the codebook using gradient-based optimization
    ///
    /// Takes training pairs of (composite, expected_factors) and optimizes
    /// the codebook vectors to minimize reconstruction loss.
    pub fn train(
        &mut self,
        training_data: &[TrainingExample],
        epochs: usize,
    ) -> Result<TrainingResult, String> {
        if training_data.is_empty() {
            return Err("No training data provided".to_string());
        }

        let mut loss_history = Vec::new();
        let batch_size = 32.min(training_data.len());

        for _epoch in 0..epochs {
            let mut epoch_loss = 0.0;
            let mut batch_count = 0;

            // Process in batches
            for batch in training_data.chunks(batch_size) {
                self.zero_gradients();

                let mut batch_loss = 0.0;
                for example in batch {
                    // Forward pass: factorize
                    let result = self.factorize(&example.composite);

                    // Compute loss: negative reconstruction quality + factor matching loss
                    let recon_loss = 1.0 - result.reconstruction_quality;
                    let factor_loss = self.compute_factor_loss(&result, &example.expected_factors);
                    let total_loss = recon_loss + factor_loss;
                    batch_loss += total_loss;

                    // Backward pass: accumulate gradients
                    self.backward(&example.composite, &result, &example.expected_factors);
                }

                // Apply gradients
                self.apply_gradients(batch.len());

                epoch_loss += batch_loss;
                batch_count += 1;
            }

            let avg_loss = epoch_loss / (batch_count * batch_size) as f64;
            loss_history.push(avg_loss);
            self.stats.current_loss = avg_loss;
            self.stats.training_steps += 1;
        }

        Ok(TrainingResult {
            final_loss: *loss_history.last().unwrap_or(&0.0),
            loss_history,
            epochs_completed: epochs,
        })
    }

    /// Zero all gradient accumulators
    fn zero_gradients(&mut self) {
        for grad in self.gradients.values_mut() {
            grad.zero_gradients();
        }
    }

    /// Compute factor matching loss
    fn compute_factor_loss(
        &self,
        result: &FactorizationResult,
        expected: &HashMap<String, u32>,
    ) -> f64 {
        let mut loss = 0.0;
        let mut count = 0;

        for (name, expected_id) in expected {
            if let Some(factor) = result.factors.get(name) {
                // Loss: 1 - confidence if wrong match, 0 if correct
                if factor.best_match_id != *expected_id {
                    loss += 1.0 - factor.confidence;
                }
                count += 1;
            }
        }

        if count > 0 {
            loss / count as f64
        } else {
            0.0
        }
    }

    /// Backward pass: compute gradients
    fn backward(
        &mut self,
        _composite: &SparseVec,
        result: &FactorizationResult,
        expected: &HashMap<String, u32>,
    ) {
        // For each factor, compute gradient to move codebook vectors
        // toward better reconstruction
        for (name, expected_id) in expected {
            if let (Some(factor), Some(codebook)) =
                (result.factors.get(name), self.codebooks.get(name))
            {
                if let Some(grad_state) = self.gradients.get_mut(name) {
                    // Gradient: push expected closer to estimate, push others away
                    for basis in &codebook.basis_vectors {
                        let grad = grad_state
                            .gradients
                            .entry(basis.id)
                            .or_insert_with(|| vec![0.0; DIM]);

                        // Compute gradient contribution
                        let sim = factor.estimate.cosine(&basis.vector);

                        if basis.id == *expected_id {
                            // Positive gradient: move toward estimate
                            add_gradient_toward(grad, &factor.estimate, &basis.vector);
                        } else if sim > 0.5 {
                            // Negative gradient: move away if too similar
                            add_gradient_away(grad, &factor.estimate);
                        }
                    }
                }
            }
        }
    }

    /// Apply accumulated gradients to codebook vectors
    fn apply_gradients(&mut self, batch_size: usize) {
        let lr = self.config.learning_rate / batch_size as f64;
        let momentum = self.config.momentum;
        let weight_decay = self.config.weight_decay;

        for (name, grad_state) in &mut self.gradients {
            if let Some(codebook) = self.codebooks.get_mut(name) {
                for basis in &mut codebook.basis_vectors {
                    if let Some(grad) = grad_state.gradients.get(&basis.id) {
                        // Get or create momentum buffer
                        let mom = grad_state
                            .momentum
                            .entry(basis.id)
                            .or_insert_with(|| vec![0.0; DIM]);

                        // Update each dimension
                        let mut new_pos = Vec::new();
                        let mut new_neg = Vec::new();

                        for dim in 0..DIM {
                            // Compute momentum-updated gradient
                            mom[dim] = momentum * mom[dim] + grad[dim];

                            // Get current value for this dimension
                            let is_pos = basis.vector.pos.contains(&dim);
                            let is_neg = basis.vector.neg.contains(&dim);
                            let current_val = if is_pos {
                                1.0
                            } else if is_neg {
                                -1.0
                            } else {
                                0.0
                            };

                            // Apply gradient with weight decay
                            let new_val = current_val + lr * mom[dim] - weight_decay * current_val;

                            // Threshold to sparse ternary
                            if new_val > 0.3 {
                                new_pos.push(dim);
                            } else if new_val < -0.3 {
                                new_neg.push(dim);
                            }
                        }

                        basis.vector.pos = new_pos;
                        basis.vector.neg = new_neg;
                    }
                }
            }
        }
    }

    /// Infer semantic variables from an encoded vector
    ///
    /// This performs factorization and returns a semantic interpretation
    /// of the encoded content.
    pub fn infer_semantics(&mut self, vec: &SparseVec) -> SemanticInference {
        let result = self.factorize(vec);

        let mut inferred_variables = HashMap::new();
        let mut confidence_scores = HashMap::new();

        for (name, factor) in &result.factors {
            // Get the semantic label from the codebook
            if let Some(codebook) = self.codebooks.get(name) {
                if let Some(basis) = codebook
                    .basis_vectors
                    .iter()
                    .find(|b| b.id == factor.best_match_id)
                {
                    let label = basis
                        .label
                        .clone()
                        .unwrap_or_else(|| format!("id_{}", basis.id));
                    inferred_variables.insert(name.clone(), label);
                    confidence_scores.insert(name.clone(), factor.confidence);
                }
            }
        }

        SemanticInference {
            variables: inferred_variables,
            confidences: confidence_scores,
            raw_factors: result.factors,
            reconstruction_quality: result.reconstruction_quality,
        }
    }
}

/// A training example for the resonator
#[derive(Clone, Debug)]
pub struct TrainingExample {
    /// The composite vector (binding of factors)
    pub composite: SparseVec,
    /// Expected factor IDs by codebook name
    pub expected_factors: HashMap<String, u32>,
}

impl TrainingExample {
    /// Create a new training example
    pub fn new(composite: SparseVec, expected_factors: HashMap<String, u32>) -> Self {
        Self {
            composite,
            expected_factors,
        }
    }

    /// Create training example by binding vectors from codebooks
    pub fn from_codebooks(
        codebooks: &HashMap<String, &Codebook>,
        factor_ids: &HashMap<String, u32>,
    ) -> Option<Self> {
        let mut composite: Option<SparseVec> = None;

        for (name, id) in factor_ids {
            if let Some(codebook) = codebooks.get(name) {
                if let Some(basis) = codebook.basis_vectors.iter().find(|b| b.id == *id) {
                    composite = Some(match composite {
                        None => basis.vector.clone(),
                        Some(c) => c.bind(&basis.vector),
                    });
                }
            }
        }

        composite.map(|c| Self::new(c, factor_ids.clone()))
    }
}

/// Result of training
#[derive(Clone, Debug)]
pub struct TrainingResult {
    /// Final loss value
    pub final_loss: f64,
    /// Loss over epochs
    pub loss_history: Vec<f64>,
    /// Epochs completed
    pub epochs_completed: usize,
}

/// Result of semantic inference
#[derive(Clone, Debug)]
pub struct SemanticInference {
    /// Inferred semantic variables by name
    pub variables: HashMap<String, String>,
    /// Confidence scores for each variable
    pub confidences: HashMap<String, f64>,
    /// Raw factor results
    pub raw_factors: HashMap<String, RecoveredFactor>,
    /// Overall reconstruction quality
    pub reconstruction_quality: f64,
}

/// Perform weighted bundle of vectors
fn weighted_bundle(weighted_vecs: &[(SparseVec, f64)]) -> SparseVec {
    if weighted_vecs.is_empty() {
        return SparseVec::random();
    }

    // Accumulate weighted votes for each dimension
    let mut dim_votes: Vec<f64> = vec![0.0; DIM];

    for (vec, weight) in weighted_vecs {
        for &pos in &vec.pos {
            if pos < DIM {
                dim_votes[pos] += weight;
            }
        }
        for &neg in &vec.neg {
            if neg < DIM {
                dim_votes[neg] -= weight;
            }
        }
    }

    // Threshold to get sparse ternary result
    let threshold = 0.3;
    let mut pos = Vec::new();
    let mut neg = Vec::new();

    for (dim, &vote) in dim_votes.iter().enumerate() {
        if vote > threshold {
            pos.push(dim);
        } else if vote < -threshold {
            neg.push(dim);
        }
    }

    SparseVec { pos, neg }
}

/// Add gradient to move vector toward target (standalone function)
fn add_gradient_toward(grad: &mut [f64], target: &SparseVec, current: &SparseVec) {
    // Simple gradient: increase overlap with target
    for &pos in &target.pos {
        if pos < DIM {
            grad[pos] += 1.0;
        }
    }
    for &neg in &target.neg {
        if neg < DIM {
            grad[neg] -= 1.0;
        }
    }

    // Decrease current-specific dimensions not in target
    for &pos in &current.pos {
        if !target.pos.contains(&pos) && pos < DIM {
            grad[pos] -= 0.5;
        }
    }
    for &neg in &current.neg {
        if !target.neg.contains(&neg) && neg < DIM {
            grad[neg] += 0.5;
        }
    }
}

/// Add gradient to move vector away from target (standalone function)
fn add_gradient_away(grad: &mut [f64], target: &SparseVec) {
    // Push away from target
    for &pos in &target.pos {
        if pos < DIM {
            grad[pos] -= 0.5;
        }
    }
    for &neg in &target.neg {
        if neg < DIM {
            grad[neg] += 0.5;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebook::BasisVector;

    #[test]
    fn test_resonator_config_default() {
        let config = ResonatorConfig::default();
        assert_eq!(config.max_iterations, 50);
        assert!((config.convergence_threshold - 0.99).abs() < 0.001);
    }

    #[test]
    fn test_resonator_new() {
        let config = ResonatorConfig::default();
        let resonator = Resonator::new(config);
        assert!(resonator.codebooks.is_empty());
        assert!(resonator.factor_order.is_empty());
    }

    #[test]
    fn test_resonator_add_codebook() {
        let mut resonator = Resonator::new(ResonatorConfig::default());
        let mut codebook = Codebook::new(DIM);
        codebook.basis_vectors.push(BasisVector {
            id: 0,
            vector: SparseVec::random(),
            label: Some("test".to_string()),
            weight: 1.0,
        });

        resonator.add_codebook("type", codebook);

        assert!(resonator.get_codebook("type").is_some());
        assert_eq!(resonator.factor_order, vec!["type"]);
    }

    #[test]
    fn test_factorization_empty_resonator() {
        let mut resonator = Resonator::new(ResonatorConfig::default());
        let vec = SparseVec::random();
        let result = resonator.factorize(&vec);

        assert!(result.converged);
        assert_eq!(result.iterations, 0);
        assert!(result.factors.is_empty());
    }

    #[test]
    fn test_factorization_single_factor() {
        let mut resonator = Resonator::new(ResonatorConfig::default());

        // Create a codebook with a few basis vectors
        let mut codebook = Codebook::new(DIM);
        let target_vec = SparseVec::random();
        codebook.basis_vectors.push(BasisVector {
            id: 1,
            vector: target_vec.clone(),
            label: Some("target".to_string()),
            weight: 1.0,
        });
        codebook.basis_vectors.push(BasisVector {
            id: 2,
            vector: SparseVec::random(),
            label: Some("distractor".to_string()),
            weight: 1.0,
        });

        resonator.add_codebook("type", codebook);

        // The "composite" is just the target itself (no binding)
        let result = resonator.factorize(&target_vec);

        assert!(result.factors.contains_key("type"));
        let factor = result.factors.get("type").unwrap();

        // Should match the target with high confidence
        assert_eq!(factor.best_match_id, 1);
        assert!(factor.confidence > 0.5);
    }

    #[test]
    fn test_training_example_creation() {
        let vec = SparseVec::random();
        let mut factors = HashMap::new();
        factors.insert("type".to_string(), 1u32);

        let example = TrainingExample::new(vec.clone(), factors);
        assert_eq!(example.composite.pos, vec.pos);
        assert_eq!(example.expected_factors.get("type"), Some(&1u32));
    }

    #[test]
    fn test_weighted_bundle() {
        let v1 = SparseVec {
            pos: vec![1, 2, 3],
            neg: vec![4, 5],
        };
        let v2 = SparseVec {
            pos: vec![1, 6],
            neg: vec![4, 7],
        };

        let result = weighted_bundle(&[(v1, 0.6), (v2, 0.4)]);

        // Dimension 1 should be positive (0.6 + 0.4 = 1.0 > 0.3)
        assert!(result.pos.contains(&1));
        // Dimension 4 should be negative (-0.6 - 0.4 = -1.0 < -0.3)
        assert!(result.neg.contains(&4));
    }

    #[test]
    fn test_semantic_inference() {
        let mut resonator = Resonator::new(ResonatorConfig::default());

        let mut codebook = Codebook::new(DIM);
        let vec = SparseVec::random();
        codebook.basis_vectors.push(BasisVector {
            id: 42,
            vector: vec.clone(),
            label: Some("semantic_label".to_string()),
            weight: 1.0,
        });

        resonator.add_codebook("content", codebook);

        let inference = resonator.infer_semantics(&vec);

        assert!(inference.variables.contains_key("content"));
        assert!(inference.confidences.contains_key("content"));
    }
}
