//! Core observability types: OpenTelemetry-style distributed tracing
//! (trace/span/event), structured logging, metrics, and correlation
//! context, shared by [`crate::tracing`], [`crate::logging`],
//! [`crate::metrics`], and [`crate::correlation`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Identifier for a distributed trace
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct TraceId(pub String);

/// Identifier for a single span within a trace
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct SpanId(pub String);

/// Identifier correlating logs/metrics/spans across service boundaries
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct CorrelationId(pub String);

/// The kind of work a span represents (OpenTelemetry span kinds)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum SpanKind {
    Unspecified,
    Internal,
    Server,
    Client,
    Producer,
    Consumer,
}

/// Span completion status
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum SpanStatus {
    /// Default status before the span completes
    Unset,
    /// Span completed successfully
    Ok,
    /// Span completed with an error
    Error,
}

/// A point-in-time event attached to a span (e.g. a log line or exception)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub attributes: HashMap<String, String>,
}

/// A link from this span to a causally-related span in another trace
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpanLink {
    pub trace_id: TraceId,
    pub span_id: SpanId,
}

/// A single unit of work within a distributed trace
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Span {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
    pub name: String,
    pub kind: SpanKind,
    pub status: SpanStatus,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_micros: Option<u64>,
    pub attributes: HashMap<String, String>,
    pub events: Vec<SpanEvent>,
    pub links: Vec<SpanLink>,
}

/// A complete distributed trace: a root span plus all its descendants
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trace {
    pub trace_id: TraceId,
    pub root_span_id: SpanId,
    pub spans: Vec<Span>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_micros: u64,
    pub span_count: usize,
}

/// Correlation context propagated across service/process boundaries
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorrelationContext {
    pub correlation_id: CorrelationId,
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub baggage: HashMap<String, String>,
}

/// Structured log severity
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// A single structured log entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub service: String,
    pub trace_id: Option<TraceId>,
    pub span_id: Option<SpanId>,
    pub correlation_id: Option<CorrelationId>,
    pub fields: HashMap<String, String>,
}

/// A single recorded metric sample
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricValue {
    pub name: String,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub labels: HashMap<String, String>,
    pub unit: String,
}

/// Aggregated statistics over a set of [`MetricValue`] samples
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    pub timestamp: DateTime<Utc>,
    pub count: u64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}
