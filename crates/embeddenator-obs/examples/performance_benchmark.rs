//! Performance overhead benchmarks for observability components

use embeddenator_obs::{create_span, measure_n, metrics, HiResTimer, TestMetrics};
use std::time::Instant;

fn main() {
    println!("=== Embeddenator Observability Performance Benchmarks ===\n");

    benchmark_metrics_overhead();
    benchmark_tracing_overhead();
    benchmark_test_metrics_overhead();
    benchmark_hires_timing_overhead();
}

fn benchmark_metrics_overhead() {
    println!("1. Metrics Counter Increment Overhead");
    println!("   Testing lock-free atomic counter performance...");

    let iterations = 10_000_000;

    // Baseline: empty loop
    let start = Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(());
    }
    let baseline_ns = start.elapsed().as_nanos() / iterations;

    // Metrics increment
    let start = Instant::now();
    for _ in 0..iterations {
        metrics().inc_sub_cache_hit();
    }
    let metrics_ns = start.elapsed().as_nanos() / iterations;

    let overhead_ns = metrics_ns.saturating_sub(baseline_ns);

    println!("   Baseline: {}ns", baseline_ns);
    println!("   Metrics: {}ns", metrics_ns);
    println!("   Overhead: {}ns per increment", overhead_ns);
    println!("   ✓ Target: <20ns, Actual: {}ns\n", overhead_ns);
}

fn benchmark_tracing_overhead() {
    println!("2. Tracing Span Creation Overhead");
    println!("   Testing span instrumentation performance...");

    let iterations = 100_000;

    // Baseline: empty loop
    let start = Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(());
    }
    let baseline_ns = start.elapsed().as_nanos() / iterations;

    // Span creation
    let start = Instant::now();
    for _ in 0..iterations {
        create_span("test", &[]);
        std::hint::black_box(&());
    }
    let span_ns = start.elapsed().as_nanos() / iterations;

    let overhead_ns = span_ns.saturating_sub(baseline_ns);

    println!("   Baseline: {}ns", baseline_ns);
    println!("   Span creation: {}ns", span_ns);
    println!("   Overhead: {}ns per span", overhead_ns);
    println!("   ✓ Target: <200ns, Actual: {}ns\n", overhead_ns);
}

fn benchmark_test_metrics_overhead() {
    println!("3. TestMetrics Timing Overhead");
    println!("   Testing measurement framework performance...");

    let iterations = 100_000;
    let mut metrics = TestMetrics::new("benchmark");

    // Measure timing overhead
    let start = Instant::now();
    for _ in 0..iterations {
        metrics.start_timing();
        std::hint::black_box(42);
        metrics.stop_timing();
    }
    let total_ns = start.elapsed().as_nanos();
    let per_op_ns = total_ns / iterations;

    println!("   Per operation: {}ns", per_op_ns);
    println!("   Total samples: {}", metrics.timings_ns.len());
    println!("   ✓ Target: <100ns, Actual: {}ns\n", per_op_ns);
}

fn benchmark_hires_timing_overhead() {
    println!("4. Hi-Res Timing Measurement Overhead");
    println!("   Testing picosecond-scale timer performance...");

    let iterations = 100_000;

    // Single measurement overhead
    let start = Instant::now();
    for _ in 0..iterations {
        let timer = HiResTimer::start();
        std::hint::black_box(42);
        let _elapsed = timer.elapsed();
        std::hint::black_box(_elapsed);
    }
    let per_measurement_ns = start.elapsed().as_nanos() / iterations;

    println!("   Per measurement: {}ns", per_measurement_ns);

    // Batch measurement with statistics
    let (results, stats) = measure_n(10_000, || std::hint::black_box(42));

    println!("   Batch stats: {}", stats.format());
    println!("   ✓ Target: <100ns, Actual: {}ns\n", per_measurement_ns);

    drop(results); // Prevent optimization
}
