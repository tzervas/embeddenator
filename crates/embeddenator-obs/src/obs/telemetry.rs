//! Telemetry Aggregation and Export
//!
//! Collects and aggregates observability data for export to monitoring
//! systems. Provides periodic snapshots and structured export formats.
//!
//! # Features
//!
//! - Periodic metric snapshots
//! - JSON export for external systems
//! - Performance counter aggregation
//! - Telemetry collection intervals
//! - Low-overhead sampling
//!
//! # Usage
//!
//! ```rust,ignore
//! use embeddenator_obs::telemetry::{Telemetry, TelemetryConfig};
//!
//! let config = TelemetryConfig::default();
//! let mut telemetry = Telemetry::new(config);
//!
//! // Record operations
//! telemetry.record_operation("query", 1250); // microseconds
//! telemetry.increment_counter("cache_hits");
//!
//! // Get snapshot for export
//! let snapshot = telemetry.snapshot();
//! println!("{}", snapshot.to_json());
//! ```

use crate::metrics::MetricsSnapshot;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Telemetry aggregation configuration.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Enable telemetry collection
    pub enabled: bool,
    /// Sample rate (0.0 to 1.0, where 1.0 = 100%)
    pub sample_rate: f64,
    /// Snapshot interval
    pub snapshot_interval: Duration,
    /// Maximum history to retain
    pub max_history_entries: usize,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sample_rate: 1.0,
            snapshot_interval: Duration::from_secs(60),
            max_history_entries: 100,
        }
    }
}

/// Main telemetry collector.
pub struct Telemetry {
    config: TelemetryConfig,
    start_time: Instant,
    operation_timings: HashMap<String, OperationStats>,
    counters: HashMap<String, u64>,
    gauges: HashMap<String, f64>,
    last_snapshot: Instant,
}

impl Telemetry {
    /// Create new telemetry collector.
    pub fn new(config: TelemetryConfig) -> Self {
        Self {
            config,
            start_time: Instant::now(),
            operation_timings: HashMap::new(),
            counters: HashMap::new(),
            gauges: HashMap::new(),
            last_snapshot: Instant::now(),
        }
    }

    /// Create with default configuration.
    pub fn default_config() -> Self {
        Self::new(TelemetryConfig::default())
    }

    /// Record operation timing (microseconds).
    pub fn record_operation(&mut self, name: &str, duration_us: u64) {
        if !self.config.enabled {
            return;
        }

        let stats = self
            .operation_timings
            .entry(name.to_string())
            .or_insert_with(OperationStats::new);

        stats.record(duration_us);
    }

    /// Increment a counter.
    pub fn increment_counter(&mut self, name: &str) {
        if !self.config.enabled {
            return;
        }

        *self.counters.entry(name.to_string()).or_insert(0) += 1;
    }

    /// Add to a counter.
    pub fn add_to_counter(&mut self, name: &str, value: u64) {
        if !self.config.enabled {
            return;
        }

        *self.counters.entry(name.to_string()).or_insert(0) += value;
    }

    /// Set gauge value.
    pub fn set_gauge(&mut self, name: &str, value: f64) {
        if !self.config.enabled {
            return;
        }

        self.gauges.insert(name.to_string(), value);
    }

    /// Get current snapshot.
    pub fn snapshot(&self) -> TelemetrySnapshot {
        let uptime = self.start_time.elapsed();
        let since_last = self.last_snapshot.elapsed();

        TelemetrySnapshot {
            timestamp_secs: uptime.as_secs(),
            uptime_secs: uptime.as_secs(),
            since_last_snapshot_secs: since_last.as_secs(),
            operation_stats: self.operation_timings.clone(),
            counters: self.counters.clone(),
            gauges: self.gauges.clone(),
            metrics: crate::metrics::metrics().snapshot(),
        }
    }

    /// Reset all collected data (useful for testing or periodic resets).
    pub fn reset(&mut self) {
        self.operation_timings.clear();
        self.counters.clear();
        self.gauges.clear();
        self.last_snapshot = Instant::now();
    }

    /// Get uptime in seconds.
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

/// Statistics for a single operation type.
#[derive(Debug, Clone)]
pub struct OperationStats {
    pub count: u64,
    pub total_us: u64,
    pub min_us: u64,
    pub max_us: u64,
    pub last_us: u64,
    /// Histogram buckets for percentile calculation (microseconds)
    pub histogram: Vec<u64>,
    /// Sum of squares for variance calculation
    pub sum_of_squares: f64,
}

impl OperationStats {
    fn new() -> Self {
        Self {
            count: 0,
            total_us: 0,
            min_us: u64::MAX,
            max_us: 0,
            last_us: 0,
            histogram: Vec::new(),
            sum_of_squares: 0.0,
        }
    }

