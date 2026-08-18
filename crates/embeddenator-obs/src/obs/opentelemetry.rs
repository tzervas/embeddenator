//! OpenTelemetry Integration
//!
//! Provides OpenTelemetry-compatible tracing and metrics export for
//! distributed observability and integration with OTLP collectors.
//!
//! # Features
//!
//! - Span context propagation (W3C Trace Context)
//! - OTLP-compatible span export
//! - Distributed trace IDs
//! - Parent-child span relationships
//! - Span attributes and events
//!
//! # Usage
//!
//! ```rust,ignore
//! use embeddenator_obs::opentelemetry::{OtelSpan, OtelExporter};
//!
//! let mut span = OtelSpan::new("operation");
//! span.set_attribute("key", "value");
//! span.add_event("checkpoint");
//! span.end();
//!
//! let exporter = OtelExporter::new();
//! let json = exporter.export_spans(&[span]);
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Global trace ID counter for generating unique IDs.
static TRACE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
static SPAN_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// OpenTelemetry span status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStatus {
    /// Span completed successfully
    Ok,
    /// Span encountered an error
    Error,
    /// Status not set
    Unset,
}

/// OpenTelemetry span kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    /// Internal operation
    Internal,
    /// Server-side operation
    Server,
    /// Client-side operation
    Client,
    /// Producer (message queue)
    Producer,
    /// Consumer (message queue)
    Consumer,
}

/// OpenTelemetry span with full tracing context.
#[derive(Debug, Clone)]
pub struct OtelSpan {
    /// Unique trace ID (128-bit in production, 64-bit here for simplicity)
    pub trace_id: u64,
    /// Unique span ID
    pub span_id: u64,
    /// Parent span ID (0 if root)
    pub parent_span_id: u64,
    /// Operation name
    pub name: String,
    /// Span kind
    pub kind: SpanKind,
    /// Start timestamp (nanoseconds since epoch)
    pub start_time_ns: u64,
    /// End timestamp (0 if still active)
    pub end_time_ns: u64,
    /// Span status
    pub status: SpanStatus,
    /// Span attributes (key-value pairs)
    pub attributes: HashMap<String, String>,
    /// Span events
    pub events: Vec<SpanEvent>,
}

/// Span event (checkpoint within a span).
#[derive(Debug, Clone)]
pub struct SpanEvent {
    /// Event name
    pub name: String,
    /// Timestamp (nanoseconds since epoch)
    pub timestamp_ns: u64,
    /// Event attributes
    pub attributes: HashMap<String, String>,
}

impl OtelSpan {
    /// Create new root span.
    pub fn new(name: impl Into<String>) -> Self {
        let trace_id = TRACE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let span_id = SPAN_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        Self {
            trace_id,
            span_id,
            parent_span_id: 0,
            name: name.into(),
            kind: SpanKind::Internal,
            start_time_ns: system_time_nanos(),
            end_time_ns: 0,
            status: SpanStatus::Unset,
            attributes: HashMap::new(),
            events: Vec::new(),
        }
    }

    /// Create child span with parent context.
    pub fn new_child(name: impl Into<String>, parent: &OtelSpan) -> Self {
        let span_id = SPAN_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        Self {
            trace_id: parent.trace_id,
            span_id,
            parent_span_id: parent.span_id,
            name: name.into(),
            kind: SpanKind::Internal,
            start_time_ns: system_time_nanos(),
            end_time_ns: 0,
            status: SpanStatus::Unset,
            attributes: HashMap::new(),
            events: Vec::new(),
        }
    }

    /// Set span kind.
    pub fn set_kind(&mut self, kind: SpanKind) {
        self.kind = kind;
    }

    /// Set span attribute.
    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(key.into(), value.into());
    }

    /// Add span event.
    pub fn add_event(&mut self, name: impl Into<String>) {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp_ns: system_time_nanos(),
            attributes: HashMap::new(),
        });
    }

    /// Add span event with attributes.
    pub fn add_event_with_attributes(
        &mut self,
        name: impl Into<String>,
        attributes: HashMap<String, String>,
    ) {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp_ns: system_time_nanos(),
            attributes,
        });
    }

    /// Mark span as completed successfully.
    pub fn end(&mut self) {
        self.end_time_ns = system_time_nanos();
        if self.status == SpanStatus::Unset {
            self.status = SpanStatus::Ok;
        }
    }

    /// Mark span as failed.
    pub fn end_with_error(&mut self, error: impl Into<String>) {
        self.end_time_ns = system_time_nanos();
        self.status = SpanStatus::Error;
        self.set_attribute("error.message", error);
    }

    /// Get span duration in nanoseconds.
    pub fn duration_ns(&self) -> u64 {
        if self.end_time_ns > 0 {
            self.end_time_ns.saturating_sub(self.start_time_ns)
        } else {
            0
        }
    }

    /// Check if span is root (no parent).
    pub fn is_root(&self) -> bool {
        self.parent_span_id == 0
    }

    /// Export as W3C Trace Context header (traceparent).
    pub fn to_traceparent(&self) -> String {
        format!("00-{:032x}-{:016x}-01", self.trace_id, self.span_id)
    }

    /// Parse W3C Trace Context header.
    pub fn from_traceparent(traceparent: &str, name: impl Into<String>) -> Option<Self> {
        let parts: Vec<&str> = traceparent.split('-').collect();
        if parts.len() != 4 || parts[0] != "00" {
            return None;
        }

        let trace_id = u64::from_str_radix(&parts[1][16..32], 16).ok()?;
        let parent_span_id = u64::from_str_radix(parts[2], 16).ok()?;
        let span_id = SPAN_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        Some(Self {
            trace_id,
            span_id,
            parent_span_id,
            name: name.into(),
            kind: SpanKind::Internal,
            start_time_ns: system_time_nanos(),
            end_time_ns: 0,
            status: SpanStatus::Unset,
            attributes: HashMap::new(),
            events: Vec::new(),
        })
    }
}

