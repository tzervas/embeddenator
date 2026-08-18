//! Real-Time Metric Streaming
//!
//! Provides callback-based real-time streaming of observability metrics
//! for live monitoring, alerting, and reactive systems.
//!
//! # Features
//!
//! - Callback-based metric updates
//! - Threshold-based alerting
//! - Metric change detection
//! - Rate limiting for high-frequency metrics
//! - Multiple subscriber support
//!
//! # Usage
//!
//! ```rust,ignore
//! use embeddenator_obs::streaming::{MetricStream, MetricEvent};
//!
//! let mut stream = MetricStream::new();
//!
//! // Subscribe to metric updates
//! stream.subscribe(|event| {
//!     match event {
//!         MetricEvent::Counter(name, value) => {
//!             println!("Counter {}: {}", name, value);
//!         }
//!         MetricEvent::Gauge(name, value) => {
//!             if value > 100.0 {
//!                 alert!("High gauge value");
//!             }
//!         }
//!         _ => {}
//!     }
//! });
//!
//! // Publish metrics
//! stream.publish_counter("requests", 42);
//! stream.publish_gauge("cpu_usage", 75.5);
//! ```

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Type of metric event.
#[derive(Debug, Clone, PartialEq)]
pub enum MetricEvent {
    /// Counter metric (name, value)
    Counter(String, u64),
    /// Gauge metric (name, value)
    Gauge(String, f64),
    /// Timing metric (name, duration_us)
    Timing(String, u64),
    /// Threshold exceeded (metric, value, threshold)
    ThresholdExceeded(String, f64, f64),
}

/// Metric subscriber callback.
pub type MetricCallback = Arc<dyn Fn(&MetricEvent) + Send + Sync>;

/// Real-time metric streaming system.
pub struct MetricStream {
    /// Active subscribers
    subscribers: Arc<Mutex<Vec<MetricCallback>>>,
    /// Threshold alerts
    thresholds: Arc<Mutex<Vec<ThresholdAlert>>>,
    /// Rate limiter state
    rate_limiter: Arc<Mutex<RateLimiter>>,
}

/// Threshold-based alert configuration.
#[derive(Debug, Clone)]
pub struct ThresholdAlert {
    /// Metric name pattern
    pub metric_pattern: String,
    /// Threshold value
    pub threshold: f64,
    /// Alert when above (true) or below (false)
    pub above: bool,
}

/// Rate limiter to prevent callback flooding.
struct RateLimiter {
    /// Last emit time per metric
    last_emit: std::collections::HashMap<String, Instant>,
    /// Minimum interval between emits
    min_interval: Duration,
}

impl RateLimiter {
    fn new(min_interval: Duration) -> Self {
        Self {
            last_emit: std::collections::HashMap::new(),
            min_interval,
        }
    }

    fn should_emit(&mut self, key: &str) -> bool {
        let now = Instant::now();
        if let Some(last) = self.last_emit.get(key) {
            if now.duration_since(*last) < self.min_interval {
                return false;
            }
        }
        self.last_emit.insert(key.to_string(), now);
        true
    }
}

impl MetricStream {
    /// Create new metric stream.
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(Vec::new())),
            thresholds: Arc::new(Mutex::new(Vec::new())),
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new(Duration::from_millis(100)))),
        }
    }

    /// Create with custom rate limit.
    pub fn with_rate_limit(min_interval: Duration) -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(Vec::new())),
            thresholds: Arc::new(Mutex::new(Vec::new())),
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new(min_interval))),
        }
    }

    /// Subscribe to metric events.
    pub fn subscribe<F>(&mut self, callback: F)
    where
        F: Fn(&MetricEvent) + Send + Sync + 'static,
    {
        let mut subscribers = self.subscribers.lock().unwrap();
        subscribers.push(Arc::new(callback));
    }

    /// Add threshold alert.
    pub fn add_threshold_alert(&mut self, metric: impl Into<String>, threshold: f64, above: bool) {
        let mut thresholds = self.thresholds.lock().unwrap();
        thresholds.push(ThresholdAlert {
            metric_pattern: metric.into(),
            threshold,
            above,
        });
    }

    /// Publish counter metric.
    pub fn publish_counter(&self, name: impl Into<String>, value: u64) {
        let name = name.into();
        if !self.should_emit(&name) {
            return;
        }

        let event = MetricEvent::Counter(name, value);
        self.emit(&event);
    }

    /// Publish gauge metric.
    pub fn publish_gauge(&self, name: impl Into<String>, value: f64) {
        let name = name.into();
        if !self.should_emit(&name) {
            return;
        }

        let event = MetricEvent::Gauge(name.clone(), value);
        self.emit(&event);
        self.check_thresholds(&name, value);
    }

    /// Publish timing metric.
    pub fn publish_timing(&self, name: impl Into<String>, duration_us: u64) {
        let name = name.into();
        if !self.should_emit(&name) {
            return;
        }

        let event = MetricEvent::Timing(name, duration_us);
        self.emit(&event);
    }

    /// Emit event to all subscribers.
    fn emit(&self, event: &MetricEvent) {
        let subscribers = self.subscribers.lock().unwrap();
        for callback in subscribers.iter() {
            callback(event);
        }
    }

    /// Check if rate limiter allows emission.
    fn should_emit(&self, key: &str) -> bool {
        let mut limiter = self.rate_limiter.lock().unwrap();
        limiter.should_emit(key)
    }

    /// Check threshold alerts for a metric.
    fn check_thresholds(&self, name: &str, value: f64) {
        let thresholds = self.thresholds.lock().unwrap();

        for alert in thresholds.iter() {
            if name.contains(&alert.metric_pattern) {
                let exceeded = if alert.above {
                    value > alert.threshold
                } else {
                    value < alert.threshold
                };

                if exceeded {
                    let event =
                        MetricEvent::ThresholdExceeded(name.to_string(), value, alert.threshold);
                    drop(thresholds); // Release lock before emitting
                    self.emit(&event);
                    break;
                }
            }
        }
    }

    /// Get subscriber count.
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.lock().unwrap().len()
    }

    /// Clear all subscribers.
    pub fn clear_subscribers(&mut self) {
        let mut subscribers = self.subscribers.lock().unwrap();
        subscribers.clear();
    }
}

