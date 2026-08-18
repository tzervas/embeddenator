# Embeddenator-Obs Integration Guide

This guide shows how to integrate embeddenator-obs into other Embeddenator components.

## Quick Integration Checklist

- [ ] Add dependency to Cargo.toml
- [ ] Initialize observability at startup
- [ ] Add metrics to critical paths
- [ ] Instrument functions with spans
- [ ] Add test metrics to benchmarks
- [ ] Configure logging for development

## Step 1: Add Dependency

### In Component Cargo.toml

```toml
[dependencies]
embeddenator-obs = { version = "0.20.0-alpha.1", features = ["metrics"] }

# For development builds, add tracing
[dev-dependencies]
embeddenator-obs = { version = "0.20.0-alpha.1", features = ["full"] }
```

### For Local Development

```toml
[patch."https://github.com/tzervas/embeddenator-obs"]
embeddenator-obs = { path = "../embeddenator-obs" }
```

## Step 2: Initialize at Startup

### In main.rs or lib.rs

```rust
use embeddenator_obs::init_tracing;

fn main() {
    // Initialize once at startup
    init_tracing();
    
    // Your application code
}
```

### Configuration via Environment

```bash
# Set log level
export EMBEDDENATOR_LOG=info
export RUST_LOG=debug

# Set output format
export EMBEDDENATOR_LOG_FORMAT=json
```

## Step 3: Add Metrics to Critical Paths

### Example: VSA Operations

```rust
use embeddenator_obs::metrics;

pub fn bind(a: &Vector, b: &Vector) -> Vector {
    // Increment operation counter
    metrics().inc_sub_cache_miss();
    
    // Your implementation
    let result = compute_bind(a, b);
    
    result
}

pub fn bundle(a: &Vector, b: &Vector) -> Vector {
    metrics().inc_sub_cache_hit();
    // Implementation
}
```

### Example: Retrieval Operations

```rust
use embeddenator_obs::metrics;
use std::time::Instant;

pub fn query(index: &Index, query: &Vector, k: usize) -> Vec<Result> {
    let start = Instant::now();
    
    // Perform query
    let results = index.search(query, k);
    
    // Record timing
    metrics().record_retrieval_query(start.elapsed());
    
    results
}

pub fn rerank(candidates: &[Result], query: &Vector) -> Vec<Result> {
    let start = Instant::now();
    
    // Reranking logic
    let reranked = // ...
    
    metrics().record_rerank(start.elapsed());
    
    reranked
}
```

## Step 4: Instrument with Spans

### Function-Level Instrumentation

```rust
use embeddenator_obs::create_span;

pub fn process_batch(items: &[Item]) -> Result<Vec<Output>> {
    let _span = create_span("process_batch", &[
        ("item_count", &items.len().to_string())
    ]);
    
    // Your implementation
    // Span automatically records timing on drop
}
```

### Nested Spans

```rust
use embeddenator_obs::{create_span, create_debug_span};

pub fn complex_operation(data: &Data) -> Result<Output> {
    let _outer = create_span("complex_operation", &[]);
    
    {
        let _phase1 = create_debug_span("phase1_preprocessing", &[]);
        preprocess(data)?;
    }
    
    {
        let _phase2 = create_debug_span("phase2_computation", &[]);
        compute(data)?;
    }
    
    {
        let _phase3 = create_debug_span("phase3_postprocessing", &[]);
        postprocess(data)
    }
}
```

## Step 5: Add Test Metrics to Benchmarks

### Basic Performance Test

```rust
use embeddenator_obs::TestMetrics;

#[test]
fn benchmark_bind_operation() {
    let mut metrics = TestMetrics::new("bind");
    
    let a = create_test_vector(768);
    let b = create_test_vector(768);
    
    // Time 100 iterations
    for _ in 0..100 {
        metrics.start_timing();
        let _result = bind(&a, &b);
        metrics.stop_timing();
    }
    
    let stats = metrics.timing_stats();
    
    // Assert performance requirements
    assert!(stats.mean_ns < 10_000, "Bind too slow: {:.2}µs", stats.mean_ns / 1000.0);
    assert!(stats.p99_ns < 20_000, "P99 too high: {:.2}µs", stats.p99_ns as f64 / 1000.0);
    
    // Print summary
    println!("{}", metrics.summary());
}
```

### Advanced Benchmarking

```rust
use embeddenator_obs::TestMetrics;

#[test]
fn comprehensive_benchmark() {
    let mut metrics = TestMetrics::new("comprehensive_test");
    
    let iterations = 1000;
    let mut total_items = 0;
    
    for i in 0..iterations {
        let size = 100 + i;
        let data = generate_test_data(size);
        
        let result = metrics.time_operation(|| {
            process(&data)
        });
        
        total_items += result.len();
        metrics.inc_op("processed");
        
        if i % 100 == 0 {
            metrics.record_memory(estimate_memory_usage());
        }
    }
    
    // Record custom metrics
    metrics.record_metric("items_per_op", total_items as f64 / iterations as f64);
    metrics.record_metric("throughput_items_per_sec", 
        total_items as f64 / (metrics.timing_stats().total_ns as f64 / 1e9));
    
    // Verify and print
    println!("{}", metrics.summary());
    
    let stats = metrics.timing_stats();
    assert!(stats.ops_per_sec() >= 1000.0, "Throughput too low");
}
```

## Step 6: Telemetry for Production Monitoring

### Setup Telemetry Collector

