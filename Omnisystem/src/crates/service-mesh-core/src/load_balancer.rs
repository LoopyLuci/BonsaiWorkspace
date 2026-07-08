use crate::{MeshError, MeshResult, RequestMetrics, ServiceEndpoint, ServiceId};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub struct LoadBalancer {
    metrics: Arc<DashMap<String, RequestMetrics>>,
    round_robin_index: Arc<AtomicU32>,
}

impl LoadBalancer {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(DashMap::new()),
            round_robin_index: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn select_round_robin(&self, endpoints: &[ServiceEndpoint]) -> MeshResult<ServiceEndpoint> {
        if endpoints.is_empty() {
            return Err(MeshError::EndpointUnavailable);
        }

        let current = self.round_robin_index.fetch_add(1, Ordering::SeqCst);
        let index = (current as usize) % endpoints.len();
        Ok(endpoints[index].clone())
    }

    pub fn select_least_connections(
        &self,
        endpoints: &[ServiceEndpoint],
    ) -> MeshResult<ServiceEndpoint> {
        if endpoints.is_empty() {
            return Err(MeshError::EndpointUnavailable);
        }

        let mut least_failures = u32::MAX;
        let mut selected = &endpoints[0];

        for endpoint in endpoints {
            let failure_count = endpoint.failure_count;
            if failure_count < least_failures {
                least_failures = failure_count;
                selected = endpoint;
            }
        }

        Ok(selected.clone())
    }

    pub fn select_weighted(&self, endpoints: &[ServiceEndpoint]) -> MeshResult<ServiceEndpoint> {
        if endpoints.is_empty() {
            return Err(MeshError::EndpointUnavailable);
        }

        let total_weight: u32 = endpoints.iter().map(|e| e.weight).sum();
        if total_weight == 0 {
            return self.select_round_robin(endpoints);
        }

        let current = self.round_robin_index.fetch_add(1, Ordering::SeqCst);
        let mut accumulated_weight = 0;
        let target = (current % total_weight) + 1;

        for endpoint in endpoints {
            accumulated_weight += endpoint.weight;
            if accumulated_weight >= target {
                return Ok(endpoint.clone());
            }
        }

        Ok(endpoints[0].clone())
    }

    pub async fn update_metrics(&self, service_id: &ServiceId, metrics: &RequestMetrics) -> MeshResult<()> {
        self.metrics.insert(service_id.0.clone(), metrics.clone());
        Ok(())
    }

    pub async fn get_metrics(&self, service_id: &ServiceId) -> MeshResult<RequestMetrics> {
        self.metrics
            .get(&service_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| MeshError::ServiceNotFound(service_id.0.clone()))
    }

    pub fn metrics_count(&self) -> usize {
        self.metrics.len()
    }
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServiceStatus;
    use chrono::Utc;

    fn create_test_endpoint(id: u32, weight: u32) -> ServiceEndpoint {
        ServiceEndpoint {
            id: crate::EndpointId(format!("endpoint-{}", id)),
            address: format!("127.0.0.{}", id),
            port: 8000 + id as u16,
            weight,
            status: ServiceStatus::Healthy,
            last_checked: Utc::now(),
            failure_count: id,
            success_count: 10,
        }
    }

    #[test]
    fn test_round_robin_selection() {
        let lb = LoadBalancer::new();
        let endpoints = vec![
            create_test_endpoint(1, 100),
            create_test_endpoint(2, 100),
            create_test_endpoint(3, 100),
        ];

        let ep1 = lb.select_round_robin(&endpoints).unwrap();
        let ep2 = lb.select_round_robin(&endpoints).unwrap();
        let ep3 = lb.select_round_robin(&endpoints).unwrap();

        assert_eq!(ep1.id.0, "endpoint-1");
        assert_eq!(ep2.id.0, "endpoint-2");
        assert_eq!(ep3.id.0, "endpoint-3");
    }

    #[test]
    fn test_least_connections() {
        let lb = LoadBalancer::new();
        let endpoints = vec![
            create_test_endpoint(1, 100),
            create_test_endpoint(5, 100),
            create_test_endpoint(3, 100),
        ];

        let selected = lb.select_least_connections(&endpoints).unwrap();
        assert_eq!(selected.id.0, "endpoint-1");
    }

    #[test]
    fn test_weighted_selection() {
        let lb = LoadBalancer::new();
        let endpoints = vec![
            create_test_endpoint(1, 70),
            create_test_endpoint(2, 30),
        ];

        let selected = lb.select_weighted(&endpoints).unwrap();
        assert!(selected.id.0 == "endpoint-1" || selected.id.0 == "endpoint-2");
    }

    #[tokio::test]
    async fn test_update_metrics() {
        let lb = LoadBalancer::new();
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());

        let metrics = RequestMetrics {
            request_count: 100,
            success_count: 95,
            failure_count: 5,
            total_latency_ms: 5000,
            min_latency_ms: 10,
            max_latency_ms: 200,
            p50_latency_ms: 50,
            p95_latency_ms: 150,
            p99_latency_ms: 190,
        };

        lb.update_metrics(&service_id, &metrics).await.unwrap();
        assert_eq!(lb.metrics_count(), 1);
    }

    #[tokio::test]
    async fn test_get_metrics() {
        let lb = LoadBalancer::new();
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());

        let metrics = RequestMetrics {
            request_count: 100,
            success_count: 95,
            failure_count: 5,
            total_latency_ms: 5000,
            min_latency_ms: 10,
            max_latency_ms: 200,
            p50_latency_ms: 50,
            p95_latency_ms: 150,
            p99_latency_ms: 190,
        };

        lb.update_metrics(&service_id, &metrics).await.unwrap();
        let retrieved = lb.get_metrics(&service_id).await.unwrap();

        assert_eq!(retrieved.success_count, 95);
    }
}