/// OpenTelemetry exporter for OTLP-compatible output.
pub struct OtelExporter {
    /// Service name
    service_name: String,
}

impl OtelExporter {
    /// Create new OTLP exporter.
    pub fn new() -> Self {
        Self {
            service_name: "embeddenator".to_string(),
        }
    }

    /// Set service name.
    pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = name.into();
        self
    }

    /// Export spans as JSON (simplified OTLP format).
    pub fn export_spans(&self, spans: &[OtelSpan]) -> String {
        let mut output = String::from("{\n  \"resourceSpans\": [\n    {\n");
        output.push_str(&format!("      \"resource\": {{\"attributes\": [{{\"key\": \"service.name\", \"value\": \"{}\"}}]}},\n", self.service_name));
        output.push_str("      \"scopeSpans\": [\n        {\n          \"spans\": [\n");

        for (i, span) in spans.iter().enumerate() {
            if i > 0 {
                output.push_str(",\n");
            }
            output.push_str(&self.span_to_json(span));
        }

        output.push_str("\n          ]\n        }\n      ]\n    }\n  ]\n}");
        output
    }

    fn span_to_json(&self, span: &OtelSpan) -> String {
        let mut json = String::new();
        json.push_str("            {\n");
        json.push_str(&format!(
            "              \"traceId\": \"{:032x}\",\n",
            span.trace_id
        ));
        json.push_str(&format!(
            "              \"spanId\": \"{:016x}\",\n",
            span.span_id
        ));
        if span.parent_span_id != 0 {
            json.push_str(&format!(
                "              \"parentSpanId\": \"{:016x}\",\n",
                span.parent_span_id
            ));
        }
        json.push_str(&format!("              \"name\": \"{}\",\n", span.name));
        json.push_str(&format!(
            "              \"kind\": {:?},\n",
            span.kind as u32
        ));
        json.push_str(&format!(
            "              \"startTimeUnixNano\": {},\n",
            span.start_time_ns
        ));
        json.push_str(&format!(
            "              \"endTimeUnixNano\": {},\n",
            span.end_time_ns
        ));
        json.push_str(&format!(
            "              \"status\": {{\"code\": {}}}\n",
            span.status as u32
        ));
        json.push_str("            }");
        json
    }
}

impl Default for OtelExporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current system time in nanoseconds since UNIX epoch.
fn system_time_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_creation() {
        let span = OtelSpan::new("test_operation");
        assert!(span.trace_id > 0);
        assert!(span.span_id > 0);
        assert_eq!(span.parent_span_id, 0);
        assert!(span.is_root());
        assert_eq!(span.status, SpanStatus::Unset);
    }

    #[test]
    fn test_child_span() {
        let parent = OtelSpan::new("parent");
        let child = OtelSpan::new_child("child", &parent);

        assert_eq!(child.trace_id, parent.trace_id);
        assert_ne!(child.span_id, parent.span_id);
        assert_eq!(child.parent_span_id, parent.span_id);
        assert!(!child.is_root());
    }

    #[test]
    fn test_span_attributes() {
        let mut span = OtelSpan::new("test");
        span.set_attribute("key1", "value1");
        span.set_attribute("key2", "value2");

        assert_eq!(span.attributes.get("key1"), Some(&"value1".to_string()));
        assert_eq!(span.attributes.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_span_events() {
        let mut span = OtelSpan::new("test");
        span.add_event("event1");
        span.add_event("event2");

        assert_eq!(span.events.len(), 2);
        assert_eq!(span.events[0].name, "event1");
        assert_eq!(span.events[1].name, "event2");
    }

    #[test]
    fn test_span_end() {
        let mut span = OtelSpan::new("test");
        std::thread::sleep(Duration::from_millis(10));
        span.end();

        assert!(span.end_time_ns > span.start_time_ns);
        assert_eq!(span.status, SpanStatus::Ok);
        assert!(span.duration_ns() > 0);
    }

    #[test]
    fn test_span_error() {
        let mut span = OtelSpan::new("test");
        span.end_with_error("Something went wrong");

        assert_eq!(span.status, SpanStatus::Error);
        assert!(span.attributes.contains_key("error.message"));
    }

    #[test]
    fn test_traceparent_export() {
        let span = OtelSpan::new("test");
        let traceparent = span.to_traceparent();

        assert!(traceparent.starts_with("00-"));
        assert!(traceparent.ends_with("-01"));
    }

    #[test]
    fn test_traceparent_parse() {
        let parent = OtelSpan::new("parent");
        let traceparent = parent.to_traceparent();

        let child = OtelSpan::from_traceparent(&traceparent, "child").unwrap();
        assert_eq!(child.trace_id, parent.trace_id);
        assert_eq!(child.parent_span_id, parent.span_id);
    }

    #[test]
    fn test_exporter() {
        let mut span = OtelSpan::new("test");
        span.set_attribute("key", "value");
        span.end();

        let exporter = OtelExporter::new().with_service_name("test_service");
        let json = exporter.export_spans(&[span]);

        assert!(json.contains("test_service"));
        assert!(json.contains("test"));
        assert!(json.contains("traceId"));
    }
}
