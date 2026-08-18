//! Integration tests for observability components

use embeddenator_obs::{create_span, init_tracing, metrics, Telemetry, TestMetrics};
use std::time::Duration;

#[test]
fn test_metrics_tracking() {
    let before = metrics().snapshot();

    // Simulate cache operations
    metrics().inc_sub_cache_hit();
    metrics().inc_sub_cache_hit();
    metrics().inc_sub_cache_miss();

    let after = metrics().snapshot();

    #[cfg(feature = "metrics")]
    {
        assert!(after.sub_cache_hits > before.sub_cache_hits);
        assert!(after.sub_cache_misses > before.sub_cache_misses);
    }
}

#[test]
fn test_operation_timing() {
    let before = metrics().snapshot();

    metrics().record_retrieval_query(Duration::from_micros(1500));
    metrics().record_retrieval_query(Duration::from_micros(2000));

    let after = metrics().snapshot();

    #[cfg(feature = "metrics")]
    {
        assert!(after.retrieval_query_calls > before.retrieval_query_calls);
        assert!(after.retrieval_query_ns_total >= before.retrieval_query_ns_total);
        assert!(after.retrieval_query_ns_max >= 1_500_000); // microseconds to ns
    }
}

#[test]
fn test_test_metrics_workflow() {
    let mut metrics = TestMetrics::new("integration_test");

    // Simulate multiple operations
    for i in 0..10 {
        metrics.start_timing();
        std::thread::sleep(Duration::from_micros(100 + i * 10));
        metrics.stop_timing();

        metrics.inc_op("iterations");
    }

    metrics.record_metric("success_rate", 0.95);
    metrics.record_memory(1024 * 1024);

    let stats = metrics.timing_stats();
    assert_eq!(stats.count, 10);
    assert!(stats.min_ns > 0);
    assert!(stats.max_ns >= stats.min_ns);

    let summary = metrics.summary();
    assert!(summary.contains("integration_test"));
    assert!(summary.contains("Timing"));
}

#[test]
fn test_telemetry_collection() {
    let mut telemetry = Telemetry::default_config();

    // Record various operations
    telemetry.record_operation("query", 1500);
    telemetry.record_operation("query", 2000);
    telemetry.record_operation("insert", 500);

    telemetry.increment_counter("requests");
    telemetry.increment_counter("requests");
    telemetry.set_gauge("memory_mb", 256.0);

    let snapshot = telemetry.snapshot();

    // Verify query stats
    let query_stats = snapshot.operation_stats.get("query").unwrap();
    assert_eq!(query_stats.count, 2);
    assert_eq!(query_stats.min_us, 1500);
    assert_eq!(query_stats.max_us, 2000);

    // Verify counters
    assert_eq!(snapshot.counters.get("requests"), Some(&2));
    assert_eq!(snapshot.gauges.get("memory_mb"), Some(&256.0));
}

#[test]
fn test_tracing_init() {
    // Should not panic
    init_tracing();

    // Create spans (should work regardless of feature state)
    create_span("test_operation", &[("key", "value")]);
}

#[test]
fn test_combined_observability() {
    // Initialize
    init_tracing();

    let mut test_metrics = TestMetrics::new("combined_test");
    let mut telemetry = Telemetry::default_config();

    // Simulate workload
    for _ in 0..5 {
        create_span("operation", &[]);

        test_metrics.time_operation(|| {
            std::thread::sleep(Duration::from_micros(100));

            metrics().inc_sub_cache_hit();
            metrics().record_retrieval_query(Duration::from_micros(150));

            telemetry.record_operation("work", 150);
            telemetry.increment_counter("ops");
        });
    }

    // Verify all systems captured data
    let test_stats = test_metrics.timing_stats();
    assert_eq!(test_stats.count, 5);

    let tel_snapshot = telemetry.snapshot();
    assert_eq!(tel_snapshot.counters.get("ops"), Some(&5));

    let metrics_snapshot = metrics().snapshot();
    #[cfg(feature = "metrics")]
    {
        assert!(metrics_snapshot.retrieval_query_calls >= 5);
    }
}

#[test]
#[cfg(feature = "telemetry")]
fn test_telemetry_json_export() {
    let mut telemetry = Telemetry::default_config();

    telemetry.record_operation("test", 1000);
    telemetry.increment_counter("count");

    let snapshot = telemetry.snapshot();
    let json = snapshot.to_json();

    // Should produce valid JSON structure
    assert!(json.contains("timestamp_secs"));
    assert!(json.contains("operations"));
}

#[test]
fn test_error_tracking() {
    let mut metrics = TestMetrics::new("error_test");

    metrics.record_error();
    metrics.record_error();
    metrics.record_warning();

    assert_eq!(metrics.error_count, 2);
    assert_eq!(metrics.warning_count, 1);

    let summary = metrics.summary();
    assert!(summary.contains("errors=2"));
    assert!(summary.contains("warnings=1"));
}

#[test]
fn test_memory_tracking() {
    let mut metrics = TestMetrics::new("memory_test");

    // Simulate memory growth
    for i in 1..=5 {
        metrics.record_memory(i * 1024 * 1024); // 1MB, 2MB, ..., 5MB
    }

    let summary = metrics.summary();
    assert!(summary.contains("Memory"));
    assert!(summary.contains("peak="));
    assert!(summary.contains("avg="));
}

#[test]
fn test_percentile_accuracy() {
    let mut metrics = TestMetrics::new("percentile_test");

    // Create known distribution: 100ns to 1000ns in steps of 100ns
    for i in 1..=10 {
        metrics.timings_ns.push(i * 100);
    }

    let stats = metrics.timing_stats();

    assert_eq!(stats.count, 10);
    assert_eq!(stats.min_ns, 100);
    assert_eq!(stats.max_ns, 1000);
    // P50 is the value at index 5 (after sorting), which is 600ns
    assert_eq!(stats.p50_ns, 600);
    assert_eq!(stats.mean_ns, 550.0); // average
}

#[test]
fn test_throughput_calculation() {
    let mut metrics = TestMetrics::new("throughput_test");

    // 100 operations taking 1 second total
    for _ in 0..100 {
        metrics.timings_ns.push(10_000_000); // 10ms each
    }

    let stats = metrics.timing_stats();
    let ops_per_sec = stats.ops_per_sec();

    // Should be approximately 100 ops/sec
    assert!((99.0..=101.0).contains(&ops_per_sec));
}
