use crate::{Span, SpanStatus, Metric, Trace, DistributedContext, ObservabilityError, ObservabilityResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct ObservabilityCore {
    spans: Arc<DashMap<Uuid, Span>>,
    metrics: Arc<DashMap<Uuid, Metric>>,
    traces: Arc<DashMap<Uuid, Trace>>,
    contexts: Arc<DashMap<Uuid, DistributedContext>>,
}

impl ObservabilityCore {
    pub fn new() -> Self {
        Self {
            spans: Arc::new(DashMap::new()),
            metrics: Arc::new(DashMap::new()),
            traces: Arc::new(DashMap::new()),
            contexts: Arc::new(DashMap::new()),
        }
    }

    pub async fn start_span(&self, trace_id: Uuid, operation_name: &str) -> ObservabilityResult<Span> {
        let span = Span {
            span_id: Uuid::new_v4(),
            trace_id,
            operation_name: operation_name.to_string(),
            start_time: Utc::now(),
            end_time: None,
            duration_ms: None,
            status: SpanStatus::Running,
        };

        self.spans.insert(span.span_id, span.clone());
        Ok(span)
    }

    pub async fn end_span(&self, span_id: Uuid) -> ObservabilityResult<()> {
        if let Some(mut entry) = self.spans.get_mut(&span_id) {
            let now = Utc::now();
            entry.end_time = Some(now);
            entry.status = SpanStatus::Completed;
            entry.duration_ms = Some((now - entry.start_time).num_milliseconds() as u64);
        }

        Ok(())
    }

    pub async fn record_metric(&self, metric: &Metric) -> ObservabilityResult<()> {
        self.metrics.insert(metric.metric_id, metric.clone());
        Ok(())
    }

    pub async fn create_trace(&self, service_name: &str) -> ObservabilityResult<Trace> {
        let trace = Trace {
            trace_id: Uuid::new_v4(),
            root_span_id: Uuid::new_v4(),
            service_name: service_name.to_string(),
            start_time: Utc::now(),
            total_duration_ms: 0,
        };

        self.traces.insert(trace.trace_id, trace.clone());
        Ok(trace)
    }

    pub async fn propagate_context(&self, context: &DistributedContext) -> ObservabilityResult<()> {
        self.contexts.insert(context.trace_id, context.clone());
        Ok(())
    }

    pub async fn get_trace(&self, trace_id: Uuid) -> ObservabilityResult<Trace> {
        self.traces
            .get(&trace_id)
            .map(|t| t.clone())
            .ok_or(ObservabilityError::SpanNotFound)
    }

    pub fn span_count(&self) -> usize {
        self.spans.len()
    }
}

impl Default for ObservabilityCore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_span() {
        let obs = ObservabilityCore::new();
        let trace_id = Uuid::new_v4();
        let span = obs.start_span(trace_id, "api_call").await.unwrap();

        assert_eq!(span.status, SpanStatus::Running);
        assert_eq!(obs.span_count(), 1);
    }

    #[tokio::test]
    async fn test_end_span() {
        let obs = ObservabilityCore::new();
        let trace_id = Uuid::new_v4();
        let span = obs.start_span(trace_id, "db_query").await.unwrap();

        obs.end_span(span.span_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_record_metric() {
        let obs = ObservabilityCore::new();
        let metric = Metric {
            metric_id: Uuid::new_v4(),
            name: "cpu_usage".to_string(),
            value: 45.5,
            timestamp: Utc::now(),
            labels: vec![("host".to_string(), "server-1".to_string())],
        };

        obs.record_metric(&metric).await.unwrap();
    }

    #[tokio::test]
    async fn test_create_trace() {
        let obs = ObservabilityCore::new();
        let trace = obs.create_trace("payment-service").await.unwrap();

        assert_eq!(trace.service_name, "payment-service");
        let retrieved = obs.get_trace(trace.trace_id).await.unwrap();
        assert_eq!(retrieved.service_name, "payment-service");
    }
}
