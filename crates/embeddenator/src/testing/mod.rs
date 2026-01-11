//! Testing Utilities for Embeddenator QA
//!
//! This module provides comprehensive testing infrastructure including:
//! - Granular performance metrics and timing
//! - Data integrity validation (bitflips, corruption, algebraic invariants)
//! - Storage footprint calculations
//! - Resilience testing helpers (chaos injection, noise tolerance)
//!
//! # Usage
//!
//! ```rust,ignore
//! use embeddenator::testing::{TestMetrics, IntegrityValidator, StorageFootprint};
//!
//! let mut metrics = TestMetrics::new("bind_operation");
//! metrics.start_timing();
//! let result = a.bind(&b);
//! metrics.stop_timing();
//! metrics.record_operation(result.nnz());
//! println!("{}", metrics.summary());
//! ```

use std::collections::HashMap;
use std::time::{Duration, Instant};

// Import types from re-exports
use crate::{BitslicedTritVec, Trit};

// ============================================================================
// PERFORMANCE METRICS
// ============================================================================

/// Granular performance metrics for test operations.
#[derive(Clone, Debug)]
pub struct TestMetrics {
    /// Operation name for reporting
    pub name: String,
    /// Individual timing samples (nanoseconds)
    pub timings_ns: Vec<u64>,
    /// Start time for current measurement
    start: Option<Instant>,
    /// Operation counts by category
    pub op_counts: HashMap<String, u64>,
    /// Custom numeric metrics
    pub custom_metrics: HashMap<String, f64>,
    /// Memory snapshots (bytes)
    pub memory_samples: Vec<usize>,
    /// Error/warning counts
    pub error_count: u64,
    pub warning_count: u64,
}

impl TestMetrics {
    /// Create new metrics collector for named operation.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            timings_ns: Vec::new(),
            start: None,
            op_counts: HashMap::new(),
            custom_metrics: HashMap::new(),
            memory_samples: Vec::new(),
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Start timing measurement.
    #[inline]
    pub fn start_timing(&mut self) {
        self.start = Some(Instant::now());
    }

    /// Stop timing and record sample.
    #[inline]
    pub fn stop_timing(&mut self) {
        if let Some(start) = self.start.take() {
            self.timings_ns.push(start.elapsed().as_nanos() as u64);
        }
    }

    /// Record a timed operation with closure.
    #[inline]
    pub fn time_operation<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.start_timing();
        let result = f();
        self.stop_timing();
        result
    }

    /// Increment operation counter.
    #[inline]
    pub fn inc_op(&mut self, category: &str) {
        *self.op_counts.entry(category.to_string()).or_insert(0) += 1;
    }

    /// Record custom metric.
    #[inline]
    pub fn record_metric(&mut self, name: &str, value: f64) {
        self.custom_metrics.insert(name.to_string(), value);
    }

    /// Record memory usage.
    #[inline]
    pub fn record_memory(&mut self, bytes: usize) {
        self.memory_samples.push(bytes);
    }

    /// Record an error.
    #[inline]
    pub fn record_error(&mut self) {
        self.error_count += 1;
    }

    /// Record a warning.
    #[inline]
    pub fn record_warning(&mut self) {
        self.warning_count += 1;
    }

    /// Get timing statistics.
    pub fn timing_stats(&self) -> TimingStats {
        if self.timings_ns.is_empty() {
            return TimingStats::default();
        }

        let mut sorted = self.timings_ns.clone();
        sorted.sort_unstable();

        let sum: u64 = sorted.iter().sum();
        let count = sorted.len() as f64;
        let mean = sum as f64 / count;

        let variance = sorted.iter().map(|&t| {
            let diff = t as f64 - mean;
            diff * diff
        }).sum::<f64>() / count;

        TimingStats {
            count: sorted.len(),
            min_ns: sorted[0],
            max_ns: sorted[sorted.len() - 1],
            mean_ns: mean,
            std_dev_ns: variance.sqrt(),
            p50_ns: sorted[sorted.len() / 2],
            p95_ns: sorted[(sorted.len() as f64 * 0.95) as usize],
            p99_ns: sorted[(sorted.len() as f64 * 0.99).min(sorted.len() as f64 - 1.0) as usize],
            total_ns: sum,
        }
    }

    /// Generate summary report.
    pub fn summary(&self) -> String {
        let stats = self.timing_stats();
        let mut report = format!("=== {} Metrics ===\n", self.name);

        if stats.count > 0 {
            report.push_str(&format!(
                "Timing: {} ops, mean={:.2}µs, p50={:.2}µs, p95={:.2}µs, p99={:.2}µs\n",
                stats.count,
                stats.mean_ns / 1000.0,
                stats.p50_ns as f64 / 1000.0,
                stats.p95_ns as f64 / 1000.0,
                stats.p99_ns as f64 / 1000.0,
            ));
            report.push_str(&format!(
                "        min={:.2}µs, max={:.2}µs, stddev={:.2}µs\n",
                stats.min_ns as f64 / 1000.0,
                stats.max_ns as f64 / 1000.0,
                stats.std_dev_ns / 1000.0,
            ));
        }

        if !self.op_counts.is_empty() {
            report.push_str("Operations: ");
            let ops: Vec<_> = self.op_counts.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            report.push_str(&ops.join(", "));
            report.push('\n');
        }

        if !self.custom_metrics.is_empty() {
            report.push_str("Metrics: ");
            let metrics: Vec<_> = self.custom_metrics.iter()
                .map(|(k, v)| format!("{}={:.4}", k, v))
                .collect();
            report.push_str(&metrics.join(", "));
            report.push('\n');
        }

        if !self.memory_samples.is_empty() {
            let max_mem = self.memory_samples.iter().max().unwrap_or(&0);
            let avg_mem = self.memory_samples.iter().sum::<usize>() / self.memory_samples.len();
            report.push_str(&format!(
                "Memory: peak={}KB, avg={}KB\n",
                max_mem / 1024,
                avg_mem / 1024,
            ));
        }

        if self.error_count > 0 || self.warning_count > 0 {
            report.push_str(&format!(
                "Issues: errors={}, warnings={}\n",
                self.error_count, self.warning_count
            ));
        }

        report
    }
}

