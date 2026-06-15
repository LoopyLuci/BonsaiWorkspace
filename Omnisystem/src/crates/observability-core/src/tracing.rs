use crate::{ObservabilityError, ObservabilityResult, Span, SpanId, SpanKind, SpanStatus, Trace, TraceId, SpanEvent};
use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;

pub struct DistributedTracer {
    traces: Arc<DashMap<String, Trace>>,
    spans: Arc<DashMap<String, Span>>,
}

impl DistributedTracer {
    pub fn new() -> Self {
        Self {
            traces: Arc::new(DashMap::new()),
            spans: Arc::new(DashMap::new()),
        }
    }

    pub async fn start_trace(&self, trace_id: &TraceId, root_span_id: &SpanId, name: &str) -> ObservabilityResult<()> {
        let now = Utc::now();

        let span = Span {
            trace_id: trace_id.clone(),
            span_id: root_span_id.clone(),
            parent_span_id: None,
            name: name.to_string(),
            kind: SpanKind::Server,
            status: SpanStatus::Unset,
            start_time: now,
            end_time: None,
            duration_micros: None,
            attributes: HashMap::new(),
            events: Vec::new(),
            links: Vec::new(),
        };

        self.spans.insert(root_span_id.0.clone(), span.clone());

        let trace = Trace {
            trace_id: trace_id.clone(),
            root_span_id: root_span_id.clone(),
            spans: vec![span],
            start_time: now,
            end_time: now,
            duration_micros: 0,
            span_count: 1,
        };

        self.traces.insert(trace_id.0.clone(), trace);
        Ok(())
    }

    pub async fn start_span(
        &self,
        trace_id: &TraceId,
        span_id: &SpanId,
        parent_span_id: Option<&SpanId>,
        name: &str,
        kind: SpanKind,
    ) -> ObservabilityResult<()> {
        if !self.traces.contains_key(&trace_id.0) {
            return Err(ObservabilityError::TraceNotFound(trace_id.0.clone()));
        }

        let now = Utc::now();
        let span = Span {
            trace_id: trace_id.clone(),
            span_id: span_id.clone(),
            parent_span_id: parent_span_id.cloned(),
            name: name.to_string(),
            kind,
            status: SpanStatus::Unset,
            start_time: now,
            end_time: None,
            duration_micros: None,
            attributes: HashMap::new(),
            events: Vec::new(),
            links: Vec::new(),
        };

        self.spans.insert(span_id.0.clone(), span.clone());

        if let Some(mut trace) = self.traces.get_mut(&trace_id.0) {
            trace.spans.push(span);
            trace.span_count += 1;
        }

        Ok(())
    }

    pub async fn end_span(&self, span_id: &SpanId) -> ObservabilityResult<()> {
        if let Some(mut span) = self.spans.get_mut(&span_id.0) {
            let end_time = Utc::now();
            span.end_time = Some(end_time);
            span.status = SpanStatus::Ok;
            span.duration_micros = Some((end_time - span.start_time).num_microseconds().unwrap_or(0) as u64);
        }
        Ok(())
    }

    pub async fn add_event(
        &self,
        span_id: &SpanId,
        event_name: &str,
        attributes: HashMap<String, String>,
    ) -> ObservabilityResult<()> {
        if let Some(mut span) = self.spans.get_mut(&span_id.0) {
            let event = SpanEvent {
                name: event_name.to_string(),
                timestamp: Utc::now(),
                attributes,
            };
            span.events.push(event);
        }
        Ok(())
    }

    pub async fn set_attribute(&self, span_id: &SpanId, key: &str, value: &str) -> ObservabilityResult<()> {
        if let Some(mut span) = self.spans.get_mut(&span_id.0) {
            span.attributes.insert(key.to_string(), value.to_string());
        }
        Ok(())
    }

    pub async fn record_exception(&self, span_id: &SpanId, error: &str) -> ObservabilityResult<()> {
        if let Some(mut span) = self.spans.get_mut(&span_id.0) {
            span.status = SpanStatus::Error;
            span.attributes.insert("error".to_string(), error.to_string());
        }
        Ok(())
    }

    pub async fn get_span(&self, span_id: &SpanId) -> ObservabilityResult<Span> {
        self.spans
            .get(&span_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| ObservabilityError::SpanNotFound(span_id.0.clone()))
    }

    pub async fn get_trace(&self, trace_id: &TraceId) -> ObservabilityResult<Trace> {
        self.traces
            .get(&trace_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| ObservabilityError::TraceNotFound(trace_id.0.clone()))
    }

    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    pub fn trace_count(&self) -> usize {
        self.traces.len()
    }
}

impl Default for DistributedTracer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_trace() {
        let tracer = DistributedTracer::new();
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());
        let span_id = SpanId(uuid::Uuid::new_v4().to_string());

        tracer.start_trace(&trace_id, &span_id, "test-trace").await.unwrap();
        assert_eq!(tracer.trace_count(), 1);
    }

    #[tokio::test]
    async fn test_start_span() {
        let tracer = DistributedTracer::new();
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());
        let root_span = SpanId(uuid::Uuid::new_v4().to_string());
        let child_span = SpanId(uuid::Uuid::new_v4().to_string());

        tracer.start_trace(&trace_id, &root_span, "root").await.unwrap();
        tracer.start_span(&trace_id, &child_span, Some(&root_span), "child", SpanKind::Internal).await.unwrap();

        assert_eq!(tracer.span_count(), 2);
    }

    #[tokio::test]
    async fn test_end_span() {
        let tracer = DistributedTracer::new();
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());
        let span_id = SpanId(uuid::Uuid::new_v4().to_string());

        tracer.start_trace(&trace_id, &span_id, "test").await.unwrap();
        tracer.end_span(&span_id).await.unwrap();

        let span = tracer.get_span(&span_id).await.unwrap();
        assert_eq!(span.status, SpanStatus::Ok);
        assert!(span.end_time.is_some());
    }

    #[tokio::test]
    async fn test_add_event() {
        let tracer = DistributedTracer::new();
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());
        let span_id = SpanId(uuid::Uuid::new_v4().to_string());

        tracer.start_trace(&trace_id, &span_id, "test").await.unwrap();
        tracer.add_event(&span_id, "test-event", HashMap::new()).await.unwrap();

        let span = tracer.get_span(&span_id).await.unwrap();
        assert_eq!(span.events.len(), 1);
    }

    #[tokio::test]
    async fn test_record_exception() {
        let tracer = DistributedTracer::new();
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());
        let span_id = SpanId(uuid::Uuid::new_v4().to_string());

        tracer.start_trace(&trace_id, &span_id, "test").await.unwrap();
        tracer.record_exception(&span_id, "test error").await.unwrap();

        let span = tracer.get_span(&span_id).await.unwrap();
        assert_eq!(span.status, SpanStatus::Error);
    }
}
