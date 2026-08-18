//! # embeddenator-obs
//!
//! Observability: metrics, logging, tracing, and telemetry for Embeddenator.
//!
//! Extracted from embeddenator core as part of Phase 2A component decomposition.
//!
//! ## Features
//!
//! This crate provides comprehensive observability infrastructure:
//!
//! - **Metrics**: Lock-free atomic counters for performance tracking
//! - **Logging**: Structured logging with environment-based filtering
//! - **Tracing**: Span instrumentation for distributed tracing
//! - **Telemetry**: Aggregation and export of observability data
//! - **Hi-Res Timing**: Picosecond-scale timing for performance analysis
//! - **Test Metrics**: Comprehensive testing/benchmarking utilities
//!
//! ## Feature Flags
//!
//! - `metrics` (default): Enable atomic performance counters
//! - `tracing`: Enable span instrumentation and distributed tracing
//! - `logging`: Enable structured logging (implies tracing)
//! - `telemetry`: Enable telemetry aggregation and export
//! - `prometheus`: Enable Prometheus metrics export format
//! - `opentelemetry`: Enable OpenTelemetry distributed tracing
//! - `streaming`: Enable real-time metric streaming with callbacks
//! - `advanced-stats`: Enable advanced statistical analysis (percentiles, std dev)
//! - `full`: Enable all features
//!
//! ## Quick Start
//!
//! ```rust
//! use embeddenator_obs::{init_tracing, TestMetrics};
//!
//! // Initialize at startup
//! init_tracing();
//!
//! // Track performance in tests
//! let mut metrics = TestMetrics::new("operation");
//! metrics.start_timing();
//! // ... perform work ...
//! metrics.stop_timing();
//! println!("{}", metrics.summary());
//! ```
//!
//! ## Metrics Usage
//!
//! ```rust
//! use embeddenator_obs::metrics;
//! use std::time::Duration;
//!
//! // Increment counters
//! metrics().inc_sub_cache_hit();
//! metrics().inc_sub_cache_miss();
//!
//! // Record durations
//! metrics().record_retrieval_query(Duration::from_micros(1500));
//!
//! // Get snapshot
//! let snapshot = metrics().snapshot();
//! println!("Cache hits: {}", snapshot.sub_cache_hits);
//! ```
//!
//! ## Tracing Usage
//!
//! ```rust,ignore
//! use embeddenator_obs::create_span;
//!
//! let _span = create_span("query_operation", &[("dim", "768")]);
//! // Work happens here, timing is automatic
//! ```
//!
//! ## Architecture
//!
//! See [ADR-016](https://github.com/tzervas/embeddenator/blob/main/docs/adr/ADR-016-component-decomposition.md)
//! for component decomposition rationale.

pub mod obs;
pub use obs::*;

// Re-export commonly used types for convenience
pub use obs::{
    create_span, init_tracing, metrics, EventLevel, HiResMetrics, HiResTimer, HiResTimestamp,
    MetricEvent, MetricStream, Metrics, MetricsSnapshot, OperationStats, OtelExporter, OtelSpan,
    PrometheusExporter, SpanGuard, SpanKind, SpanStatus, Telemetry, TelemetryConfig,
    TelemetrySnapshot, TestMetrics, ThresholdAlert, TimingStats,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_loads() {
        // Verify module loads successfully
        let _ = metrics();
    }

    #[test]
    fn test_metrics_accessible() {
        let snapshot = metrics().snapshot();
        // Should compile and not panic
        let _ = snapshot.sub_cache_hits;
    }

    #[test]
    fn test_init_tracing_no_panic() {
        init_tracing();
    }

    #[test]
    fn test_metrics_new() {
        // Verify core types are accessible
        let metrics = Metrics::new();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.poison_recoveries_total, 0);
    }
}
