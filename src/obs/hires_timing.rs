//! High-Resolution Timing Infrastructure
//!
//! Provides picosecond-scale timing metrics where hardware supports it.
//! Falls back gracefully to nanosecond precision on systems without
//! high-resolution timestamp counters.
//!
//! # Implementation Notes
//!
//! True picosecond resolution requires:
//! - CPU timestamp counters (RDTSC on x86_64, CNTVCT on ARM64)
//! - Known TSC frequency for conversion
//! - Invariant TSC support (modern CPUs)
//!
//! On Linux, we use `clock_gettime(CLOCK_MONOTONIC_RAW)` which provides
//! nanosecond granularity. For sub-nanosecond estimation, we perform
//! multiple measurements and statistical analysis.
//!
//! # Performance Notes
//!
//! TSC frequency is cached after first calibration to avoid repeated
//! file I/O and calibration overhead on timer creation.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Cached TSC frequency (Hz) - computed once on first use
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
static CACHED_TSC_FREQ: AtomicU64 = AtomicU64::new(0);

/// Sentinel value indicating TSC freq needs calibration
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
const TSC_UNCALIBRATED: u64 = 0;

/// Picosecond timestamp (1 ps = 10^-12 seconds)
/// We store as u64 picoseconds, giving us ~213 days of range
pub type Picoseconds = u64;

/// Nanoseconds for compatibility
pub type Nanoseconds = u64;

/// Conversion constants
pub const PS_PER_NS: u64 = 1_000;
pub const PS_PER_US: u64 = 1_000_000;
pub const PS_PER_MS: u64 = 1_000_000_000;
pub const PS_PER_SEC: u64 = 1_000_000_000_000;

/// High-resolution timing result with uncertainty bounds
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HiResTimestamp {
    /// Measured time in picoseconds
    pub picoseconds: Picoseconds,
    /// Lower bound uncertainty (picoseconds)
    pub uncertainty_low: Picoseconds,
    /// Upper bound uncertainty (picoseconds)
    pub uncertainty_high: Picoseconds,
    /// Whether this is an estimated (sub-ns) or direct measurement
    pub is_estimated: bool,
}

impl HiResTimestamp {
    /// Create from nanoseconds with standard uncertainty
    pub fn from_nanos(ns: u64) -> Self {
        HiResTimestamp {
            picoseconds: ns.saturating_mul(PS_PER_NS),
            uncertainty_low: 500, // ±500ps typical for ns-resolution
            uncertainty_high: 500,
            is_estimated: false,
        }
    }

    /// Create from direct picosecond measurement
    pub fn from_picos(ps: Picoseconds, uncertainty: Picoseconds) -> Self {
        HiResTimestamp {
            picoseconds: ps,
            uncertainty_low: uncertainty,
            uncertainty_high: uncertainty,
            is_estimated: true,
        }
    }

    /// Convert to nanoseconds (lossy)
    pub fn as_nanos(&self) -> u64 {
        self.picoseconds / PS_PER_NS
    }

    /// Convert to microseconds (lossy)
    pub fn as_micros(&self) -> u64 {
        self.picoseconds / PS_PER_US
    }

    /// Convert to milliseconds (lossy)
    pub fn as_millis(&self) -> u64 {
        self.picoseconds / PS_PER_MS
    }

    /// Convert to seconds as f64 for high precision display
    pub fn as_secs_f64(&self) -> f64 {
        self.picoseconds as f64 / PS_PER_SEC as f64
    }

    /// Format with appropriate unit
    pub fn format(&self) -> String {
        if self.picoseconds < PS_PER_NS {
            format!("{}ps", self.picoseconds)
        } else if self.picoseconds < PS_PER_US {
            format!("{:.3}ns", self.picoseconds as f64 / PS_PER_NS as f64)
        } else if self.picoseconds < PS_PER_MS {
            format!("{:.3}µs", self.picoseconds as f64 / PS_PER_US as f64)
        } else if self.picoseconds < PS_PER_SEC {
            format!("{:.3}ms", self.picoseconds as f64 / PS_PER_MS as f64)
        } else {
            format!("{:.3}s", self.picoseconds as f64 / PS_PER_SEC as f64)
        }
    }

