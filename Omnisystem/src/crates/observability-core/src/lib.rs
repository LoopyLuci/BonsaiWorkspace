//! Observability Core
//!
//! OpenTelemetry-style building blocks for distributed observability:
//! - [`tracing::DistributedTracer`]: trace/span lifecycle, events, attributes
//! - [`logging::LogCollector`]: structured, trace-correlated log storage/query
//! - [`metrics::MetricsAggregator`]: metric recording with percentile aggregation
//! - [`correlation::CorrelationManager`]: cross-service correlation context/baggage
//!
//! Abstract contracts for each of these live in [`traits`] (kept
//! module-scoped rather than re-exported at the crate root, since each
//! trait shares its name with the concrete type that implements it).

pub mod correlation;
pub mod error;
pub mod logging;
pub mod metrics;
pub mod traits;
pub mod tracing;
pub mod types;

pub use correlation::CorrelationManager;
pub use error::{ObservabilityError, ObservabilityResult};
pub use logging::LogCollector;
pub use metrics::MetricsAggregator;
pub use tracing::DistributedTracer;
pub use types::{
    AggregatedMetrics, CorrelationContext, CorrelationId, LogEntry, LogLevel, MetricValue, Span,
    SpanEvent, SpanId, SpanKind, SpanLink, SpanStatus, Trace, TraceId,
};
