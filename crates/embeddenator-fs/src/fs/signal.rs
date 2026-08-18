//! Signal handling for graceful FUSE unmount
//!
//! This module provides cross-platform signal handling for FUSE filesystems,
//! enabling clean shutdown when receiving SIGINT (Ctrl+C), SIGTERM, or SIGHUP.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Signal Sources                           │
//! │  SIGINT (Ctrl+C)  │  SIGTERM (kill)  │  SIGHUP (terminal)  │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   signal-hook crate                         │
//! │              (cross-platform signal handling)               │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    ShutdownSignal                           │
//! │           (AtomicBool + optional callback)                  │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  FUSE mount loop                            │
//! │  1. Check shutdown flag periodically                        │
//! │  2. Flush WAL/pending writes                                │
//! │  3. Clean unmount via fuser::Session::unmount()             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use embeddenator_fs::fs::signal::{ShutdownSignal, install_signal_handlers};
//! use std::sync::Arc;
//!
//! // Create shutdown signal
//! let shutdown = Arc::new(ShutdownSignal::new());
//!
//! // Install handlers for SIGINT, SIGTERM, SIGHUP
//! install_signal_handlers(shutdown.clone()).expect("Failed to install signal handlers");
//!
//! // In your mount loop, check for shutdown
//! loop {
//!     if shutdown.is_signaled() {
//!         println!("Shutdown requested, cleaning up...");
//!         break;
//!     }
//!     // ... do work ...
//! }
//! ```

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;

use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};

/// Shutdown signal state for coordinating graceful unmount
///
/// This struct provides thread-safe coordination between signal handlers
/// and the FUSE filesystem operations.
#[derive(Debug)]
pub struct ShutdownSignal {
    /// Whether shutdown has been requested
    signaled: AtomicBool,
    /// The signal number that triggered shutdown (0 if not triggered)
    signal_num: AtomicI32,
}

impl ShutdownSignal {
    /// Create a new shutdown signal in the non-signaled state
    pub fn new() -> Self {
        Self {
            signaled: AtomicBool::new(false),
            signal_num: AtomicI32::new(0),
        }
    }

    /// Check if shutdown has been signaled
    #[inline]
    pub fn is_signaled(&self) -> bool {
        self.signaled.load(Ordering::Acquire)
    }

    /// Get the signal number that triggered shutdown (0 if none)
    pub fn signal_number(&self) -> i32 {
        self.signal_num.load(Ordering::Acquire)
    }

    /// Manually trigger shutdown (useful for testing or programmatic unmount)
    pub fn trigger(&self, signal_num: i32) {
        self.signal_num.store(signal_num, Ordering::Release);
        self.signaled.store(true, Ordering::Release);
    }

    /// Reset the signal (useful for testing)
    #[cfg(test)]
    pub fn reset(&self) {
        self.signaled.store(false, Ordering::Release);
        self.signal_num.store(0, Ordering::Release);
    }