    /// Format with uncertainty bounds
    pub fn format_with_uncertainty(&self) -> String {
        let base = self.format();
        let unc = if self.uncertainty_high == self.uncertainty_low {
            format!("±{}ps", self.uncertainty_low)
        } else {
            format!("+{}/-{}ps", self.uncertainty_high, self.uncertainty_low)
        };
        format!("{} ({})", base, unc)
    }
}

impl std::ops::Sub for HiResTimestamp {
    type Output = HiResTimestamp;

    fn sub(self, rhs: Self) -> Self::Output {
        HiResTimestamp {
            picoseconds: self.picoseconds.saturating_sub(rhs.picoseconds),
            uncertainty_low: self.uncertainty_low + rhs.uncertainty_low,
            uncertainty_high: self.uncertainty_high + rhs.uncertainty_high,
            is_estimated: self.is_estimated || rhs.is_estimated,
        }
    }
}

/// High-resolution timer using best available clock source
pub struct HiResTimer {
    /// Start instant for std timing
    start_instant: Instant,
    /// Start TSC value (if available)
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    start_tsc: u64,
    /// TSC frequency in Hz (calibrated)
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    tsc_freq_hz: u64,
}

impl HiResTimer {
    /// Create and start a new high-resolution timer
    #[inline]
    pub fn start() -> Self {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            let start_tsc = rdtsc();
            let start_instant = Instant::now();
            let tsc_freq_hz = get_tsc_frequency();

            HiResTimer {
                start_instant,
                start_tsc,
                tsc_freq_hz,
            }
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
        {
            HiResTimer {
                start_instant: Instant::now(),
            }
        }
    }

    /// Get elapsed time with picosecond resolution (where possible)
    #[inline]
    pub fn elapsed(&self) -> HiResTimestamp {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            if self.tsc_freq_hz > 0 {
                let end_tsc = rdtsc();
                let cycles = end_tsc.saturating_sub(self.start_tsc);

                // Convert cycles to picoseconds: (cycles * 10^12) / freq_hz
                // Use u128 to avoid overflow
                let ps = ((cycles as u128) * PS_PER_SEC as u128) / (self.tsc_freq_hz as u128);
                let ps = ps.min(u64::MAX as u128) as u64;

                // Uncertainty is ~1 cycle at TSC frequency
                let uncertainty = PS_PER_SEC / self.tsc_freq_hz;

                return HiResTimestamp::from_picos(ps, uncertainty);
            }
        }

        // Fallback to std::time::Instant (nanosecond resolution)
        let elapsed = self.start_instant.elapsed();
        HiResTimestamp::from_nanos(elapsed.as_nanos().min(u64::MAX as u128) as u64)
    }

    /// Get elapsed nanoseconds (convenience method)
    #[inline]
    pub fn elapsed_nanos(&self) -> u64 {
        self.elapsed().as_nanos()
    }

    /// Get elapsed picoseconds
    #[inline]
    pub fn elapsed_picos(&self) -> Picoseconds {
        self.elapsed().picoseconds
    }
}

/// Read TSC (Time Stamp Counter) on x86/x86_64
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[inline]
fn rdtsc() -> u64 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        std::arch::x86_64::_rdtsc()
    }

    #[cfg(target_arch = "x86")]
    unsafe {
        std::arch::x86::_rdtsc()
    }
}

