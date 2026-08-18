//! Structured Logging Infrastructure
//!
//! Provides configurable logging with multiple output formats and
//! environment-based filtering. Integrates with the `tracing` ecosystem
//! for structured, leveled logging.
//!
//! # Features
//!
//! - Environment-based log filtering
//! - Multiple output formats (compact, pretty, JSON)
//! - Zero-cost when disabled
//! - Standard error and warning helpers
//!
//! # Configuration
//!
//! Set log level via environment variables:
//! - `EMBEDDENATOR_LOG="info"` - custom filter
//! - `RUST_LOG="debug"` - fallback filter
//!
//! Set output format:
//! - `EMBEDDENATOR_LOG_FORMAT="json"` - structured JSON output
//! - `EMBEDDENATOR_LOG_FORMAT="pretty"` - pretty-printed output
//! - `EMBEDDENATOR_LOG_FORMAT="compact"` - compact output (default)

#[cfg(feature = "logging")]
use std::io;

/// Initialize structured logging.
///
/// Behavior:
/// - With `--features logging`: installs a `tracing_subscriber` configured by
///   `EMBEDDENATOR_LOG` or `RUST_LOG`.
/// - Without the feature: no-op.
///
/// By default (no env var), logging is disabled.
#[cfg(feature = "logging")]
pub fn init() {
    use tracing_subscriber::fmt;

    let filter = std::env::var("EMBEDDENATOR_LOG")
        .ok()
        .or_else(|| std::env::var("RUST_LOG").ok())
        .unwrap_or_else(|| "off".to_string());

    let format = std::env::var("EMBEDDENATOR_LOG_FORMAT")
        .ok()
        .unwrap_or_else(|| "compact".to_string());

    match format.as_str() {
        "json" => {
            let _ = fmt()
                .json()
                .with_env_filter(filter)
                .with_writer(io::stderr)
                .try_init();
        }
        "pretty" => {
            let _ = fmt()
                .pretty()
                .with_env_filter(filter)
                .with_writer(io::stderr)
                .try_init();
        }
        _ => {
            let _ = fmt()
                .compact()
                .with_env_filter(filter)
                .with_writer(io::stderr)
                .try_init();
        }
    }
}

#[cfg(not(feature = "logging"))]
pub fn init() {}

/// Emit a warning in the best available way.
///
/// This intentionally preserves existing default behavior for builds without
/// `logging` (warnings still go to stderr). When `logging` is enabled, warnings
/// become structured `tracing` events.
#[cfg(feature = "logging")]
pub fn warn(message: &str) {
    tracing::warn!(message = %message);
}

#[cfg(not(feature = "logging"))]
pub fn warn(message: &str) {
    eprintln!("WARN: {}", message);
}

/// Emit an error message.
#[cfg(feature = "logging")]
pub fn error(message: &str) {
    tracing::error!(message = %message);
}

#[cfg(not(feature = "logging"))]
pub fn error(message: &str) {
    eprintln!("ERROR: {}", message);
}

/// Emit an info message.
#[cfg(feature = "logging")]
pub fn info(message: &str) {
    tracing::info!(message = %message);
}

#[cfg(not(feature = "logging"))]
pub fn info(_message: &str) {
    // No-op without logging feature
}

/// Emit a debug message.
#[cfg(feature = "logging")]
pub fn debug(message: &str) {
    tracing::debug!(message = %message);
}

#[cfg(not(feature = "logging"))]
pub fn debug(_message: &str) {
    // No-op without logging feature
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_no_panic() {
        init();
    }

    #[test]
    fn test_warn() {
        warn("test warning");
    }

    #[test]
    fn test_error() {
        error("test error");
    }

    #[test]
    fn test_info() {
        info("test info");
    }

    #[test]
    fn test_debug() {
        debug("test debug");
    }
}