/// Timing statistics.
#[derive(Clone, Debug, Default)]
pub struct TimingStats {
    pub count: usize,
    pub min_ns: u64,
    pub max_ns: u64,
    pub mean_ns: f64,
    pub std_dev_ns: f64,
    pub p50_ns: u64,
    pub p95_ns: u64,
    pub p99_ns: u64,
    pub total_ns: u64,
}

impl TimingStats {
    /// Total time as Duration.
    pub fn total_duration(&self) -> Duration {
        Duration::from_nanos(self.total_ns)
    }

    /// Throughput in operations per second.
    pub fn ops_per_sec(&self) -> f64 {
        if self.total_ns == 0 {
            0.0
        } else {
            (self.count as f64) / (self.total_ns as f64 / 1_000_000_000.0)
        }
    }
}

// ============================================================================
// DATA INTEGRITY VALIDATION
// ============================================================================

/// Results from integrity validation.
#[derive(Clone, Debug, Default)]
pub struct IntegrityReport {
    /// Total checks performed
    pub checks_total: u64,
    /// Checks that passed
    pub checks_passed: u64,
    /// Detected bitflips (single bit errors)
    pub bitflips_detected: u64,
    /// Multi-bit corruption events
    pub corruption_events: u64,
    /// Algebraic invariant violations
    pub invariant_violations: u64,
    /// Specific failure messages
    pub failures: Vec<String>,
}

impl IntegrityReport {
    /// Check if all validations passed.
    pub fn is_ok(&self) -> bool {
        self.checks_passed == self.checks_total && self.failures.is_empty()
    }

    /// Pass rate as percentage.
    pub fn pass_rate(&self) -> f64 {
        if self.checks_total == 0 {
            100.0
        } else {
            (self.checks_passed as f64 / self.checks_total as f64) * 100.0
        }
    }

    /// Record a passed check.
    pub fn pass(&mut self) {
        self.checks_total += 1;
        self.checks_passed += 1;
    }

    /// Record a failed check with message.
    pub fn fail(&mut self, msg: impl Into<String>) {
        self.checks_total += 1;
        self.failures.push(msg.into());
    }

    /// Record detected bitflip.
    pub fn record_bitflip(&mut self) {
        self.bitflips_detected += 1;
    }

    /// Record corruption event.
    pub fn record_corruption(&mut self) {
        self.corruption_events += 1;
    }

    /// Record invariant violation.
    pub fn record_invariant_violation(&mut self, msg: impl Into<String>) {
        self.invariant_violations += 1;
        self.failures.push(format!("INVARIANT: {}", msg.into()));
    }
}

/// Validates data integrity for VSA operations.
pub struct IntegrityValidator {
    /// Enable verbose logging
    pub verbose: bool,
}