    fn record(&mut self, duration_us: u64) {
        self.count += 1;
        self.total_us += duration_us;
        self.min_us = self.min_us.min(duration_us);
        self.max_us = self.max_us.max(duration_us);
        self.last_us = duration_us;

        // Update histogram (limited to 10000 samples for memory efficiency)
        if self.histogram.len() < 10000 {
            self.histogram.push(duration_us);
        }

        // Update sum of squares for variance calculation
        let val = duration_us as f64;
        self.sum_of_squares += val * val;
    }

    /// Calculate average duration.
    pub fn avg_us(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.total_us as f64 / self.count as f64
        }
    }

    /// Calculate operations per second (estimate based on total time).
    pub fn ops_per_sec(&self) -> f64 {
        if self.total_us == 0 {
            0.0
        } else {
            (self.count as f64 * 1_000_000.0) / self.total_us as f64
        }
    }

    /// Calculate standard deviation.
    pub fn std_dev_us(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }

        let mean = self.avg_us();
        let variance = (self.sum_of_squares / self.count as f64) - (mean * mean);
        variance.max(0.0).sqrt()
    }

    /// Calculate percentile from histogram (requires sorted data).
    pub fn percentile(&self, p: f64) -> u64 {
        if self.histogram.is_empty() {
            return 0;
        }

        let mut sorted = self.histogram.clone();
        sorted.sort_unstable();

        let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    /// Calculate median (P50).
    pub fn median_us(&self) -> u64 {
        self.percentile(50.0)
    }

    /// Calculate P95.
    pub fn p95_us(&self) -> u64 {
        self.percentile(95.0)
    }

    /// Calculate P99.
    pub fn p99_us(&self) -> u64 {
        self.percentile(99.0)
    }

    /// Count samples below threshold (for Prometheus histogram buckets).
    pub fn count_below(&self, threshold_us: u64) -> u64 {
        self.histogram.iter().filter(|&&x| x < threshold_us).count() as u64
    }
}

/// Point-in-time telemetry snapshot.
#[derive(Debug, Clone)]
pub struct TelemetrySnapshot {
    pub timestamp_secs: u64,
    pub uptime_secs: u64,
    pub since_last_snapshot_secs: u64,
    pub operation_stats: HashMap<String, OperationStats>,
    pub counters: HashMap<String, u64>,
    pub gauges: HashMap<String, f64>,
    pub metrics: MetricsSnapshot,
}