    /// Get a human-readable name for the signal
    pub fn signal_name(&self) -> &'static str {
        match self.signal_num.load(Ordering::Acquire) {
            SIGINT => "SIGINT",
            SIGTERM => "SIGTERM",
            SIGHUP => "SIGHUP",
            0 => "none",
            _ => "unknown",
        }
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

/// Install signal handlers for graceful FUSE unmount
///
/// This function registers handlers for:
/// - `SIGINT` (Ctrl+C): Interactive interrupt
/// - `SIGTERM`: Standard termination signal (e.g., from `kill`)
/// - `SIGHUP`: Hangup signal (terminal closed)
///
/// When any of these signals is received, the shutdown signal will be set
/// and the signal number recorded.
///
/// # Arguments
///
/// * `shutdown` - Arc to the shutdown signal that will be triggered
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if signal handler installation fails.
///
/// # Platform Support
///
/// This function works on Unix-like systems. On Windows, only SIGINT is supported.
///
/// # Example
///
/// ```rust,no_run
/// use embeddenator_fs::fs::signal::{ShutdownSignal, install_signal_handlers};
/// use std::sync::Arc;
///
/// let shutdown = Arc::new(ShutdownSignal::new());
/// install_signal_handlers(shutdown.clone()).expect("Signal handler setup failed");
/// ```
pub fn install_signal_handlers(shutdown: Arc<ShutdownSignal>) -> std::io::Result<()> {
    // Use signal-hook's low_level::register for custom signal handling
    // This allows us to track both the flag and the signal number

    // Install handler for SIGINT (Ctrl+C)
    // SAFETY: signal handlers are properly registered and our closure only touches atomics
    let shutdown_sigint = shutdown.clone();
    unsafe {
        signal_hook::low_level::register(SIGINT, move || {
            shutdown_sigint.trigger(SIGINT);
        })?;
    }

    // Install handler for SIGTERM
    let shutdown_sigterm = shutdown.clone();
    unsafe {
        signal_hook::low_level::register(SIGTERM, move || {
            shutdown_sigterm.trigger(SIGTERM);
        })?;
    }

    // Install handler for SIGHUP (Unix only)
    #[cfg(unix)]
    {
        let shutdown_sighup = shutdown.clone();
        unsafe {
            signal_hook::low_level::register(SIGHUP, move || {
                shutdown_sighup.trigger(SIGHUP);
            })?;
        }
    }

    Ok(())
}

/// Install simple signal handlers using just an AtomicBool flag
///
/// This is a simpler version that just sets a boolean flag without tracking
/// which signal was received. Useful when you only need to know if shutdown
/// was requested.
///
/// # Arguments
///
/// * `shutdown_flag` - Arc to an AtomicBool that will be set to true on signal
///
/// # Example
///
/// ```rust,no_run
/// use std::sync::atomic::{AtomicBool, Ordering};
/// use std::sync::Arc;
/// use embeddenator_fs::fs::signal::install_simple_signal_handlers;
///
/// let shutdown_flag = Arc::new(AtomicBool::new(false));
/// install_simple_signal_handlers(shutdown_flag.clone()).expect("Signal handler setup failed");
///
/// // Later, check the flag
/// if shutdown_flag.load(Ordering::Acquire) {
///     println!("Shutdown requested!");
/// }
/// ```
pub fn install_simple_signal_handlers(shutdown_flag: Arc<AtomicBool>) -> std::io::Result<()> {
    // SAFETY: signal handlers are properly registered and only touch atomics
    // Register for SIGINT
    let flag_sigint = shutdown_flag.clone();
    unsafe {
        signal_hook::low_level::register(SIGINT, move || {
            flag_sigint.store(true, Ordering::Release);
        })?;
    }

    // Register for SIGTERM
    let flag_sigterm = shutdown_flag.clone();
    unsafe {
        signal_hook::low_level::register(SIGTERM, move || {
            flag_sigterm.store(true, Ordering::Release);
        })?;
    }

    // Register for SIGHUP (Unix only)
    #[cfg(unix)]
    {
        let flag_sighup = shutdown_flag;
        unsafe {
            signal_hook::low_level::register(SIGHUP, move || {
                flag_sighup.store(true, Ordering::Release);
            })?;
        }
    }

    Ok(())
}

/// RAII guard that triggers shutdown on drop
///
/// This is useful for ensuring cleanup happens even on panic.
#[derive(Debug)]
pub struct ShutdownGuard {
    shutdown: Arc<ShutdownSignal>,
    triggered: bool,
}

impl ShutdownGuard {
    /// Create a new shutdown guard
    pub fn new(shutdown: Arc<ShutdownSignal>) -> Self {
        Self {
            shutdown,
            triggered: false,
        }
    }

    /// Mark the guard as triggered (prevents drop from triggering)
    pub fn disarm(&mut self) {
        self.triggered = true;
    }
}

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        if !self.triggered && !self.shutdown.is_signaled() {
            // Trigger shutdown if we're being dropped unexpectedly
            self.shutdown.trigger(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_signal_initial_state() {
        let signal = ShutdownSignal::new();
        assert!(!signal.is_signaled());
        assert_eq!(signal.signal_number(), 0);
        assert_eq!(signal.signal_name(), "none");
    }

    #[test]
    fn test_shutdown_signal_trigger() {
        let signal = ShutdownSignal::new();
        signal.trigger(SIGINT);
        assert!(signal.is_signaled());
        assert_eq!(signal.signal_number(), SIGINT);
        assert_eq!(signal.signal_name(), "SIGINT");
    }

    #[test]
    fn test_shutdown_signal_reset() {
        let signal = ShutdownSignal::new();
        signal.trigger(SIGTERM);
        assert!(signal.is_signaled());

        signal.reset();
        assert!(!signal.is_signaled());
        assert_eq!(signal.signal_number(), 0);
    }

    #[test]
    fn test_shutdown_guard_disarm() {
        let signal = Arc::new(ShutdownSignal::new());
        {
            let mut guard = ShutdownGuard::new(signal.clone());
            guard.disarm();
        }
        // Guard was disarmed, so signal should not be triggered
        assert!(!signal.is_signaled());
    }

    #[test]
    fn test_shutdown_guard_no_disarm() {
        let signal = Arc::new(ShutdownSignal::new());
        {
            let _guard = ShutdownGuard::new(signal.clone());
            // Guard not disarmed
        }
        // Guard was dropped without disarming, so signal should be triggered
        assert!(signal.is_signaled());
    }

    #[test]
    fn test_signal_names() {
        let signal = ShutdownSignal::new();

        signal.trigger(SIGINT);
        assert_eq!(signal.signal_name(), "SIGINT");
        signal.reset();

        signal.trigger(SIGTERM);
        assert_eq!(signal.signal_name(), "SIGTERM");
        signal.reset();

        signal.trigger(SIGHUP);
        assert_eq!(signal.signal_name(), "SIGHUP");
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let signal = Arc::new(ShutdownSignal::new());

        // Spawn multiple threads checking the signal
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let sig = signal.clone();
                thread::spawn(move || {
                    for _ in 0..1000 {
                        let _ = sig.is_signaled();
                    }
                })
            })
            .collect();

        // Trigger signal from main thread
        signal.trigger(SIGINT);

        // Wait for all threads
        for h in handles {
            h.join().unwrap();
        }

        assert!(signal.is_signaled());
    }
}