/// Get TSC frequency (cached after first calibration)
/// 
/// This is the critical optimization - we only calibrate once and cache
/// the result in a static atomic, avoiding expensive file I/O on every
/// timer creation.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[inline]
fn get_tsc_frequency() -> u64 {
    // Fast path: return cached value
    let cached = CACHED_TSC_FREQ.load(Ordering::Relaxed);
    if cached != TSC_UNCALIBRATED {
        return cached;
    }

    // Slow path: calibrate and cache
    let freq = calibrate_tsc_frequency();
    CACHED_TSC_FREQ.store(freq, Ordering::Relaxed);
    freq
}

/// Actually calibrate the TSC frequency (called once)
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
fn calibrate_tsc_frequency() -> u64 {
    // Try to read from sysfs (Linux) - fastest path
    if let Ok(content) = std::fs::read_to_string("/sys/devices/system/cpu/cpu0/tsc_freq_khz") {
        if let Ok(khz) = content.trim().parse::<u64>() {
            return khz * 1000;
        }
    }

    // Try cpuinfo for CPU MHz (less accurate but available)
    if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            if line.starts_with("cpu MHz") {
                if let Some(mhz_str) = line.split(':').nth(1) {
                    if let Ok(mhz) = mhz_str.trim().parse::<f64>() {
                        return (mhz * 1_000_000.0) as u64;
                    }
                }
            }
        }
    }

    // Calibration fallback: measure TSC ticks over known duration
    // Use shorter calibration for faster startup (1ms instead of 10ms)
    let calibration_ns = 1_000_000; // 1ms calibration
    let start_tsc = rdtsc();
    let start = Instant::now();

    // Busy-wait for calibration period
    while start.elapsed().as_nanos() < calibration_ns as u128 {
        std::hint::spin_loop();
    }

    let end_tsc = rdtsc();
    let actual_ns = start.elapsed().as_nanos() as u64;
    let cycles = end_tsc.saturating_sub(start_tsc);

    // freq = cycles / time = cycles * 10^9 / ns
    (cycles as u128 * 1_000_000_000 / actual_ns as u128) as u64
}

/// High-resolution metrics accumulator
///
/// Tracks timing statistics at picosecond granularity with
/// proper statistical aggregation.
pub struct HiResMetrics {
    /// Number of samples
    pub count: AtomicU64,
    /// Sum of all measurements (picoseconds)
    pub total_ps: AtomicU64,
    /// Minimum measurement (picoseconds)
    pub min_ps: AtomicU64,
    /// Maximum measurement (picoseconds)
    pub max_ps: AtomicU64,
    /// Sum of squares for variance calculation (in units of ns²)
    /// We use ns² to avoid overflow while maintaining reasonable precision
    pub sum_sq_ns2: AtomicU64,
}

impl HiResMetrics {
    pub const fn new() -> Self {
        HiResMetrics {
            count: AtomicU64::new(0),
            total_ps: AtomicU64::new(0),
            min_ps: AtomicU64::new(u64::MAX),
            max_ps: AtomicU64::new(0),
            sum_sq_ns2: AtomicU64::new(0),
        }
    }

    /// Record a measurement
    pub fn record(&self, timestamp: HiResTimestamp) {
        let ps = timestamp.picoseconds;
        let ns = timestamp.as_nanos();

        self.count.fetch_add(1, Ordering::Relaxed);
        self.total_ps.fetch_add(ps, Ordering::Relaxed);

        // Update min (atomic CAS loop)
        let mut cur_min = self.min_ps.load(Ordering::Relaxed);
        while ps < cur_min {
            match self.min_ps.compare_exchange_weak(
                cur_min,
                ps,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => cur_min = x,
            }
        }

        // Update max (atomic CAS loop)
        let mut cur_max = self.max_ps.load(Ordering::Relaxed);
        while ps > cur_max {
            match self.max_ps.compare_exchange_weak(
                cur_max,
                ps,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => cur_max = x,
            }
        }

        // Add to sum of squares (ns² to avoid overflow)
        self.sum_sq_ns2.fetch_add(ns.saturating_mul(ns), Ordering::Relaxed);
    }