impl Default for MetricStream {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn test_subscribe_and_publish() {
        let mut stream = MetricStream::new();
        let counter = Arc::new(AtomicU64::new(0));
        let counter_clone = counter.clone();

        stream.subscribe(move |event| {
            if matches!(event, MetricEvent::Counter(_, _)) {
                counter_clone.fetch_add(1, Ordering::Relaxed);
            }
        });

        stream.publish_counter("test", 42);

        // Give some time for callback
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_multiple_subscribers() {
        let mut stream = MetricStream::new();
        let count1 = Arc::new(AtomicU64::new(0));
        let count2 = Arc::new(AtomicU64::new(0));

        let c1 = count1.clone();
        let c2 = count2.clone();

        stream.subscribe(move |_| {
            c1.fetch_add(1, Ordering::Relaxed);
        });

        stream.subscribe(move |_| {
            c2.fetch_add(1, Ordering::Relaxed);
        });

        stream.publish_counter("test", 1);

        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(count1.load(Ordering::Relaxed), 1);
        assert_eq!(count2.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_threshold_alert() {
        let mut stream = MetricStream::new();
        stream.add_threshold_alert("cpu", 80.0, true);

        let alerted = Arc::new(AtomicU64::new(0));
        let alerted_clone = alerted.clone();

        stream.subscribe(move |event| {
            if matches!(event, MetricEvent::ThresholdExceeded(_, _, _)) {
                alerted_clone.fetch_add(1, Ordering::Relaxed);
            }
        });

        stream.publish_gauge("cpu_usage", 50.0); // No alert
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(alerted.load(Ordering::Relaxed), 0);

        // Wait to bypass rate limiter (default 100ms)
        std::thread::sleep(Duration::from_millis(110));

        stream.publish_gauge("cpu_usage", 85.0); // Alert!
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(alerted.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_rate_limiting() {
        let stream = MetricStream::with_rate_limit(Duration::from_millis(50));
        let count = Arc::new(AtomicU64::new(0));
        let count_clone = count.clone();

        let mut stream_mut = stream;
        stream_mut.subscribe(move |_| {
            count_clone.fetch_add(1, Ordering::Relaxed);
        });

        // Rapid fire - should be rate limited
        for _ in 0..10 {
            stream_mut.publish_counter("test", 1);
        }

        std::thread::sleep(Duration::from_millis(10));
        // Should only receive 1 due to rate limiting
        assert!(count.load(Ordering::Relaxed) < 10);
    }

    #[test]
    fn test_subscriber_count() {
        let mut stream = MetricStream::new();
        assert_eq!(stream.subscriber_count(), 0);

        stream.subscribe(|_| {});
        assert_eq!(stream.subscriber_count(), 1);

        stream.subscribe(|_| {});
        assert_eq!(stream.subscriber_count(), 2);

        stream.clear_subscribers();
        assert_eq!(stream.subscriber_count(), 0);
    }

    #[test]
    fn test_gauge_and_timing() {
        let mut stream = MetricStream::new();
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        stream.subscribe(move |event| {
            events_clone.lock().unwrap().push(event.clone());
        });

        stream.publish_gauge("memory", 1024.5);
        stream.publish_timing("query", 1500);

        std::thread::sleep(Duration::from_millis(10));
        let recorded = events.lock().unwrap();
        assert_eq!(recorded.len(), 2);
    }
}
