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
    let filter = std::env::var("EMBEDDENATOR_LOG")
        .ok()
        .or_else(|| std::env::var("RUST_LOG").ok())
        .unwrap_or_else(|| "off".to_string());

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .try_init();
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
    eprintln!("{message}");
}
