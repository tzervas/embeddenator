//! Test Metrics and Performance Tracking
//!
//! Comprehensive performance metrics for testing and benchmarking.
//! Extracted from embeddenator core testing utilities.
//!
//! # Features
//!
//! - Multi-sample timing with percentile statistics
//! - Operation counting by category
//! - Custom metric recording
//! - Memory usage tracking
//! - Error/warning counting
//! - Automatic statistical analysis
//!
//! # Usage
//!
//! ```rust
//! use embeddenator_obs::test_metrics::TestMetrics;
//!
//! let mut metrics = TestMetrics::new("bind_operation");
//! metrics.start_timing();
//! // ... perform operation ...
//! metrics.stop_timing();
//! metrics.record_operation(1000); // e.g., items processed
//!
//! println!("{}", metrics.summary());
//! ```

use std::collections::HashMap;
use std::time::{Duration, Instant};

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

    /// Record operation count.
    #[inline]
    pub fn record_operation(&mut self, count: u64) {
        *self.op_counts.entry("operations".to_string()).or_insert(0) += count;
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

        let variance = sorted
            .iter()
            .map(|&t| {
                let diff = t as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / count;

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
            let ops: Vec<_> = self
                .op_counts
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            report.push_str(&ops.join(", "));
            report.push('\n');
        }

        if !self.custom_metrics.is_empty() {
            report.push_str("Metrics: ");
            let metrics: Vec<_> = self
                .custom_metrics
                .iter()
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

    /// Reset all metrics (useful for reusing the same collector).
    pub fn reset(&mut self) {
        self.timings_ns.clear();
        self.start = None;
        self.op_counts.clear();
        self.custom_metrics.clear();
        self.memory_samples.clear();
        self.error_count = 0;
        self.warning_count = 0;
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

    /// Average latency in microseconds.
    pub fn avg_latency_us(&self) -> f64 {
        self.mean_ns / 1000.0
    }

    /// P50 latency in microseconds.
    pub fn p50_latency_us(&self) -> f64 {
        self.p50_ns as f64 / 1000.0
    }

    /// P95 latency in microseconds.
    pub fn p95_latency_us(&self) -> f64 {
        self.p95_ns as f64 / 1000.0
    }

    /// P99 latency in microseconds.
    pub fn p99_latency_us(&self) -> f64 {
        self.p99_ns as f64 / 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_timing() {
        let mut metrics = TestMetrics::new("test_op");

        metrics.start_timing();
        std::thread::sleep(Duration::from_micros(100));
        metrics.stop_timing();

        assert_eq!(metrics.timings_ns.len(), 1);
        assert!(metrics.timings_ns[0] >= 100_000); // at least 100µs in ns
    }

    #[test]
    fn test_time_operation() {
        let mut metrics = TestMetrics::new("closure_test");

        let result = metrics.time_operation(|| {
            std::thread::sleep(Duration::from_micros(50));
            42
        });

        assert_eq!(result, 42);
        assert_eq!(metrics.timings_ns.len(), 1);
    }

    #[test]
    fn test_operation_counting() {
        let mut metrics = TestMetrics::new("ops");

        metrics.inc_op("reads");
        metrics.inc_op("reads");
        metrics.inc_op("writes");

        assert_eq!(metrics.op_counts.get("reads"), Some(&2));
        assert_eq!(metrics.op_counts.get("writes"), Some(&1));
    }

    #[test]
    fn test_custom_metrics() {
        let mut metrics = TestMetrics::new("custom");

        metrics.record_metric("accuracy", 0.95);
        metrics.record_metric("loss", 0.12);

        assert_eq!(metrics.custom_metrics.get("accuracy"), Some(&0.95));
        assert_eq!(metrics.custom_metrics.get("loss"), Some(&0.12));
    }

    #[test]
    fn test_memory_tracking() {
        let mut metrics = TestMetrics::new("memory");

        metrics.record_memory(1024);
        metrics.record_memory(2048);
        metrics.record_memory(1536);

        assert_eq!(metrics.memory_samples.len(), 3);
    }

    #[test]
    fn test_error_warning_counts() {
        let mut metrics = TestMetrics::new("issues");

        metrics.record_error();
        metrics.record_error();
        metrics.record_warning();

        assert_eq!(metrics.error_count, 2);
        assert_eq!(metrics.warning_count, 1);
    }

    #[test]
    fn test_timing_stats() {
        let mut metrics = TestMetrics::new("stats");

        // Add known values for easy verification
        metrics.timings_ns = vec![100, 200, 150, 300, 250];

        let stats = metrics.timing_stats();
        assert_eq!(stats.count, 5);
        assert_eq!(stats.min_ns, 100);
        assert_eq!(stats.max_ns, 300);
        assert_eq!(stats.mean_ns, 200.0);
    }

    #[test]
    fn test_summary_generation() {
        let mut metrics = TestMetrics::new("summary_test");

        metrics.timings_ns = vec![1000, 2000, 1500];
        metrics.inc_op("test");
        metrics.record_metric("score", 0.85);

        let summary = metrics.summary();

        assert!(summary.contains("summary_test"));
        assert!(summary.contains("Timing"));
        assert!(summary.contains("Operations"));
        assert!(summary.contains("Metrics"));
    }

    #[test]
    fn test_reset() {
        let mut metrics = TestMetrics::new("reset_test");

        metrics.start_timing();
        std::thread::sleep(Duration::from_micros(10));
        metrics.stop_timing();
        metrics.inc_op("test");
        metrics.record_error();

        assert_eq!(metrics.timings_ns.len(), 1);
        assert_eq!(metrics.error_count, 1);

        metrics.reset();

        assert_eq!(metrics.timings_ns.len(), 0);
        assert_eq!(metrics.error_count, 0);
        assert_eq!(metrics.op_counts.len(), 0);
    }

    #[test]
    fn test_empty_stats() {
        let metrics = TestMetrics::new("empty");
        let stats = metrics.timing_stats();

        assert_eq!(stats.count, 0);
        assert_eq!(stats.total_ns, 0);
    }

    #[test]
    fn test_throughput_calculation() {
        let stats = TimingStats {
            count: 1000,
            total_ns: 1_000_000_000, // 1 second
            ..Default::default()
        };

        assert_eq!(stats.ops_per_sec(), 1000.0);
    }
}