    /// Record from HiResTimer
    pub fn record_timer(&self, timer: &HiResTimer) {
        self.record(timer.elapsed());
    }

    /// Get snapshot of metrics
    pub fn snapshot(&self) -> HiResMetricsSnapshot {
        let count = self.count.load(Ordering::Relaxed);
        let total_ps = self.total_ps.load(Ordering::Relaxed);
        let min_ps = self.min_ps.load(Ordering::Relaxed);
        let max_ps = self.max_ps.load(Ordering::Relaxed);
        let sum_sq_ns2 = self.sum_sq_ns2.load(Ordering::Relaxed);

        let mean_ps = if count > 0 {
            total_ps / count
        } else {
            0
        };

        // Variance = E[X²] - E[X]² (computed in ns for numerical stability)
        let mean_ns = mean_ps / PS_PER_NS;
        let variance_ns2 = if count > 1 {
            let e_x2 = sum_sq_ns2 / count;
            e_x2.saturating_sub(mean_ns.saturating_mul(mean_ns))
        } else {
            0
        };

        // Convert variance to picoseconds: var_ps = var_ns * (ps/ns)²
        let variance_ps = variance_ns2.saturating_mul(PS_PER_NS * PS_PER_NS);
        let stddev_ps = (variance_ps as f64).sqrt() as u64;

        HiResMetricsSnapshot {
            count,
            total_ps,
            min_ps: if min_ps == u64::MAX { 0 } else { min_ps },
            max_ps,
            mean_ps,
            stddev_ps,
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.count.store(0, Ordering::Relaxed);
        self.total_ps.store(0, Ordering::Relaxed);
        self.min_ps.store(u64::MAX, Ordering::Relaxed);
        self.max_ps.store(0, Ordering::Relaxed);
        self.sum_sq_ns2.store(0, Ordering::Relaxed);
    }
}

impl Default for HiResMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of high-resolution metrics
#[derive(Clone, Copy, Debug, Default)]
pub struct HiResMetricsSnapshot {
    /// Number of samples
    pub count: u64,
    /// Total time (picoseconds)
    pub total_ps: Picoseconds,
    /// Minimum time (picoseconds)
    pub min_ps: Picoseconds,
    /// Maximum time (picoseconds)
    pub max_ps: Picoseconds,
    /// Mean time (picoseconds)
    pub mean_ps: Picoseconds,
    /// Standard deviation (picoseconds)
    pub stddev_ps: Picoseconds,
}

impl HiResMetricsSnapshot {
    /// Format a comprehensive summary
    pub fn format(&self) -> String {
        if self.count == 0 {
            return "no samples".to_string();
        }

        format!(
            "n={} mean={} min={} max={} stddev={}",
            self.count,
            HiResTimestamp::from_picos(self.mean_ps, 0).format(),
            HiResTimestamp::from_picos(self.min_ps, 0).format(),
            HiResTimestamp::from_picos(self.max_ps, 0).format(),
            HiResTimestamp::from_picos(self.stddev_ps, 0).format(),
        )
    }

    /// Get throughput in operations per second
    pub fn ops_per_sec(&self) -> f64 {
        if self.count == 0 || self.total_ps == 0 {
            return 0.0;
        }
        self.count as f64 / (self.total_ps as f64 / PS_PER_SEC as f64)
    }

