use crate::{AggregatorResult, ServiceObservabilityMetrics};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct ObservabilityIntegrationLayer {
    service_metrics: Arc<DashMap<String, ServiceObservabilityMetrics>>,
    event_log: Arc<DashMap<String, Vec<String>>>,
    correlation_map: Arc<DashMap<String, String>>,
}

impl ObservabilityIntegrationLayer {
    pub fn new() -> Self {
        Self {
            service_metrics: Arc::new(DashMap::new()),
            event_log: Arc::new(DashMap::new()),
            correlation_map: Arc::new(DashMap::new()),
        }
    }

    pub async fn record_service_metrics(
        &self,
        service_id: &str,
        request_count: u64,
        error_count: u64,
        latencies: Vec<f64>,
    ) -> AggregatorResult<()> {
        let mut sorted_latencies = latencies.clone();
        sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p50_idx = (sorted_latencies.len() as f64 * 0.5) as usize;
        let p95_idx = (sorted_latencies.len() as f64 * 0.95) as usize;
        let p99_idx = (sorted_latencies.len() as f64 * 0.99) as usize;

        let p50 = sorted_latencies.get(p50_idx).copied().unwrap_or(0.0);
        let p95 = sorted_latencies.get(p95_idx).copied().unwrap_or(0.0);
        let p99 = sorted_latencies.get(p99_idx).copied().unwrap_or(0.0);

        let avg = if !sorted_latencies.is_empty() {
            sorted_latencies.iter().sum::<f64>() / sorted_latencies.len() as f64
        } else {
            0.0
        };

        let min = sorted_latencies.iter().copied().fold(f64::INFINITY, f64::min);
        let max = sorted_latencies.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        let success_count = request_count.saturating_sub(error_count);
        let success_rate = if request_count > 0 {
            (success_count as f64 / request_count as f64) * 100.0
        } else {
            0.0
        };

        let metrics = ServiceObservabilityMetrics {
            service_id: service_id.to_string(),
            request_count,
            error_count,
            success_rate,
            p50_latency_ms: p50,
            p95_latency_ms: p95,
            p99_latency_ms: p99,
            avg_latency_ms: avg,
            min_latency_ms: min,
            max_latency_ms: max,
            last_updated: Utc::now(),
        };

        self.service_metrics
            .insert(service_id.to_string(), metrics);

        Ok(())
    }

    pub async fn get_service_metrics(&self, service_id: &str) -> AggregatorResult<ServiceObservabilityMetrics> {
        self.service_metrics
            .get(service_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| crate::AggregatorError::MetricNotFound(service_id.to_string()))
    }

    pub async fn record_event(&self, service_id: &str, event: &str) -> AggregatorResult<()> {
        self.event_log
            .entry(service_id.to_string())
            .or_insert_with(Vec::new)
            .push(event.to_string());

        Ok(())
    }

    pub async fn get_events(&self, service_id: &str, limit: usize) -> AggregatorResult<Vec<String>> {
        if let Some(events) = self.event_log.get(service_id) {
            Ok(events.iter().rev().take(limit).cloned().collect())
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn correlate_requests(&self, correlation_id: &str, trace_id: &str) -> AggregatorResult<()> {
        self.correlation_map
            .insert(correlation_id.to_string(), trace_id.to_string());
        Ok(())
    }

    pub async fn get_trace_id(&self, correlation_id: &str) -> AggregatorResult<Option<String>> {
        Ok(self
            .correlation_map
            .get(correlation_id)
            .map(|entry| entry.value().clone()))
    }

    pub fn service_count(&self) -> usize {
        self.service_metrics.len()
    }

    pub async fn get_all_service_metrics(&self) -> AggregatorResult<Vec<ServiceObservabilityMetrics>> {
        Ok(self
            .service_metrics
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }
}

impl Default for ObservabilityIntegrationLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_service_metrics() {
        let layer = ObservabilityIntegrationLayer::new();
        let latencies = vec![10.0, 20.0, 30.0, 40.0, 50.0];

        layer
            .record_service_metrics("api-service", 100, 5, latencies)
            .await
            .unwrap();

        assert_eq!(layer.service_count(), 1);
    }

    #[tokio::test]
    async fn test_get_service_metrics() {
        let layer = ObservabilityIntegrationLayer::new();
        let latencies = vec![10.0, 50.0, 100.0];

        layer
            .record_service_metrics("api-service", 100, 5, latencies)
            .await
            .unwrap();

        let metrics = layer.get_service_metrics("api-service").await.unwrap();
        assert_eq!(metrics.service_id, "api-service");
        assert_eq!(metrics.request_count, 100);
        assert_eq!(metrics.error_count, 5);
    }

    #[tokio::test]
    async fn test_success_rate_calculation() {
        let layer = ObservabilityIntegrationLayer::new();

        layer
            .record_service_metrics("api-service", 100, 10, vec![50.0])
            .await
            .unwrap();

        let metrics = layer.get_service_metrics("api-service").await.unwrap();
        assert_eq!(metrics.success_rate, 90.0);
    }

    #[tokio::test]
    async fn test_record_event() {
        let layer = ObservabilityIntegrationLayer::new();

        layer.record_event("api-service", "request_received").await.unwrap();
        layer.record_event("api-service", "request_completed").await.unwrap();

        let events = layer.get_events("api-service", 10).await.unwrap();
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_correlate_requests() {
        let layer = ObservabilityIntegrationLayer::new();

        layer
            .correlate_requests("corr-123", "trace-456")
            .await
            .unwrap();

        let trace_id = layer.get_trace_id("corr-123").await.unwrap();
        assert_eq!(trace_id, Some("trace-456".to_string()));
    }

    #[tokio::test]
    async fn test_get_all_service_metrics() {
        let layer = ObservabilityIntegrationLayer::new();

        layer
            .record_service_metrics("service-1", 50, 2, vec![10.0])
            .await
            .unwrap();
        layer
            .record_service_metrics("service-2", 100, 5, vec![20.0])
            .await
            .unwrap();

        let all_metrics = layer.get_all_service_metrics().await.unwrap();
        assert_eq!(all_metrics.len(), 2);
    }

    #[tokio::test]
    async fn test_percentile_calculation() {
        let layer = ObservabilityIntegrationLayer::new();
        let latencies = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];

        layer
            .record_service_metrics("api-service", 1000, 0, latencies)
            .await
            .unwrap();

        let metrics = layer.get_service_metrics("api-service").await.unwrap();
        assert!(metrics.p50_latency_ms > 0.0);
        assert!(metrics.p95_latency_ms > metrics.p50_latency_ms);
        assert!(metrics.p99_latency_ms >= metrics.p95_latency_ms);
    }
}