impl IntegrityValidator {
    pub fn new() -> Self {
        Self { verbose: false }
    }

    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Validate bitsliced vector invariants.
    ///
    /// Checks:
    /// - No position has both pos and neg bits set
    /// - Length matches word count
    /// - Trailing bits are zero
    pub fn validate_bitsliced(&self, v: &BitslicedTritVec) -> IntegrityReport {
        let mut report = IntegrityReport::default();

        // Check no overlapping pos/neg bits
        let words = BitslicedTritVec::word_count(v.len());
        for w in 0..words {
            let overlap = v.pos_word(w) & v.neg_word(w);
            if overlap != 0 {
                let count = overlap.count_ones();
                report.record_corruption();
                report.fail(format!(
                    "Word {} has {} positions with both pos and neg set",
                    w, count
                ));
            } else {
                report.pass();
            }
        }

        // Check trailing bits in last word are zero
        if words > 0 {
            let trailing_bits = v.len() % 64;
            if trailing_bits != 0 {
                let mask = !((1u64 << trailing_bits) - 1);
                let pos_trailing = v.pos_word(words - 1) & mask;
                let neg_trailing = v.neg_word(words - 1) & mask;
                if pos_trailing != 0 || neg_trailing != 0 {
                    report.fail(format!(
                        "Trailing bits not zero: pos={:016x}, neg={:016x}",
                        pos_trailing, neg_trailing
                    ));
                } else {
                    report.pass();
                }
            }
        }

        report
    }

    /// Validate algebraic invariants for bind operation.
    ///
    /// Checks:
    /// - Self-inverse: A ⊙ A = all +1 at non-zero positions
    /// - Commutativity: A ⊙ B = B ⊙ A
    pub fn validate_bind_invariants(
        &self,
        a: &BitslicedTritVec,
        b: &BitslicedTritVec,
    ) -> IntegrityReport {
        let mut report = IntegrityReport::default();

        // Self-inverse check
        let a_squared = a.bind(a);
        let a_nnz = a.nnz();
        let a2_pos = a_squared.to_sparse().pos.len();
        let a2_neg = a_squared.to_sparse().neg.len();
        
        if a2_neg != 0 {
            report.record_invariant_violation(format!(
                "Self-inverse violation: A⊙A has {} negative trits (should be 0)",
                a2_neg
            ));
        } else if a2_pos != a_nnz {
            report.record_invariant_violation(format!(
                "Self-inverse violation: A⊙A has {} positive trits (expected {})",
                a2_pos, a_nnz
            ));
        } else {
            report.pass();
        }

        // Commutativity check
        let ab = a.bind(b);
        let ba = b.bind(a);
        let ab_sparse = ab.to_sparse();
        let ba_sparse = ba.to_sparse();
        
        if ab_sparse.pos != ba_sparse.pos || ab_sparse.neg != ba_sparse.neg {
            report.record_invariant_violation("Commutativity violation: A⊙B ≠ B⊙A");
        } else {
            report.pass();
        }

        report
    }

    /// Validate bundle operation properties.
    pub fn validate_bundle_invariants(
        &self,
        a: &BitslicedTritVec,
        b: &BitslicedTritVec,
    ) -> IntegrityReport {
        let mut report = IntegrityReport::default();

        // Commutativity check
        let ab = a.bundle(b);
        let ba = b.bundle(a);
        let ab_sparse = ab.to_sparse();
        let ba_sparse = ba.to_sparse();

        if ab_sparse.pos != ba_sparse.pos || ab_sparse.neg != ba_sparse.neg {
            report.record_invariant_violation("Bundle commutativity violation: A⊕B ≠ B⊕A");
        } else {
            report.pass();
        }

        // Conflict cancel: P + N = Z
        let conflict_pos: Vec<usize> = a.to_sparse().pos.iter()
            .filter(|&&i| b.to_sparse().neg.contains(&i))
            .copied()
            .collect();
        
        for &pos in &conflict_pos {
            let result_trit = ab.get(pos);
            if result_trit != Trit::Z {
                report.fail(format!(
                    "Conflict cancel violation at {}: P+N={:?} (expected Z)",
                    pos, result_trit
                ));
            } else {
                report.pass();
            }
        }

        report
    }

