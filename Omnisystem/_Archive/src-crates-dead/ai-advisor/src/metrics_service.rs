/// Performance metrics collection and analysis for advisor services
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvisorMetrics {
    pub advisor_id: String,
    pub accuracy: f32,
    pub response_time_ms: f32,
    pub success_count: usize,
    pub failure_count: usize,
    pub total_requests: usize,
    pub last_updated: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceScore {
    pub advisor_id: String,
    pub score: f32,
    pub accuracy_weight: f32,
    pub speed_weight: f32,
    pub reliability_weight: f32,
    pub calculated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceQualityMetrics {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub average_response_time_ms: f32,
    pub accuracy_rate: f32,
    pub timestamp: i64,
}

pub struct MetricsService {
    advisor_metrics: Arc<RwLock<HashMap<String, AdvisorMetrics>>>,
    performance_scores: Arc<RwLock<HashMap<String, PerformanceScore>>>,
    service_quality: Arc<RwLock<ServiceQualityMetrics>>,
    request_times: Arc<RwLock<Vec<f32>>>,
}

impl MetricsService {
    pub fn new() -> Self {
        Self {
            advisor_metrics: Arc::new(RwLock::new(HashMap::new())),
            performance_scores: Arc::new(RwLock::new(HashMap::new())),
            service_quality: Arc::new(RwLock::new(ServiceQualityMetrics {
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                average_response_time_ms: 0.0,
                accuracy_rate: 0.0,
                timestamp: chrono::Utc::now().timestamp(),
            })),
            request_times: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn record_advisor_success(&self, advisor_id: String, accuracy: f32, response_time_ms: f32) -> Result<()> {
        let mut metrics = self.advisor_metrics.write().await;
        let entry = metrics.entry(advisor_id).or_insert(AdvisorMetrics {
            advisor_id: "".to_string(),
            accuracy: 0.0,
            response_time_ms: 0.0,
            success_count: 0,
            failure_count: 0,
            total_requests: 0,
            last_updated: 0,
        });

        entry.success_count += 1;
        entry.total_requests += 1;
        entry.accuracy = (entry.accuracy * (entry.success_count - 1) as f32 + accuracy) / entry.success_count as f32;
        entry.response_time_ms = (entry.response_time_ms * (entry.success_count - 1) as f32 + response_time_ms) / entry.success_count as f32;
        entry.last_updated = chrono::Utc::now().timestamp();

        let mut quality = self.service_quality.write().await;
        quality.successful_requests += 1;
        quality.total_requests += 1;
        quality.last_updated = chrono::Utc::now().timestamp();

        let mut times = self.request_times.write().await;
        times.push(response_time_ms);

        tracing::info!("Recorded success for advisor: {}, accuracy: {}", entry.advisor_id, accuracy);
        Ok(())
    }

    pub async fn record_advisor_failure(&self, advisor_id: String) -> Result<()> {
        let mut metrics = self.advisor_metrics.write().await;
        let entry = metrics.entry(advisor_id).or_insert(AdvisorMetrics {
            advisor_id: "".to_string(),
            accuracy: 0.0,
            response_time_ms: 0.0,
            success_count: 0,
            failure_count: 0,
            total_requests: 0,
            last_updated: 0,
        });

        entry.failure_count += 1;
        entry.total_requests += 1;
        entry.last_updated = chrono::Utc::now().timestamp();

        let mut quality = self.service_quality.write().await;
        quality.failed_requests += 1;
        quality.total_requests += 1;

        tracing::info!("Recorded failure for advisor: {}", entry.advisor_id);
        Ok(())
    }

    pub async fn calculate_performance_score(&self, advisor_id: &str) -> Result<PerformanceScore> {
        let metrics = self.advisor_metrics.read().await;
        if let Some(advisor) = metrics.get(advisor_id) {
            let reliability = if advisor.total_requests == 0 {
                0.0
            } else {
                advisor.success_count as f32 / advisor.total_requests as f32
            };

            let speed_score = if advisor.response_time_ms == 0.0 {
                1.0
            } else {
                (1000.0 / (advisor.response_time_ms + 1.0)).min(1.0)
            };

            let accuracy_weight = 0.5;
            let speed_weight = 0.25;
            let reliability_weight = 0.25;

            let score = (advisor.accuracy * accuracy_weight) + (speed_score * speed_weight) + (reliability * reliability_weight);

            let performance = PerformanceScore {
                advisor_id: advisor_id.to_string(),
                score: score.clamp(0.0, 1.0),
                accuracy_weight,
                speed_weight,
                reliability_weight,
                calculated_at: chrono::Utc::now().timestamp(),
            };

            let mut scores = self.performance_scores.write().await;
            scores.insert(advisor_id.to_string(), performance.clone());

            Ok(performance)
        } else {
            Err(anyhow::anyhow!("Advisor not found: {}", advisor_id))
        }
    }

    pub async fn get_advisor_metrics(&self, advisor_id: &str) -> Result<Option<AdvisorMetrics>> {
        let metrics = self.advisor_metrics.read().await;
        Ok(metrics.get(advisor_id).cloned())
    }

    pub async fn get_all_metrics(&self) -> Result<Vec<AdvisorMetrics>> {
        let metrics = self.advisor_metrics.read().await;
        Ok(metrics.values().cloned().collect())
    }

    pub async fn get_service_quality(&self) -> Result<ServiceQualityMetrics> {
        let quality = self.service_quality.read().await;
        Ok(quality.clone())
    }

    pub async fn get_top_advisors(&self, limit: usize) -> Result<Vec<PerformanceScore>> {
        let scores = self.performance_scores.read().await;
        let mut sorted: Vec<_> = scores.values().cloned().collect();
        sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        sorted.truncate(limit);
        Ok(sorted)
    }

    pub async fn calculate_average_accuracy(&self) -> Result<f32> {
        let metrics = self.advisor_metrics.read().await;
        if metrics.is_empty() {
            return Ok(0.0);
        }

        let sum: f32 = metrics.values().map(|m| m.accuracy).sum();
        Ok(sum / metrics.len() as f32)
    }

    pub async fn calculate_average_response_time(&self) -> Result<f32> {
        let times = self.request_times.read().await;
        if times.is_empty() {
            return Ok(0.0);
        }

        let sum: f32 = times.iter().sum();
        Ok(sum / times.len() as f32)
    }

    pub async fn reset_metrics(&self) -> Result<()> {
        let mut metrics = self.advisor_metrics.write().await;
        let mut scores = self.performance_scores.write().await;
        let mut times = self.request_times.write().await;

        metrics.clear();
        scores.clear();
        times.clear();

        tracing::info!("Reset all metrics");
        Ok(())
    }

    pub async fn get_accuracy_report(&self) -> Result<HashMap<String, f32>> {
        let metrics = self.advisor_metrics.read().await;
        let mut report = HashMap::new();

        for (id, metric) in metrics.iter() {
            report.insert(id.clone(), metric.accuracy);
        }

        Ok(report)
    }

    pub async fn get_reliability_report(&self) -> Result<HashMap<String, f32>> {
        let metrics = self.advisor_metrics.read().await;
        let mut report = HashMap::new();

        for (id, metric) in metrics.iter() {
            let reliability = if metric.total_requests == 0 {
                0.0
            } else {
                metric.success_count as f32 / metric.total_requests as f32
            };
            report.insert(id.clone(), reliability);
        }

        Ok(report)
    }
}

impl Default for MetricsService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_service_creation() {
        let service = MetricsService::new();
        let quality = service.get_service_quality().await.unwrap();
        assert_eq!(quality.total_requests, 0);
    }

    #[tokio::test]
    async fn test_record_success() {
        let service = MetricsService::new();
        let result = service.record_advisor_success(
            "advisor-1".to_string(),
            0.95,
            50.0
        ).await;
        assert!(result.is_ok());

        let metrics = service.get_advisor_metrics("advisor-1").await.unwrap();
        assert!(metrics.is_some());
    }

    #[tokio::test]
    async fn test_record_failure() {
        let service = MetricsService::new();
        let _ = service.record_advisor_success("advisor-1".to_string(), 0.9, 45.0).await;
        let result = service.record_advisor_failure("advisor-1".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_performance_score() {
        let service = MetricsService::new();
        let _ = service.record_advisor_success("advisor-1".to_string(), 0.9, 50.0).await;

        let score = service.calculate_performance_score("advisor-1").await;
        assert!(score.is_ok());
    }

    #[tokio::test]
    async fn test_accuracy_report() {
        let service = MetricsService::new();
        let _ = service.record_advisor_success("advisor-1".to_string(), 0.95, 50.0).await;

        let report = service.get_accuracy_report().await.unwrap();
        assert!(report.contains_key("advisor-1"));
    }
}
