//! Prometheus Metrics Export
//!
//! Exports observability metrics in Prometheus text format for scraping
//! by Prometheus servers or compatible monitoring systems.
//!
//! # Features
//!
//! - Counter metrics export
//! - Gauge metrics export
//! - Histogram buckets for operation timings
//! - Label support for metric dimensions
//! - Text format output (Prometheus standard)
//!
//! # Usage
//!
//! ```rust,ignore
//! use embeddenator_obs::prometheus::PrometheusExporter;
//!
//! let exporter = PrometheusExporter::new("embeddenator");
//! let snapshot = telemetry.snapshot();
//! let prometheus_text = exporter.export(&snapshot);
//!
//! // Serve via HTTP endpoint
//! // GET /metrics -> prometheus_text
//! ```

use crate::obs::telemetry::TelemetrySnapshot;
use std::fmt::Write;

/// Prometheus metrics exporter.
pub struct PrometheusExporter {
    /// Metric prefix (e.g., "embeddenator")
    prefix: String,
    /// Include help text
    include_help: bool,
    /// Include type annotations
    include_type: bool,
}

impl PrometheusExporter {
    /// Create new Prometheus exporter with prefix.
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            include_help: true,
            include_type: true,
        }
    }

    /// Disable help text (reduces output size).
    pub fn without_help(mut self) -> Self {
        self.include_help = false;
        self
    }

    /// Disable type annotations.
    pub fn without_type(mut self) -> Self {
        self.include_type = false;
        self
    }

    /// Export snapshot to Prometheus text format.
    pub fn export(&self, snapshot: &TelemetrySnapshot) -> String {
        let mut output = String::with_capacity(4096);

        // Export counters
        for (name, value) in &snapshot.counters {
            self.write_counter(&mut output, name, *value);
        }

        // Export gauges
        for (name, value) in &snapshot.gauges {
            self.write_gauge(&mut output, name, *value);
        }

        // Export operation timings as histograms
        for (name, stats) in &snapshot.operation_stats {
            self.write_histogram(&mut output, name, stats);
        }

        // Export built-in metrics
        self.write_counter(
            &mut output,
            "sub_cache_hits",
            snapshot.metrics.sub_cache_hits,
        );
        self.write_counter(
            &mut output,
            "sub_cache_misses",
            snapshot.metrics.sub_cache_misses,
        );
        self.write_counter(
            &mut output,
            "index_cache_evictions",
            snapshot.metrics.index_cache_evictions,
        );
        self.write_counter(
            &mut output,
            "poison_recoveries_total",
            snapshot.metrics.poison_recoveries_total,
        );

        // Export uptime as gauge
        self.write_gauge(&mut output, "uptime_seconds", snapshot.uptime_secs as f64);

        output
    }

    fn write_counter(&self, output: &mut String, name: &str, value: u64) {
        let metric_name = format!("{}_{}", self.prefix, sanitize_name(name));

        if self.include_help {
            writeln!(output, "# HELP {} Counter metric", metric_name).ok();
        }
        if self.include_type {
            writeln!(output, "# TYPE {} counter", metric_name).ok();
        }
        writeln!(output, "{} {}", metric_name, value).ok();
    }

    fn write_gauge(&self, output: &mut String, name: &str, value: f64) {
        let metric_name = format!("{}_{}", self.prefix, sanitize_name(name));

        if self.include_help {
            writeln!(output, "# HELP {} Gauge metric", metric_name).ok();
        }
        if self.include_type {
            writeln!(output, "# TYPE {} gauge", metric_name).ok();
        }
        writeln!(output, "{} {}", metric_name, value).ok();
    }

    fn write_histogram(
        &self,
        output: &mut String,
        name: &str,
        stats: &crate::obs::telemetry::OperationStats,
    ) {
        let metric_name = format!("{}_{}_duration_us", self.prefix, sanitize_name(name));

        if self.include_help {
            writeln!(
                output,
                "# HELP {} Operation duration histogram",
                metric_name
            )
            .ok();
        }
        if self.include_type {
            writeln!(output, "# TYPE {} histogram", metric_name).ok();
        }

        // Histogram buckets (microseconds): 100us, 500us, 1ms, 5ms, 10ms, 50ms, 100ms, +Inf
        let buckets = [100, 500, 1000, 5000, 10000, 50000, 100000];
        let mut cumulative = 0u64;

        for bucket in &buckets {
            cumulative += stats.count_below(*bucket);
            writeln!(
                output,
                "{}_bucket{{le=\"{}\"}} {}",
                metric_name, bucket, cumulative
            )
            .ok();
        }

        writeln!(
            output,
            "{}_bucket{{le=\"+Inf\"}} {}",
            metric_name, stats.count
        )
        .ok();
        writeln!(output, "{}_sum {}", metric_name, stats.total_us).ok();
        writeln!(output, "{}_count {}", metric_name, stats.count).ok();
    }
}

impl Default for PrometheusExporter {
    fn default() -> Self {
        Self::new("embeddenator")
    }
}

/// Sanitize metric name for Prometheus (replace invalid chars with underscore).
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obs::telemetry::Telemetry;

    #[test]
    fn test_prometheus_export() {
        let mut telemetry = Telemetry::default_config();
        telemetry.increment_counter("requests");
        telemetry.set_gauge("queue_size", 42.5);
        telemetry.record_operation("query", 1250);

        let snapshot = telemetry.snapshot();
        let exporter = PrometheusExporter::new("test");
        let output = exporter.export(&snapshot);

        assert!(output.contains("test_requests"));
        assert!(output.contains("test_queue_size"));
        assert!(output.contains("test_query_duration_us"));
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("valid_name"), "valid_name");
        assert_eq!(sanitize_name("invalid-name"), "invalid_name");
        assert_eq!(sanitize_name("name.with.dots"), "name_with_dots");
        assert_eq!(sanitize_name("name:with:colons"), "name_with_colons");
    }

    #[test]
    fn test_histogram_buckets() {
        let mut telemetry = Telemetry::default_config();
        telemetry.record_operation("query", 50); // < 100us
        telemetry.record_operation("query", 750); // < 1000us
        telemetry.record_operation("query", 2500); // < 5000us

        let snapshot = telemetry.snapshot();
        let exporter = PrometheusExporter::new("test");
        let output = exporter.export(&snapshot);

        assert!(output.contains("test_query_duration_us_bucket"));
        assert!(output.contains("test_query_duration_us_sum"));
        assert!(output.contains("test_query_duration_us_count 3"));
    }

    #[test]
    fn test_without_help_and_type() {
        let mut telemetry = Telemetry::default_config();
        telemetry.increment_counter("test");

        let snapshot = telemetry.snapshot();
        let exporter = PrometheusExporter::new("app").without_help().without_type();
        let output = exporter.export(&snapshot);

        assert!(!output.contains("# HELP"));
        assert!(!output.contains("# TYPE"));
        assert!(output.contains("app_test"));
    }
}
