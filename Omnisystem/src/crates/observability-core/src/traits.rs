use async_trait::async_trait;
use crate::{
    CorrelationId, LogEntry, LogLevel, ObservabilityResult, Span, SpanId, SpanKind,
    Trace, TraceId,
};
use std::collections::HashMap;

#[async_trait]
pub trait TracingBackend: Send + Sync {
    async fn start_span(
        &self,
        trace_id: &TraceId,
        span_id: &SpanId,
        parent_span_id: Option<&SpanId>,
        name: &str,
        kind: SpanKind,
    ) -> ObservabilityResult<()>;

    async fn end_span(&self, span_id: &SpanId) -> ObservabilityResult<()>;

    async fn add_event(
        &self,
        span_id: &SpanId,
        event_name: &str,
        attributes: HashMap<String, String>,
    ) -> ObservabilityResult<()>;

    async fn set_attribute(
        &self,
        span_id: &SpanId,
        key: &str,
        value: &str,
    ) -> ObservabilityResult<()>;

    async fn record_exception(
        &self,
        span_id: &SpanId,
        error: &str,
    ) -> ObservabilityResult<()>;

    async fn get_span(&self, span_id: &SpanId) -> ObservabilityResult<Span>;

    async fn get_trace(&self, trace_id: &TraceId) -> ObservabilityResult<Trace>;

    async fn list_spans(&self, trace_id: &TraceId) -> ObservabilityResult<Vec<Span>>;
}

#[async_trait]
pub trait LoggingBackend: Send + Sync {
    async fn write_log(&self, entry: &LogEntry) -> ObservabilityResult<()>;

    async fn write_batch(&self, entries: Vec<LogEntry>) -> ObservabilityResult<()>;

    async fn query_logs(
        &self,
        trace_id: Option<&TraceId>,
        level: Option<LogLevel>,
        limit: usize,
    ) -> ObservabilityResult<Vec<LogEntry>>;

    async fn get_logs_for_trace(&self, trace_id: &TraceId) -> ObservabilityResult<Vec<LogEntry>>;

    async fn flush(&self) -> ObservabilityResult<()>;
}

#[async_trait]
pub trait MetricsBackend: Send + Sync {
    async fn record_metric(
        &self,
        name: &str,
        value: f64,
        labels: HashMap<String, String>,
    ) -> ObservabilityResult<()>;

    async fn get_metrics(&self, name: &str) -> ObservabilityResult<Vec<crate::MetricValue>>;

    async fn aggregate_metrics(
        &self,
        name: &str,
    ) -> ObservabilityResult<crate::AggregatedMetrics>;

    async fn flush_metrics(&self) -> ObservabilityResult<()>;
}

#[async_trait]
pub trait CorrelationManager: Send + Sync {
    async fn create_context(
        &self,
        trace_id: &TraceId,
        span_id: &SpanId,
    ) -> ObservabilityResult<CorrelationId>;

    async fn get_context(
        &self,
        correlation_id: &CorrelationId,
    ) -> ObservabilityResult<crate::CorrelationContext>;

    async fn set_baggage(
        &self,
        correlation_id: &CorrelationId,
        key: &str,
        value: &str,
    ) -> ObservabilityResult<()>;

    async fn get_baggage(
        &self,
        correlation_id: &CorrelationId,
        key: &str,
    ) -> ObservabilityResult<Option<String>>;
}
