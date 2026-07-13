use crate::{HedgeOutcome, HedgeRequest, HedgeResult, HedgingError, HedgingMetrics, HedgingResult};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;

pub struct HedgeManager {
    metrics: Arc<DashMap<String, HedgingMetrics>>,
}

impl HedgeManager {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(DashMap::new()),
        }
    }

    pub async fn execute_hedge(&self, request: &HedgeRequest) -> HedgingResult<HedgeResult> {
        if request.max_hedges == 0 {
            return Err(HedgingError::InvalidMaxHedges);
        }

        if request.hedge_delay_ms == 0 {
            return Err(HedgingError::InvalidHedgeDelay);
        }

        let start_time = Instant::now();
        let mut hedge_latencies = Vec::new();
        let mut winning_attempt = 1;
        let mut outcome = HedgeOutcome::AllFailed;

        for attempt in 1..=request.max_hedges {
            if attempt > 1 {
                let delay = std::time::Duration::from_millis(request.hedge_delay_ms);
                tokio::time::sleep(delay).await;
            }

            let attempt_start = Instant::now();
            let simulated_success = uuid::Uuid::new_v4().as_bytes()[0] % 2 == 0;
            let attempt_duration = attempt_start.elapsed().as_millis() as u64;

            hedge_latencies.push(attempt_duration);

            if simulated_success {
                outcome = if attempt == 1 {
                    HedgeOutcome::FirstSuccess
                } else {
                    HedgeOutcome::HedgeSuccess
                };
                winning_attempt = attempt;
                break;
            }
        }

        let total_latency = start_time.elapsed().as_millis() as u64;

        let result = HedgeResult {
            request_id: request.request_id.clone(),
            outcome,
            winning_attempt,
            total_attempts: request.max_hedges,
            total_latency_ms: total_latency,
            hedge_latencies: hedge_latencies.clone(),
        };

        self.record_hedge(&request.service_id, &result).await?;
        Ok(result)
    }

    async fn record_hedge(&self, service_id: &str, result: &HedgeResult) -> HedgingResult<()> {
        let mut metrics = self
            .metrics
            .entry(service_id.to_string())
            .or_insert_with(|| HedgingMetrics {
                service_id: service_id.to_string(),
                total_requests: 0,
                hedged_requests: 0,
                hedge_success_count: 0,
                first_attempt_success_rate: 0.0,
                avg_hedge_latency_ms: 0.0,
                p99_hedge_latency_ms: 0,
            });

        metrics.total_requests += 1;

        if result.winning_attempt > 1 {
            metrics.hedged_requests += 1;
        }

        if result.outcome != HedgeOutcome::AllFailed {
            metrics.hedge_success_count += 1;
        }

        let avg_latency: f64 = result.hedge_latencies.iter().map(|l| *l as f64).sum::<f64>()
            / result.hedge_latencies.len() as f64;
        metrics.avg_hedge_latency_ms = avg_latency;

        if !result.hedge_latencies.is_empty() {
            let mut sorted = result.hedge_latencies.clone();
            sorted.sort();
            let p99_idx = ((sorted.len() as f64 * 0.99) as usize).max(0);
            metrics.p99_hedge_latency_ms = sorted[p99_idx];
        }

        metrics.first_attempt_success_rate =
            (metrics.hedge_success_count as f64 / metrics.total_requests as f64) * 100.0;

        Ok(())
    }

    pub async fn get_metrics(&self, service_id: &str) -> HedgingResult<HedgingMetrics> {
        self.metrics
            .get(service_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| HedgingError::Internal("Metrics not found".to_string()))
    }

    pub fn metrics_count(&self) -> usize {
        self.metrics.len()
    }
}

impl Default for HedgeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_hedge() {
        let manager = HedgeManager::new();
        let request = HedgeRequest {
            request_id: "req-1".to_string(),
            service_id: "service-1".to_string(),
            original_deadline: Utc::now(),
            hedge_delay_ms: 10,
            max_hedges: 2,
        };

        let result = manager.execute_hedge(&request).await.unwrap();
        assert_eq!(result.request_id, "req-1");
        assert!(result.total_latency_ms > 0);
    }

    #[tokio::test]
    async fn test_invalid_max_hedges() {
        let manager = HedgeManager::new();
        let request = HedgeRequest {
            request_id: "req-1".to_string(),
            service_id: "service-1".to_string(),
            original_deadline: Utc::now(),
            hedge_delay_ms: 10,
            max_hedges: 0,
        };

        let result = manager.execute_hedge(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_hedge_delay() {
        let manager = HedgeManager::new();
        let request = HedgeRequest {
            request_id: "req-1".to_string(),
            service_id: "service-1".to_string(),
            original_deadline: Utc::now(),
            hedge_delay_ms: 0,
            max_hedges: 2,
        };

        let result = manager.execute_hedge(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_metrics() {
        let manager = HedgeManager::new();
        let request = HedgeRequest {
            request_id: "req-1".to_string(),
            service_id: "service-1".to_string(),
            original_deadline: Utc::now(),
            hedge_delay_ms: 10,
            max_hedges: 2,
        };

        manager.execute_hedge(&request).await.unwrap();
        let metrics = manager.get_metrics("service-1").await.unwrap();

        assert_eq!(metrics.total_requests, 1);
    }
}
