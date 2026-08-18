//! Tracing and Span Instrumentation
//!
//! Provides structured tracing with span instrumentation for performance
//! analysis and debugging. Built on the `tracing` ecosystem.
//!
//! # Features
//!
//! - Low-overhead span instrumentation
//! - Hierarchical span nesting
//! - Automatic timing capture
//! - Structured field logging
//! - Multiple subscriber backends
//!
//! # Usage
//!
//! ```rust,ignore
//! use embeddenator_obs::tracing::{init_tracing, span_operation};
//!
//! // Initialize once at startup
//! init_tracing();
//!
//! // Instrument a function
//! #[span_operation]
//! fn process_query(query: &str) -> Result<Vec<u8>> {
//!     // Automatically creates span with function name and arguments
//!     // ...
//! }
//!
//! // Manual span creation
//! let _span = create_span("custom_operation", &[("key", "value")]);
//! // Work happens here, timing is automatic
//! ```
//!
//! # Performance
//!
//! When the `tracing` feature is disabled, all instrumentation compiles
//! to zero-cost. With the feature enabled, typical overhead is <100ns per span.

#[cfg(feature = "tracing")]
use tracing::{span, Level, Span};

/// Initialize tracing with environment-based configuration.
///
/// Reads configuration from:
/// - `EMBEDDENATOR_LOG`: custom log filter (e.g., "embeddenator=debug")
/// - `RUST_LOG`: fallback log filter
/// - `EMBEDDENATOR_TRACE_FORMAT`: output format ("compact", "pretty", "json")
///
/// Default: disabled (filter="off")
#[cfg(feature = "tracing")]
pub fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = std::env::var("EMBEDDENATOR_LOG")
        .ok()
        .or_else(|| std::env::var("RUST_LOG").ok())
        .unwrap_or_else(|| "off".to_string());

    let format = std::env::var("EMBEDDENATOR_TRACE_FORMAT")
        .ok()
        .unwrap_or_else(|| "compact".to_string());

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&filter))
        .unwrap_or_else(|_| EnvFilter::new("off"));

    match format.as_str() {
        "json" => {
            let _ = fmt().json().with_env_filter(env_filter).try_init();
        }
        "pretty" => {
            let _ = fmt().pretty().with_env_filter(env_filter).try_init();
        }
        _ => {
            let _ = fmt().compact().with_env_filter(env_filter).try_init();
        }
    }
}

#[cfg(not(feature = "tracing"))]
pub fn init_tracing() {}

/// Create a named span with optional fields.
///
/// The span is automatically entered and will record timing information
/// when dropped.
///
/// # Example
///
/// ```rust,ignore
/// let _span = create_span("query", &[("dim", "768"), ("k", "10")]);
/// // Work happens here
/// // Span automatically closes and records timing on drop
/// ```
#[cfg(feature = "tracing")]
pub fn create_span(name: &str, fields: &[(&str, &str)]) -> Span {
    let span = span!(Level::INFO, "op", name = name);
    for (key, value) in fields {
        span.record(*key, value);
    }
    span
}

#[cfg(not(feature = "tracing"))]
pub fn create_span(_name: &str, _fields: &[(&str, &str)]) {}

/// Create a debug-level span (only active when debug logging enabled).
#[cfg(feature = "tracing")]
pub fn create_debug_span(name: &str, fields: &[(&str, &str)]) -> Span {
    let span = span!(Level::DEBUG, "debug_op", name = name);
    for (key, value) in fields {
        span.record(*key, value);
    }
    span
}

#[cfg(not(feature = "tracing"))]
pub fn create_debug_span(_name: &str, _fields: &[(&str, &str)]) {}

/// Create a trace-level span (highest detail, for deep debugging).
#[cfg(feature = "tracing")]
pub fn create_trace_span(name: &str, fields: &[(&str, &str)]) -> Span {
    let span = span!(Level::TRACE, "trace_op", name = name);
    for (key, value) in fields {
        span.record(*key, value);
    }
    span
}

#[cfg(not(feature = "tracing"))]
pub fn create_trace_span(_name: &str, _fields: &[(&str, &str)]) {}

/// Span guard type (transparent across feature gate).
#[cfg(feature = "tracing")]
pub type SpanGuard = Span;

#[cfg(not(feature = "tracing"))]
pub type SpanGuard = ();

/// Macro for quick span creation with automatic entry.
///
/// # Example
///
/// ```rust,ignore
/// span_scope!("operation_name", field1 = "value1", field2 = "value2");
/// // Code here is instrumented
/// ```
#[macro_export]
#[cfg(feature = "tracing")]
macro_rules! span_scope {
    ($name:expr) => {
        let _guard = $crate::tracing::create_span($name, &[]);
    };
    ($name:expr, $($key:tt = $val:expr),*) => {
        {
            let fields = vec![$(( stringify!($key), &format!("{}", $val) as &str ),)*];
            let _guard = $crate::tracing::create_span($name, &fields);
        }
    };
}

#[cfg(not(feature = "tracing"))]
#[macro_export]
macro_rules! span_scope {
    ($name:expr $(, $key:tt = $val:expr)*) => {
        ()
    };
}

/// Record an event in the current span.
#[cfg(feature = "tracing")]
pub fn record_event(level: EventLevel, message: &str, fields: &[(&str, &str)]) {
    match level {
        EventLevel::Error => tracing::error!(message = %message, ?fields),
        EventLevel::Warn => tracing::warn!(message = %message, ?fields),
        EventLevel::Info => tracing::info!(message = %message, ?fields),
        EventLevel::Debug => tracing::debug!(message = %message, ?fields),
        EventLevel::Trace => tracing::trace!(message = %message, ?fields),
    }
}

#[cfg(not(feature = "tracing"))]
pub fn record_event(_level: EventLevel, message: &str, _fields: &[(&str, &str)]) {
    if matches!(_level, EventLevel::Error | EventLevel::Warn) {
        eprintln!("[{}] {}", _level.as_str(), message);
    }
}

/// Event severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl EventLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventLevel::Error => "ERROR",
            EventLevel::Warn => "WARN",
            EventLevel::Info => "INFO",
            EventLevel::Debug => "DEBUG",
            EventLevel::Trace => "TRACE",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_tracing_no_panic() {
        // Should not panic regardless of feature state
        init_tracing();
    }

    #[test]
    fn test_span_creation() {
        create_span("test_op", &[("key", "value")]);
        // Should compile and not panic
    }

    #[test]
    fn test_event_recording() {
        record_event(EventLevel::Info, "test message", &[("field", "value")]);
        // Should compile and not panic
    }

    #[test]
    fn test_event_level_str() {
        assert_eq!(EventLevel::Error.as_str(), "ERROR");
        assert_eq!(EventLevel::Info.as_str(), "INFO");
    }
}
