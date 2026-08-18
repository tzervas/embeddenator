# embeddenator-obs

<!-- FLEET-BADGES:BEGIN -->
[![CI](https://github.com/tzervas/embeddenator-obs/actions/workflows/fleet-ci.yml/badge.svg?branch=main)](https://github.com/tzervas/embeddenator-obs/actions/workflows/fleet-ci.yml?query=branch%3Amain)
[![Security](https://github.com/tzervas/embeddenator-obs/actions/workflows/fleet-security.yml/badge.svg?branch=main)](https://github.com/tzervas/embeddenator-obs/actions/workflows/fleet-security.yml?query=branch%3Amain)
<!-- FLEET-BADGES:END -->

Comprehensive observability infrastructure for the Embeddenator ecosystem.

**Independent component** extracted from the Embeddenator monolithic repository. Part of the [Embeddenator workspace](https://github.com/tzervas/embeddenator).

**Repository:** [https://github.com/tzervas/embeddenator-obs](https://github.com/tzervas/embeddenator-obs)

## Overview

**embeddenator-obs** provides production-grade observability components:

- **Metrics**: Lock-free atomic counters for high-performance tracking
- **Logging**: Structured logging with environment-based filtering  
- **Tracing**: Span instrumentation for distributed tracing
- **Telemetry**: Aggregation and export of observability data
- **Hi-Res Timing**: Picosecond-scale timing measurements
- **Test Metrics**: Comprehensive performance analysis for testing

## Status

**** - Fully migrated from embeddenator core.

**Implementation**: Core features complete, optional integrations available.

## Features

### Default Features

- `metrics`: Atomic performance counters (zero-overhead when not sampled)

### Optional Features

- `tracing`: Span instrumentation and distributed tracing
- `logging`: Structured logging (requires `tracing`)
- `telemetry`: Aggregation and JSON export
- `prometheus`: Prometheus metrics export format
- `opentelemetry`: OpenTelemetry/OTLP distributed tracing
- `streaming`: Real-time metric streaming with callbacks
- `advanced-stats`: Advanced statistical analysis (percentiles, std dev)
- `full`: Enable all features

## Installation

```toml
[dependencies]
embeddenator-obs = { git = "https://github.com/tzervas/embeddenator-obs", tag = "0.21.0" }

# Or with all features
embeddenator-obs = { git = "https://github.com/tzervas/embeddenator-obs", tag = "0.21.0", features = ["full"] }
```

## Usage

### Quick Start

```rust
use embeddenator_obs::{init_tracing, metrics, TestMetrics};
use std::time::Duration;

fn main() {
    // Initialize at startup
    init_tracing();
    
    // Track operations with metrics
    metrics().inc_sub_cache_hit();
    metrics().record_retrieval_query(Duration::from_micros(1500));
    
    // Performance testing
    let mut test_metrics = TestMetrics::new("my_operation");
    test_metrics.start_timing();
    // ... perform work ...
    test_metrics.stop_timing();
    println!("{}", test_metrics.summary());
}
```

### Metrics

Lock-free atomic counters for production use:

```rust
use embeddenator_obs::metrics;
use std::time::Duration;

// Increment counters
metrics().inc_sub_cache_hit();
metrics().inc_sub_cache_miss();
metrics().inc_index_cache_eviction();

// Record operation timing
metrics().record_retrieval_query(Duration::from_micros(1250));
metrics().record_rerank(Duration::from_millis(5));

// Get snapshot for monitoring
let snapshot = metrics().snapshot();
println!("Cache hit rate: {:.2}%", 
    100.0 * snapshot.sub_cache_hits as f64 / 
    (snapshot.sub_cache_hits + snapshot.sub_cache_misses) as f64
);
```

### Logging

Environment-based structured logging:

```bash
# Set log level
export EMBEDDENATOR_LOG=info
# or
export RUST_LOG=debug

# Set output format
export EMBEDDENATOR_LOG_FORMAT=json  # json, pretty, or compact
```

```rust
use embeddenator_obs::{info, warn, error, debug};

info("Application started");
warn("Cache size approaching limit");
error("Failed to connect to database");
debug("Processing batch 42");
```

### Tracing

Span instrumentation for performance analysis:

```rust
use embeddenator_obs::create_span;

fn process_query(query: &str) -> Result<Vec<u8>> {
    let _span = create_span("query", &[("dim", "768"), ("k", "10")]);
    
    // Nested spans work automatically
    let _inner = create_span("embedding_lookup", &[]);
    // ... work happens here ...
    
    Ok(vec![])
}
```

### Test Metrics

Comprehensive performance tracking for tests:

```rust
use embeddenator_obs::TestMetrics;
use std::time::Duration;

#[test]
fn benchmark_operation() {
    let mut metrics = TestMetrics::new("operation");
    
    // Time multiple iterations
    for _ in 0..100 {
        metrics.start_timing();
        // ... operation ...
        metrics.stop_timing();
        
        metrics.inc_op("iterations");
    }
    
    // Record custom metrics
    metrics.record_metric("accuracy", 0.95);
    metrics.record_memory(1024 * 1024);
    
    // Get detailed statistics
    let stats = metrics.timing_stats();
    println!("Mean: {:.2}µs, P95: {:.2}µs, P99: {:.2}µs",
        stats.avg_latency_us(),
        stats.p95_latency_us(),
        stats.p99_latency_us()
    );
    
    // Or print full summary
    println!("{}", metrics.summary());
}
```

### Telemetry

Aggregate and export observability data:

```rust
use embeddenator_obs::{Telemetry, TelemetryConfig};

let mut telemetry = Telemetry::default_config();

// Record operations
telemetry.record_operation("query", 1500);  // microseconds
telemetry.increment_counter("requests");
telemetry.set_gauge("memory_mb", 256.0);

// Export snapshot
let snapshot = telemetry.snapshot();
println!("{}", snapshot.to_json());  // JSON export
println!("{}", snapshot.summary());  // Human-readable
```

### Hi-Res Timing

Picosecond-scale timing for micro-benchmarks:

```rust
use embeddenator_obs::{HiResTimer, measure, measure_n};

// Single measurement
let timer = HiResTimer::start();
// ... work ...
let elapsed = timer.elapsed();
println!("Elapsed: {}", elapsed.format());

// Measure closure
let (result, timing) = measure(|| {
    // ... work ...
    42
});

// Multiple measurements with statistics
let (results, stats) = measure_n(1000, || {
    // ... work ...
});
println!("Stats: {}", stats.format());
```

## Integration with Other Components

### In embeddenator-vsa

```rust
use embeddenator_obs::{metrics, create_span};

pub fn bind_vectors(a: &Vector, b: &Vector) -> Vector {
    let _span = create_span("vsa_bind", &[("dim", &a.dim().to_string())]);
    
    metrics().inc_sub_cache_miss();
    
    // ... implementation ...
}
```

### In embeddenator-retrieval

```rust
use embeddenator_obs::{metrics, create_span};
use std::time::Instant;

pub fn query(index: &Index, query: &Vector, k: usize) -> Vec<Result> {
    let _span = create_span("retrieval_query", &[("k", &k.to_string())]);
    
    let start = Instant::now();
    let results = // ... perform query ...
    metrics().record_retrieval_query(start.elapsed());
    
    results
}
```

## Performance Overhead

- **Metrics** (feature disabled): 0ns - compiles to no-op
- **Metrics** (feature enabled): ~5-10ns per counter increment
- **Tracing** (feature disabled): 0ns - compiles to no-op  
- **Tracing** (feature enabled): ~50-100ns per span
- **Hi-Res Timing**: ~20-50ns per measurement

All overhead is pay-for-what-you-use via feature flags.

## Development

```bash
# Build with default features
cargo build --manifest-path embeddenator-obs/Cargo.toml

# Build with all features
cargo build --manifest-path embeddenator-obs/Cargo.toml --all-features

# Run tests
cargo test --manifest-path embeddenator-obs/Cargo.toml --all-features

# Run specific test
cargo test --manifest-path embeddenator-obs/Cargo.toml test_metrics_tracking
```

## Testing

```bash
# Unit tests
cargo test --manifest-path embeddenator-obs/Cargo.toml --lib

# Integration tests
cargo test --manifest-path embeddenator-obs/Cargo.toml --test integration_test

# With verbose output
cargo test --manifest-path embeddenator-obs/Cargo.toml --all-features -- --nocapture
```

## Cross-Repo Development

For local development across Embeddenator components:

```toml
[patch."https://github.com/tzervas/embeddenator-obs"]
embeddenator-obs = { path = "../embeddenator-obs" }
```

## Migration Notes

**Migrated from embeddenator core:**

- `testing::TestMetrics` → `embeddenator_obs::TestMetrics`
- `obs::metrics` → `embeddenator_obs::metrics`
- `obs::logging` → `embeddenator_obs::logging`
- Hi-res timing infrastructure preserved
- All existing functionality maintained

**New additions:**

- Span-based tracing infrastructure
- Telemetry aggregation and export
- Enhanced logging with multiple formats
- Comprehensive integration tests

## Architecture

See [ADR-016](https://github.com/tzervas/embeddenator/blob/main/docs/adr/ADR-016-component-decomposition.md) for component decomposition rationale.

## Advanced Features

### Prometheus Metrics Export

Export metrics in Prometheus text format for scraping:

```rust
use embeddenator_obs::{Telemetry, PrometheusExporter};

let mut telemetry = Telemetry::default_config();
telemetry.record_operation("query", 1500);
telemetry.increment_counter("requests");

let snapshot = telemetry.snapshot();
let exporter = PrometheusExporter::new("embeddenator");
let prometheus_text = exporter.export(&snapshot);

// Serve at /metrics endpoint
println!("{}", prometheus_text);
```

Output format:
```
# HELP embeddenator_requests Counter metric
# TYPE embeddenator_requests counter
embeddenator_requests 42
# HELP embeddenator_query_duration_us Operation duration histogram
# TYPE embeddenator_query_duration_us histogram
embeddenator_query_duration_us_bucket{le="100"} 5
embeddenator_query_duration_us_bucket{le="500"} 15
...
```

### OpenTelemetry Distributed Tracing

W3C Trace Context compatible spans for distributed systems:

```rust
use embeddenator_obs::{OtelSpan, OtelExporter};

// Create root span
let mut span = OtelSpan::new("http_request");
span.set_attribute("http.method", "GET");
span.add_event("request_received");

// Create child span
let mut child = OtelSpan::new_child("database_query", &span);
child.end();
span.end();

// Export trace context (propagate to downstream services)
let traceparent = span.to_traceparent();
// Send as HTTP header: traceparent: 00-<trace_id>-<span_id>-01
```

### Advanced Statistical Analysis

Percentiles, standard deviation, and histogram buckets:

```rust
use embeddenator_obs::Telemetry;

let mut telemetry = Telemetry::default_config();

// Record many samples
for i in 1..=1000 {
    telemetry.record_operation("api_call", 100 + i);
}

let snapshot = telemetry.snapshot();
let stats = snapshot.operation_stats.get("api_call").unwrap();

println!("Average: {:.2}µs", stats.avg_us());
println!("Std Dev: {:.2}µs", stats.std_dev_us());
println!("Median (P50): {}µs", stats.median_us());
println!("P95: {}µs", stats.p95_us());
println!("P99: {}µs", stats.p99_us());

// Histogram buckets for Prometheus
let below_1ms = stats.count_below(1000);
```

### Real-Time Metric Streaming

Callback-based streaming for live monitoring and alerting:

```rust
use embeddenator_obs::{MetricStream, MetricEvent};

let mut stream = MetricStream::new();

// Add threshold alerts
stream.add_threshold_alert("cpu", 80.0, true);

// Subscribe to events
stream.subscribe(|event| {
    match event {
        MetricEvent::Counter(name, value) => {
            println!("Counter {}: {}", name, value);
        }
        MetricEvent::ThresholdExceeded(name, value, threshold) => {
            alert!("Metric {} = {} exceeded {}", name, value, threshold);
        }
        _ => {}
    }
});

// Publish metrics
stream.publish_counter("requests", 100);
stream.publish_gauge("cpu_usage", 85.0); // Triggers alert
```

## Examples

```bash
# Run advanced features demo
cargo run --manifest-path embeddenator-obs/Cargo.toml --example advanced_features --all-features

# Run performance benchmark
cargo run --manifest-path embeddenator-obs/Cargo.toml --example performance_benchmark --all-features --release
```

## License

MIT
