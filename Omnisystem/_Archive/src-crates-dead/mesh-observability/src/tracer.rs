use crate::{Trace, Span, TrafficMetric, ServiceTopology, HealthMetrics, ObservabilityError, ObservabilityResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct DistributedTracer {
    traces: Arc<DashMap<Uuid, Trace>>,
    metrics: Arc<DashMap<Uuid, TrafficMetric>>,
    topology: Arc<DashMap<Uuid, ServiceTopology>>,
    health_metrics: Arc<DashMap<String, HealthMetrics>>,
}

impl DistributedTracer {
    pub fn new() -> Self {
        Self {
            traces: Arc::new(DashMap::new()),
            metrics: Arc::new(DashMap::new()),
            topology: Arc::new(DashMap::new()),
            health_metrics: Arc::new(DashMap::new()),
        }
    }

    pub async fn start_trace(&self, service_name: &str, operation: &str) -> ObservabilityResult<Uuid> {
        let trace_id = Uuid::new_v4();
        let trace = Trace {
            trace_id,
            service_name: service_name.to_string(),
            operation_name: operation.to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_ms: 0,
            spans: Vec::new(),
        };

        self.traces.insert(trace_id, trace);
        Ok(trace_id)
    }

    pub async fn end_trace(&self, trace_id: Uuid, duration_ms: u64) -> ObservabilityResult<()> {
        if let Some(mut trace) = self.traces.get_mut(&trace_id) {
            trace.end_time = Utc::now();
            trace.duration_ms = duration_ms;
            Ok(())
        } else {
            Err(ObservabilityError::TraceNotFound)
        }
    }

    pub async fn add_span(&self, trace_id: Uuid, span: &Span) -> ObservabilityResult<()> {
        if let Some(mut trace) = self.traces.get_mut(&trace_id) {
            trace.spans.push(span.clone());
            Ok(())
        } else {
            Err(ObservabilityError::SpanNotFound)
        }
    }

    pub async fn record_traffic_metric(&self, metric: &TrafficMetric) -> ObservabilityResult<()> {
        self.metrics.insert(metric.metric_id, metric.clone());
        Ok(())
    }

    pub async fn discover_topology(&self, services: Vec<String>) -> ObservabilityResult<Uuid> {
        let topology_id = Uuid::new_v4();
        let topology = ServiceTopology {
            topology_id,
            connections: Vec::new(),
            total_services: services.len(),
            services,
        };

        self.topology.insert(topology_id, topology);
        Ok(topology_id)
    }

    pub async fn record_health(&self, health: &HealthMetrics) -> ObservabilityResult<()> {
        self.health_metrics.insert(health.service_name.clone(), health.clone());
        Ok(())
    }

    pub async fn get_service_health(&self, service_name: &str) -> ObservabilityResult<HealthMetrics> {
        self.health_metrics
            .get(service_name)
            .map(|h| h.clone())
            .ok_or(ObservabilityError::TraceNotFound)
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
        let trace_id = tracer.start_trace("api", "POST /users").await.unwrap();
        assert!(!trace_id.is_nil());
    }

    #[tokio::test]
    async fn test_end_trace() {
        let tracer = DistributedTracer::new();
        let trace_id = tracer.start_trace("web", "GET /home").await.unwrap();
        tracer.end_trace(trace_id, 150).await.unwrap();

        let trace = tracer.traces.get(&trace_id).unwrap();
        assert_eq!(trace.duration_ms, 150);
    }

    #[tokio::test]
    async fn test_record_traffic_metric() {
        let tracer = DistributedTracer::new();
        let metric = TrafficMetric {
            metric_id: Uuid::new_v4(),
            source_service: "frontend".to_string(),
            destination_service: "backend".to_string(),
            request_count: 1000,
            error_count: 5,
            avg_latency_ms: 45.5,
            success_rate: 99.5,
        };

        tracer.record_traffic_metric(&metric).await.unwrap();
    }

    #[tokio::test]
    async fn test_discover_topology() {
        let tracer = DistributedTracer::new();
        let services = vec!["api".to_string(), "db".to_string(), "cache".to_string()];

        let topology_id = tracer.discover_topology(services).await.unwrap();
        assert!(!topology_id.is_nil());
    }
}