    /// Detect potential bitflips by comparing two vectors.
    pub fn detect_bitflips(
        &self,
        expected: &BitslicedTritVec,
        actual: &BitslicedTritVec,
    ) -> IntegrityReport {
        let mut report = IntegrityReport::default();

        if expected.len() != actual.len() {
            report.fail(format!(
                "Length mismatch: expected {}, got {}",
                expected.len(), actual.len()
            ));
            return report;
        }

        let words = BitslicedTritVec::word_count(expected.len());
        let mut total_flips = 0u64;

        for w in 0..words {
            let pos_diff = expected.pos_word(w) ^ actual.pos_word(w);
            let neg_diff = expected.neg_word(w) ^ actual.neg_word(w);
            
            let pos_flips = pos_diff.count_ones();
            let neg_flips = neg_diff.count_ones();
            
            total_flips += pos_flips as u64 + neg_flips as u64;
            
            if pos_flips == 1 && neg_flips == 0 {
                report.record_bitflip();
            } else if pos_flips == 0 && neg_flips == 1 {
                report.record_bitflip();
            } else if pos_flips + neg_flips > 0 {
                report.record_corruption();
            }
        }

        if total_flips == 0 {
            report.pass();
        } else {
            report.fail(format!("Detected {} total bit differences", total_flips));
        }

        report
    }
}

impl Default for IntegrityValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// STORAGE FOOTPRINT CALCULATIONS
// ============================================================================

/// Storage footprint analysis for encoded data.
#[derive(Clone, Debug, Default)]
pub struct StorageFootprint {
    /// Original raw data size in bytes
    pub raw_bytes: u64,
    /// Encoded sparse representation size (estimated)
    pub sparse_bytes: u64,
    /// Encoded bitsliced representation size
    pub bitsliced_bytes: u64,
    /// Codebook overhead bytes
    pub codebook_bytes: u64,
    /// Manifest/metadata bytes
    pub metadata_bytes: u64,
    /// Number of chunks
    pub chunk_count: u64,
    /// Total dimension
    pub dimension: usize,
    /// Non-zero elements
    pub nnz: usize,
}

impl StorageFootprint {
    /// Create new footprint analysis.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record raw data size.
    pub fn with_raw_bytes(mut self, bytes: u64) -> Self {
        self.raw_bytes = bytes;
        self
    }

    /// Calculate from a sparse vector.
    pub fn from_sparse(sparse: &crate::vsa::SparseVec, dim: usize) -> Self {
        let nnz = sparse.pos.len() + sparse.neg.len();
        // Sparse storage: 2 vecs of usize indices
        let sparse_bytes = (nnz * std::mem::size_of::<usize>()) as u64;
        
        Self {
            sparse_bytes,
            dimension: dim,
            nnz,
            ..Default::default()
        }
    }

    /// Calculate from a bitsliced vector.
    pub fn from_bitsliced(bs: &BitslicedTritVec) -> Self {
        let words = BitslicedTritVec::word_count(bs.len());
        // Bitsliced: 2 planes of u64 words
        let bitsliced_bytes = (words * 2 * std::mem::size_of::<u64>()) as u64;
        
        Self {
            bitsliced_bytes,
            dimension: bs.len(),
            nnz: bs.nnz(),
            ..Default::default()
        }
    }

    /// Total encoded size.
    pub fn total_encoded_bytes(&self) -> u64 {
        self.sparse_bytes.max(self.bitsliced_bytes) + self.codebook_bytes + self.metadata_bytes
    }

    /// Compression ratio (raw / encoded).
    pub fn compression_ratio(&self) -> f64 {
        if self.raw_bytes == 0 {
            0.0
        } else {
            self.raw_bytes as f64 / self.total_encoded_bytes() as f64
        }
    }

    /// Space savings percentage.
    pub fn space_savings_pct(&self) -> f64 {
        if self.raw_bytes == 0 {
            0.0
        } else {
            (1.0 - (self.total_encoded_bytes() as f64 / self.raw_bytes as f64)) * 100.0
        }
    }

    /// Bits per trit (for encoded representation).
    pub fn bits_per_trit(&self) -> f64 {
        if self.dimension == 0 {
            0.0
        } else {
            (self.bitsliced_bytes * 8) as f64 / self.dimension as f64
        }
    }

    /// Density (nnz / dimension).
    pub fn density(&self) -> f64 {
        if self.dimension == 0 {
            0.0
        } else {
            self.nnz as f64 / self.dimension as f64
        }
    }

    /// Generate summary report.
    pub fn summary(&self) -> String {
        format!(
            "Storage Footprint:\n\
             - Raw:        {} bytes\n\
             - Sparse:     {} bytes\n\
             - Bitsliced:  {} bytes\n\
             - Codebook:   {} bytes\n\
             - Metadata:   {} bytes\n\
             - Total:      {} bytes\n\
             - Ratio:      {:.2}x\n\
             - Savings:    {:.1}%\n\
             - Dimension:  {}\n\
             - NNZ:        {} ({:.2}% density)\n\
             - Bits/trit:  {:.2}",
            self.raw_bytes,
            self.sparse_bytes,
            self.bitsliced_bytes,
            self.codebook_bytes,
            self.metadata_bytes,
            self.total_encoded_bytes(),
            self.compression_ratio(),
            self.space_savings_pct(),
            self.dimension,
            self.nnz,
            self.density() * 100.0,
            self.bits_per_trit(),
        )
    }
}