```rust
use embeddenator_obs::{Telemetry, TelemetryConfig};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Create shared telemetry instance
lazy_static! {
    static ref TELEMETRY: Arc<Mutex<Telemetry>> = 
        Arc::new(Mutex::new(Telemetry::default_config()));
}

// Background export thread
fn start_telemetry_export() {
    let telemetry = TELEMETRY.clone();
    
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(60));
            
            let snapshot = telemetry.lock().unwrap().snapshot();
            
            // Export to monitoring system
            export_to_prometheus(&snapshot);
            // or
            send_to_cloudwatch(&snapshot.to_json());
        }
    });
}
```

### Record Operations

```rust
fn handle_request(req: Request) -> Response {
    let start = Instant::now();
    
    let response = process_request(req);
    
    TELEMETRY.lock().unwrap()
        .record_operation("handle_request", start.elapsed().as_micros() as u64);
    
    TELEMETRY.lock().unwrap()
        .increment_counter("requests_total");
    
    response
}
```

## Integration Patterns by Component

### embeddenator-vsa

```rust
// In lib.rs
pub use embeddenator_obs::{metrics, create_span};

// In operations
pub fn bind(a: &Vector, b: &Vector) -> Vector {
    let _span = create_span("vsa_bind", &[("dim", &a.dim().to_string())]);
    metrics().inc_sub_cache_miss();
    
    // Implementation
}

pub fn bundle(a: &Vector, b: &Vector) -> Vector {
    let _span = create_span("vsa_bundle", &[]);
    metrics().inc_sub_cache_hit();
    
    // Implementation
}
```

### embeddenator-retrieval

```rust
use embeddenator_obs::{metrics, create_span};
use std::time::Instant;

pub fn hierarchical_query(
    index: &HierarchicalIndex,
    query: &Vector,
    k: usize
) -> Vec<Result> {
    let _span = create_span("hier_query", &[("k", &k.to_string())]);
    
    let start = Instant::now();
    let results = index.query_with_hierarchy(query, k);
    metrics().record_hier_query(start.elapsed());
    
    results
}
```

### embeddenator-fs

```rust
use embeddenator_obs::{metrics, create_span, info};

pub fn scan_directory(path: &Path) -> Result<Vec<Entry>> {
    let _span = create_span("fs_scan", &[("path", &path.display().to_string())]);
    
    info(&format!("Scanning directory: {}", path.display()));
    
    let entries = read_dir_recursive(path)?;
    
    metrics().inc_index_cache_hit();
    
    Ok(entries)
}
```

## Testing Observability Integration

### Unit Test

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use embeddenator_obs::metrics;
    
    #[test]
    fn test_metrics_recorded() {
        let before = metrics().snapshot();
        
        // Call instrumented function
        let _ = my_function();
        
        let after = metrics().snapshot();
        
        // Verify metrics changed
        #[cfg(feature = "metrics")]
        assert!(after.sub_cache_hits > before.sub_cache_hits);
    }
}
```

### Integration Test

```rust
#[test]
fn test_full_observability_stack() {
    use embeddenator_obs::{init_tracing, metrics, TestMetrics};
    
    init_tracing();
    
    let mut test_metrics = TestMetrics::new("integration");
    
    test_metrics.time_operation(|| {
        // Perform operations
        my_function();
        
        // Verify metrics
        let snapshot = metrics().snapshot();
        assert!(snapshot.sub_cache_hits > 0 || snapshot.sub_cache_misses > 0);
    });
    
    println!("{}", test_metrics.summary());
}
```

## Troubleshooting

### Metrics Not Recording

```rust
// Check if metrics feature is enabled
#[cfg(feature = "metrics")]
println!("Metrics enabled");

#[cfg(not(feature = "metrics"))]
println!("Metrics disabled - no-op");
```

### No Tracing Output

```bash
# Enable tracing
export EMBEDDENATOR_LOG=debug
export RUST_LOG=trace

# Run your program
cargo run
```

### Performance Regression

```rust
// Add benchmark to catch regressions
#[test]
fn performance_baseline() {
    let mut metrics = TestMetrics::new("baseline");
    
    for _ in 0..1000 {
        metrics.time_operation(|| {
            // Critical path
        });
    }
    
    let stats = metrics.timing_stats();
    
    // Set baseline thresholds
    assert!(stats.p50_ns < 1000, "P50 regression");
    assert!(stats.p99_ns < 5000, "P99 regression");
}
```

## Best Practices

1. **Initialize Once**: Call `init_tracing()` once at startup
2. **Metrics in Production**: Use metrics for production monitoring
3. **Spans for Development**: Use spans for debugging and development
4. **TestMetrics for Benchmarks**: Use TestMetrics for performance testing
5. **Feature Gates**: Respect feature gates for zero-cost in release
6. **Avoid Allocations**: Metrics are lock-free and allocation-free
7. **Span Scope**: Let spans drop naturally for automatic timing
8. **Counter Increments**: Use for counting, not for timing
9. **Duration Recording**: Use for operation timing
10. **Telemetry Export**: Export periodically, not per-operation

## Performance Targets

| Operation | Target Overhead | Actual (Release) |
|-----------|----------------|------------------|
| Counter increment | <20ns | 4ns  |
| Span creation | <200ns | 6ns  |
| TestMetrics timing | <100ns | 24ns  |
| Hi-res measurement | <100ns | 20ns  |

## Additional Resources

- [README.md](README.md) - Full documentation
- [MIGRATION_SUMMARY.md](MIGRATION_SUMMARY.md) - Migration details
- [examples/performance_benchmark.rs](examples/performance_benchmark.rs) - Performance benchmarks
- [tests/integration_test.rs](tests/integration_test.rs) - Integration examples

## Support

For issues or questions, see the main Embeddenator repository.