impl TelemetrySnapshot {
    /// Export as JSON string (requires serde feature).
    #[cfg(feature = "telemetry")]
    pub fn to_json(&self) -> String {
        use std::fmt::Write;

        let mut json = String::new();
        writeln!(json, "{{").unwrap();
        writeln!(json, r#"  "timestamp_secs": {},"#, self.timestamp_secs).unwrap();
        writeln!(json, r#"  "uptime_secs": {},"#, self.uptime_secs).unwrap();
        writeln!(
            json,
            r#"  "since_last_snapshot_secs": {},"#,
            self.since_last_snapshot_secs
        )
        .unwrap();

        // Operations
        writeln!(json, r#"  "operations": {{"#).unwrap();
        for (i, (name, stats)) in self.operation_stats.iter().enumerate() {
            let comma = if i < self.operation_stats.len() - 1 {
                ","
            } else {
                ""
            };
            writeln!(json, r#"    "{}": {{"#, name).unwrap();
            writeln!(json, r#"      "count": {},"#, stats.count).unwrap();
            writeln!(json, r#"      "avg_us": {:.2},"#, stats.avg_us()).unwrap();
            writeln!(json, r#"      "min_us": {},"#, stats.min_us).unwrap();
            writeln!(json, r#"      "max_us": {}"#, stats.max_us).unwrap();
            writeln!(json, r#"    }}{}"#, comma).unwrap();
        }
        writeln!(json, r#"  }},"#).unwrap();

        // Counters
        writeln!(json, r#"  "counters": {{"#).unwrap();
        for (i, (name, value)) in self.counters.iter().enumerate() {
            let comma = if i < self.counters.len() - 1 { "," } else { "" };
            writeln!(json, r#"    "{}": {}{}"#, name, value, comma).unwrap();
        }
        writeln!(json, r#"  }},"#).unwrap();

        // Gauges
        writeln!(json, r#"  "gauges": {{"#).unwrap();
        for (i, (name, value)) in self.gauges.iter().enumerate() {
            let comma = if i < self.gauges.len() - 1 { "," } else { "" };
            writeln!(json, r#"    "{}": {:.4}{}"#, name, value, comma).unwrap();
        }
        writeln!(json, r#"  }}"#).unwrap();

        writeln!(json, "}}").unwrap();
        json
    }

    #[cfg(not(feature = "telemetry"))]
    pub fn to_json(&self) -> String {
        "{{}}".to_string()
    }

    /// Format as human-readable summary.
    pub fn summary(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!(
            "=== Telemetry Snapshot (uptime: {}s) ===\n",
            self.uptime_secs
        ));

        if !self.operation_stats.is_empty() {
            output.push_str("\nOperations:\n");
            for (name, stats) in &self.operation_stats {
                output.push_str(&format!(
                    "  {}: count={}, avg={:.2}µs, min={}µs, max={}µs\n",
                    name,
                    stats.count,
                    stats.avg_us(),
                    stats.min_us,
                    stats.max_us
                ));
            }
        }

        if !self.counters.is_empty() {
            output.push_str("\nCounters:\n");
            for (name, value) in &self.counters {
                output.push_str(&format!("  {}: {}\n", name, value));
            }
        }

        if !self.gauges.is_empty() {
            output.push_str("\nGauges:\n");
            for (name, value) in &self.gauges {
                output.push_str(&format!("  {}: {:.4}\n", name, value));
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_basic() {
        let mut telemetry = Telemetry::default_config();

        telemetry.record_operation("query", 1500);
        telemetry.record_operation("query", 2000);
        telemetry.increment_counter("cache_hits");
        telemetry.set_gauge("memory_mb", 256.5);

        let snapshot = telemetry.snapshot();
        assert_eq!(snapshot.counters.get("cache_hits"), Some(&1));
        assert_eq!(snapshot.gauges.get("memory_mb"), Some(&256.5));

        let query_stats = snapshot.operation_stats.get("query").unwrap();
        assert_eq!(query_stats.count, 2);
        assert_eq!(query_stats.min_us, 1500);
        assert_eq!(query_stats.max_us, 2000);
    }

    #[test]
    fn test_operation_stats() {
        let mut stats = OperationStats::new();
        stats.record(100);
        stats.record(200);
        stats.record(150);

        assert_eq!(stats.count, 3);
        assert_eq!(stats.min_us, 100);
        assert_eq!(stats.max_us, 200);
        assert_eq!(stats.avg_us(), 150.0);
    }

    #[test]
    fn test_advanced_statistics() {
        let mut stats = OperationStats::new();

        // Record multiple samples
        for val in &[100, 150, 200, 250, 300, 350, 400, 450, 500] {
            stats.record(*val);
        }

        assert_eq!(stats.count, 9);
        assert_eq!(stats.avg_us(), 300.0);

        // Test percentiles
        let p50 = stats.percentile(50.0);
        assert!((250..=350).contains(&p50)); // Median should be ~300

        let p95 = stats.p95_us();
        assert!(p95 >= 400); // P95 should be high

        let p99 = stats.p99_us();
        assert!(p99 >= 450); // P99 should be very high

        // Test standard deviation (should be non-zero for varied data)
        let std_dev = stats.std_dev_us();
        assert!(std_dev > 0.0);
        assert!(std_dev < 200.0); // Reasonable for this data set
    }

    #[test]
    fn test_histogram_buckets() {
        let mut stats = OperationStats::new();

        stats.record(50);
        stats.record(150);
        stats.record(250);
        stats.record(750);
        stats.record(1500);

        // Count below thresholds
        assert_eq!(stats.count_below(100), 1); // Only 50
        assert_eq!(stats.count_below(500), 3); // 50, 150, 250
        assert_eq!(stats.count_below(1000), 4); // All except 1500
        assert_eq!(stats.count_below(2000), 5); // All samples
    }

    #[test]
    fn test_telemetry_reset() {
        let mut telemetry = Telemetry::default_config();

        telemetry.increment_counter("test");
        assert_eq!(telemetry.snapshot().counters.get("test"), Some(&1));

        telemetry.reset();
        assert_eq!(telemetry.snapshot().counters.get("test"), None);
    }

    #[test]
    fn test_snapshot_summary() {
        let mut telemetry = Telemetry::default_config();
        telemetry.record_operation("test_op", 500);

        let snapshot = telemetry.snapshot();
        let summary = snapshot.summary();

        assert!(summary.contains("Telemetry Snapshot"));
        assert!(summary.contains("test_op"));
    }

    #[test]
    fn test_disabled_telemetry() {
        let config = TelemetryConfig {
            enabled: false,
            ..TelemetryConfig::default()
        };
        let mut telemetry = Telemetry::new(config);

        telemetry.record_operation("query", 1000);
        telemetry.increment_counter("test");

        let snapshot = telemetry.snapshot();
        assert!(snapshot.operation_stats.is_empty());
        assert!(snapshot.counters.is_empty());
    }
}