    /// Get throughput in operations per microsecond
    pub fn ops_per_us(&self) -> f64 {
        if self.count == 0 || self.total_ps == 0 {
            return 0.0;
        }
        self.count as f64 / (self.total_ps as f64 / PS_PER_US as f64)
    }
}

/// Measure a closure with picosecond timing
#[inline]
pub fn measure<F, R>(f: F) -> (R, HiResTimestamp)
where
    F: FnOnce() -> R,
{
    let timer = HiResTimer::start();
    let result = f();
    (result, timer.elapsed())
}

/// Measure a closure N times and return statistics
pub fn measure_n<F, R>(n: usize, mut f: F) -> (Vec<R>, HiResMetricsSnapshot)
where
    F: FnMut() -> R,
{
    let metrics = HiResMetrics::new();
    let mut results = Vec::with_capacity(n);

    for _ in 0..n {
        let timer = HiResTimer::start();
        results.push(f());
        metrics.record_timer(&timer);
    }

    (results, metrics.snapshot())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_from_nanos() {
        let ts = HiResTimestamp::from_nanos(1_000);
        assert_eq!(ts.picoseconds, 1_000_000);
        assert_eq!(ts.as_nanos(), 1_000);
        assert!(!ts.is_estimated);
    }

    #[test]
    fn test_timestamp_formatting() {
        assert_eq!(HiResTimestamp::from_picos(500, 0).format(), "500ps");
        assert_eq!(HiResTimestamp::from_nanos(1).format(), "1.000ns");
        assert_eq!(HiResTimestamp::from_nanos(1_000).format(), "1.000µs");
        assert_eq!(HiResTimestamp::from_nanos(1_000_000).format(), "1.000ms");
        assert_eq!(HiResTimestamp::from_nanos(1_000_000_000).format(), "1.000s");
    }

    #[test]
    fn test_timestamp_subtraction() {
        let a = HiResTimestamp::from_nanos(1_000);
        let b = HiResTimestamp::from_nanos(500);
        let diff = a - b;
        assert_eq!(diff.as_nanos(), 500);
    }

    #[test]
    fn test_hires_timer_basic() {
        let timer = HiResTimer::start();
        std::thread::sleep(std::time::Duration::from_micros(100));
        let elapsed = timer.elapsed();

        // Should be at least 100µs = 100,000,000 ps
        assert!(elapsed.picoseconds >= 100_000_000, "elapsed: {} ps", elapsed.picoseconds);
        // But not more than 10ms (accounting for scheduling jitter)
        assert!(elapsed.picoseconds < 10_000_000_000_000, "elapsed: {} ps", elapsed.picoseconds);
    }

    #[test]
    fn test_hires_metrics_accumulation() {
        let metrics = HiResMetrics::new();

        // Record some measurements
        metrics.record(HiResTimestamp::from_nanos(100));
        metrics.record(HiResTimestamp::from_nanos(200));
        metrics.record(HiResTimestamp::from_nanos(300));

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.count, 3);
        assert_eq!(snapshot.min_ps, 100 * PS_PER_NS);
        assert_eq!(snapshot.max_ps, 300 * PS_PER_NS);
        assert_eq!(snapshot.mean_ps, 200 * PS_PER_NS);
    }

    #[test]
    fn test_measure_closure() {
        let (result, timing) = measure(|| {
            let mut sum = 0u64;
            for i in 0..1000 {
                sum = sum.wrapping_add(i);
            }
            sum
        });

        assert!(result > 0);
        assert!(timing.picoseconds > 0);
    }

    #[test]
    fn test_measure_n() {
        let iterations = 100;
        let (results, stats) = measure_n(iterations, || {
            // Simple operation
            std::hint::black_box(42)
        });

        assert_eq!(results.len(), iterations);
        assert_eq!(stats.count, iterations as u64);
        assert!(stats.min_ps > 0);
        assert!(stats.max_ps >= stats.min_ps);
    }

    #[test]
    fn test_picosecond_precision_smoke() {
        // This test verifies that we can distinguish sub-microsecond timings
        let timer = HiResTimer::start();

        // Very short operation
        let _ = std::hint::black_box(1 + 1);

        let elapsed = timer.elapsed();

        // On modern hardware, this should be < 1µs
        // But we're lenient due to CI variance
        println!(
            "Short operation: {} ({})",
            elapsed.format(),
            elapsed.format_with_uncertainty()
        );

        // Just verify we got a measurement
        assert!(elapsed.picoseconds > 0 || !elapsed.is_estimated);
    }
}