// ============================================================================
// CHAOS / RESILIENCE TESTING
// ============================================================================

/// Chaos injection utilities for resilience testing.
pub struct ChaosInjector {
    /// Random seed for reproducibility
    seed: u64,
    /// Injection probability (0.0 - 1.0)
    probability: f64,
}

impl ChaosInjector {
    /// Create new chaos injector with seed.
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            probability: 0.01, // 1% default
        }
    }

    /// Set injection probability.
    pub fn with_probability(mut self, p: f64) -> Self {
        self.probability = p.clamp(0.0, 1.0);
        self
    }

    /// Inject random bitflips into a bitsliced vector.
    pub fn inject_bitflips(
        &self,
        v: &mut BitslicedTritVec,
        count: usize,
    ) -> Vec<usize> {
        use std::collections::HashSet;

        let mut flipped = Vec::new();
        let mut seen = HashSet::new();
        let mut state = self.seed;

        for _ in 0..count {
            // Simple LCG for reproducibility
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let pos = (state as usize) % v.len();

            if seen.insert(pos) {
                let current = v.get(pos);
                let new_trit = match current {
                    Trit::P => Trit::N,
                    Trit::N => Trit::P,
                    Trit::Z => {
                        if state % 2 == 0 {
                            Trit::P
                        } else {
                            Trit::N
                        }
                    }
                };
                v.set(pos, new_trit);
                flipped.push(pos);
            }
        }

        flipped
    }

    /// Inject noise by randomly setting trits to zero.
    pub fn inject_erasures(
        &self,
        v: &mut BitslicedTritVec,
        count: usize,
    ) -> Vec<usize> {
        let mut erased = Vec::new();
        let mut state = self.seed.wrapping_add(12345);

        for _ in 0..count {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let pos = (state as usize) % v.len();

            if v.get(pos) != Trit::Z {
                v.set(pos, Trit::Z);
                erased.push(pos);
            }
        }

        erased
    }

    /// Create corrupted copy with specified error rate.
    pub fn corrupt_copy(
        &self,
        v: &BitslicedTritVec,
        error_rate: f64,
    ) -> BitslicedTritVec {
        let mut corrupted = v.clone();
        let errors = ((v.len() as f64) * error_rate) as usize;
        self.inject_bitflips(&mut corrupted, errors);
        corrupted
    }
}

// ============================================================================
// TEST ASSERTIONS
// ============================================================================

/// Assert that two bitsliced vectors are exactly equal.
#[macro_export]
macro_rules! assert_bitsliced_eq {
    ($left:expr, $right:expr) => {
        {
            let left = &$left;
            let right = &$right;
            assert_eq!(left.len(), right.len(), "Length mismatch");
            for i in 0..left.len() {
                assert_eq!(
                    left.get(i), right.get(i),
                    "Mismatch at position {}: left={:?}, right={:?}",
                    i, left.get(i), right.get(i)
                );
            }
        }
    };
    ($left:expr, $right:expr, $($arg:tt)+) => {
        {
            let left = &$left;
            let right = &$right;
            assert_eq!(left.len(), right.len(), "Length mismatch: {}", format!($($arg)+));
            for i in 0..left.len() {
                assert_eq!(
                    left.get(i), right.get(i),
                    "Mismatch at position {}: left={:?}, right={:?} - {}",
                    i, left.get(i), right.get(i), format!($($arg)+)
                );
            }
        }
    };
}

/// Assert that cosine similarity is above threshold.
#[macro_export]
macro_rules! assert_cosine_above {
    ($a:expr, $b:expr, $threshold:expr) => {
        {
            let cos = $a.cosine(&$b);
            assert!(
                cos >= $threshold,
                "Cosine similarity {:.4} below threshold {:.4}",
                cos, $threshold
            );
        }
    };
}

/// Assert that an operation preserves nnz.
#[macro_export]
macro_rules! assert_nnz_preserved {
    ($before:expr, $after:expr) => {
        assert_eq!(
            $before.nnz(), $after.nnz(),
            "NNZ changed: {} -> {}",
            $before.nnz(), $after.nnz()
        );
    };
}

// ============================================================================
// TESTS FOR THE TESTING MODULE ITSELF
// ============================================================================
// Tests moved to tests/testing_infrastructure.rs for better organization
