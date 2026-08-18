//! Example: Prometheus and OpenTelemetry Integration
//!
//! Demonstrates how to use the new observability features:
//! - Prometheus metrics export
//! - OpenTelemetry distributed tracing
//! - Advanced statistical analysis
//! - Real-time metric streaming

use embeddenator_obs::{MetricEvent, MetricStream, OtelSpan, PrometheusExporter, Telemetry};
use std::time::Duration;

fn main() {
    println!("=== Embeddenator Observability - Advanced Features Demo ===\n");

    // 1. Telemetry with advanced statistics
    demo_advanced_telemetry();

    // 2. Prometheus export
    demo_prometheus_export();

    // 3. OpenTelemetry distributed tracing
    demo_opentelemetry_tracing();

    // 4. Real-time metric streaming
    demo_metric_streaming();
}

fn demo_advanced_telemetry() {
    println!("--- 1. Advanced Telemetry Statistics ---");

    let mut telemetry = Telemetry::default_config();

    // Record various operation timings
    for i in 1..=100 {
        let duration = 100 + (i * 10); // Simulated latency
        telemetry.record_operation("api_call", duration);
    }

    let snapshot = telemetry.snapshot();
    if let Some(stats) = snapshot.operation_stats.get("api_call") {
        println!("Operation: api_call");
        println!("  Count: {}", stats.count);
        println!("  Average: {:.2}µs", stats.avg_us());
        println!("  Std Dev: {:.2}µs", stats.std_dev_us());
        println!("  Min: {}µs, Max: {}µs", stats.min_us, stats.max_us);
        println!("  P50 (median): {}µs", stats.median_us());
        println!("  P95: {}µs", stats.p95_us());
        println!("  P99: {}µs", stats.p99_us());
    }

    println!();
}

fn demo_prometheus_export() {
    println!("--- 2. Prometheus Metrics Export ---");

    let mut telemetry = Telemetry::default_config();
    telemetry.record_operation("query", 1250);
    telemetry.record_operation("query", 1500);
    telemetry.increment_counter("requests");
    telemetry.increment_counter("requests");
    telemetry.set_gauge("memory_mb", 512.5);

    let snapshot = telemetry.snapshot();
    let exporter = PrometheusExporter::new("embeddenator");
    let prometheus_text = exporter.export(&snapshot);

    println!("Prometheus Format Output:");
    println!("{}", prometheus_text);
    println!("(This can be scraped by Prometheus at /metrics endpoint)\n");
}

fn demo_opentelemetry_tracing() {
    println!("--- 3. OpenTelemetry Distributed Tracing ---");

    // Create root span
    let mut root_span = OtelSpan::new("http_request");
    root_span.set_attribute("http.method", "GET");
    root_span.set_attribute("http.url", "/api/search");

    // Simulate work
    std::thread::sleep(Duration::from_millis(10));

    // Create child span
    let mut db_span = OtelSpan::new_child("database_query", &root_span);
    db_span.set_attribute("db.type", "postgres");
    db_span.add_event("connection_acquired");

    // Simulate DB work
    std::thread::sleep(Duration::from_millis(5));

    db_span.end();
    root_span.end();

    // Export trace context (W3C format)
    let traceparent = root_span.to_traceparent();
    println!("W3C Trace Context:");
    println!("  traceparent: {}", traceparent);
    println!("  Trace ID: {:x}", root_span.trace_id);
    println!("  Root Span ID: {:x}", root_span.span_id);
    println!("  Root Duration: {}µs", root_span.duration_ns() / 1000);
    println!("  Child Duration: {}µs", db_span.duration_ns() / 1000);

    println!();
}

fn demo_metric_streaming() {
    println!("--- 4. Real-Time Metric Streaming ---");

    let mut stream = MetricStream::new();

    // Add threshold alert
    stream.add_threshold_alert("cpu", 80.0, true);
    stream.add_threshold_alert("memory", 90.0, true);

    // Subscribe to metric events
    stream.subscribe(|event| match event {
        MetricEvent::Counter(name, value) => {
            println!("  [COUNTER] {}: {}", name, value);
        }
        MetricEvent::Gauge(name, value) => {
            println!("  [GAUGE] {}: {:.2}", name, value);
        }
        MetricEvent::Timing(name, duration_us) => {
            println!("  [TIMING] {}: {}µs", name, duration_us);
        }
        MetricEvent::ThresholdExceeded(name, value, threshold) => {
            println!(
                "  [ALERT] {} = {:.2} exceeded threshold {:.2}",
                name, value, threshold
            );
        }
    });

    // Publish metrics
    stream.publish_counter("requests", 100);
    std::thread::sleep(Duration::from_millis(120)); // Wait for rate limiter

    stream.publish_gauge("cpu_usage", 75.5);
    std::thread::sleep(Duration::from_millis(120));

    stream.publish_gauge("cpu_usage", 85.0); // Will trigger alert
    std::thread::sleep(Duration::from_millis(120));

    stream.publish_timing("query", 1500);
    std::thread::sleep(Duration::from_millis(50));

    println!();
}
