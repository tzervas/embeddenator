//! Common test utilities with high-precision timing
//!
//! Provides nanosecond-precision timing instrumentation for tests.

#![allow(dead_code)] // Utility functions may not be used by all tests

use std::time::{Duration, Instant};

/// High-precision timer for test measurements
pub struct PrecisionTimer {
    start: Instant,
    name: String,
    laps: Vec<(String, Duration)>,
}

impl PrecisionTimer {
    /// Create a new timer with a name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            name: name.into(),
            laps: Vec::new(),
        }
    }

    /// Record a lap time with a label
    pub fn lap(&mut self, label: impl Into<String>) {
        self.laps.push((label.into(), self.start.elapsed()));
    }

    /// Get elapsed time as nanoseconds
    pub fn elapsed_nanos(&self) -> u128 {
        self.start.elapsed().as_nanos()
    }

    /// Get elapsed time as picoseconds (estimated from nanoseconds)
    /// Note: Rust's Instant doesn't provide true picosecond resolution,
    /// but we extrapolate for display purposes
    pub fn elapsed_picos(&self) -> u128 {
        self.start.elapsed().as_nanos() * 1000
    }

    /// Format duration with appropriate precision
    pub fn format_duration(d: Duration) -> String {
        let nanos = d.as_nanos();
        if nanos < 1_000 {
            format!("{} ns", nanos)
        } else if nanos < 1_000_000 {
            format!("{:.3} μs ({} ns)", nanos as f64 / 1_000.0, nanos)
        } else if nanos < 1_000_000_000 {
            format!("{:.3} ms ({} ns)", nanos as f64 / 1_000_000.0, nanos)
        } else {
            format!("{:.3} s ({} ns)", nanos as f64 / 1_000_000_000.0, nanos)
        }
    }

    /// Print timing report
    pub fn report(&self) {
        let total = self.start.elapsed();
        println!("\n╔══════════════════════════════════════════════════════════════════╗");
        println!("║ TIMING REPORT: {:<50} ║", self.name);
        println!("╠══════════════════════════════════════════════════════════════════╣");

        if !self.laps.is_empty() {
            let mut prev_time = Duration::ZERO;
            for (label, cumulative) in &self.laps {
                let delta = *cumulative - prev_time;
                println!(
                    "║ {:30} │ {:>12} (Δ {:>12}) ║",
                    label,
                    Self::format_duration(*cumulative),
                    Self::format_duration(delta)
                );
                prev_time = *cumulative;
            }
            println!("╠══════════════════════════════════════════════════════════════════╣");
        }

        println!("║ TOTAL: {:>58} ║", Self::format_duration(total));
        println!(
            "║ (= {} nanoseconds = {} picoseconds estimated) ║",
            total.as_nanos(),
            total.as_nanos() * 1000
        );
        println!("╚══════════════════════════════════════════════════════════════════╝\n");
    }
}

impl Drop for PrecisionTimer {
    fn drop(&mut self) {
        // Auto-report on drop if environment variable is set
        if std::env::var("TIMING_REPORT").is_ok() {
            self.report();
        }
    }
}

/// Measure a closure with nanosecond precision
pub fn time_ns<F, R>(label: &str, f: F) -> (R, u128)
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed().as_nanos();
    println!("⏱ {}: {} ns", label, elapsed);
    (result, elapsed)
}

/// Measure a closure with formatted output
pub fn time_formatted<F, R>(label: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed();
    println!(
        "⏱ {}: {} ({} ns)",
        label,
        PrecisionTimer::format_duration(elapsed),
        elapsed.as_nanos()
    );
    result
}

/// Statistics from multiple timing runs
#[derive(Debug, Clone)]
pub struct TimingStats {
    pub name: String,
    pub samples: Vec<u128>,
}

impl TimingStats {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            samples: Vec::new(),
        }
    }

    pub fn add_sample(&mut self, nanos: u128) {
        self.samples.push(nanos);
    }

    pub fn min(&self) -> u128 {
        *self.samples.iter().min().unwrap_or(&0)
    }

    pub fn max(&self) -> u128 {
        *self.samples.iter().max().unwrap_or(&0)
    }

    pub fn mean(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.samples.iter().sum::<u128>() as f64 / self.samples.len() as f64
    }

    pub fn median(&self) -> u128 {
        if self.samples.is_empty() {
            return 0;
        }
        let mut sorted = self.samples.clone();
        sorted.sort();
        sorted[sorted.len() / 2]
    }

    pub fn std_dev(&self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }
        let mean = self.mean();
        let variance: f64 = self
            .samples
            .iter()
            .map(|&x| {
                let diff = x as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / (self.samples.len() - 1) as f64;
        variance.sqrt()
    }

    pub fn report(&self) {
        let d = |ns: u128| PrecisionTimer::format_duration(Duration::from_nanos(ns as u64));
        println!("\n┌─────────────────────────────────────────────────────────────────┐");
        println!("│ TIMING STATS: {:<50} │", self.name);
        println!("├─────────────────────────────────────────────────────────────────┤");
        println!("│ Samples: {:<56} │", self.samples.len());
        println!("│ Min:     {:<56} │", d(self.min()));
        println!("│ Max:     {:<56} │", d(self.max()));
        println!("│ Mean:    {:<56} │", format!("{:.2} ns", self.mean()));
        println!("│ Median:  {:<56} │", d(self.median()));
        println!("│ StdDev:  {:<56} │", format!("{:.2} ns", self.std_dev()));
        println!("└─────────────────────────────────────────────────────────────────┘\n");
    }
}

/// Run a function multiple times and collect timing statistics
pub fn benchmark<F, R>(name: &str, iterations: usize, mut f: F) -> TimingStats
where
    F: FnMut() -> R,
{
    let mut stats = TimingStats::new(name);

    // Warmup
    for _ in 0..3 {
        let _ = f();
    }

    // Actual measurements
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = f();
        stats.add_sample(start.elapsed().as_nanos());
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precision_timer() {
        let mut timer = PrecisionTimer::new("test_timer");
        std::thread::sleep(Duration::from_micros(100));
        timer.lap("after 100μs");
        std::thread::sleep(Duration::from_micros(50));
        timer.lap("after 150μs total");

        // Verify elapsed time is reasonable (at least 100μs = 100_000ns)
        assert!(timer.elapsed_nanos() >= 100_000);
        timer.report();
    }

    #[test]
    fn test_timing_stats() {
        let mut stats = TimingStats::new("test_stats");
        for i in [100, 200, 150, 300, 250] {
            stats.add_sample(i);
        }

        assert_eq!(stats.min(), 100);
        assert_eq!(stats.max(), 300);
        assert_eq!(stats.median(), 200);
        assert!((stats.mean() - 200.0).abs() < 0.01);
        stats.report();
    }
}
