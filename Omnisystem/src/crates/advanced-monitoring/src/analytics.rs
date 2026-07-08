use crate::{AnalyticsResult, Metric, MonitoringError, MonitoringResult};

pub struct AnalyticsEngine;

impl AnalyticsEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn analyze(&self, metrics: &[Metric]) -> MonitoringResult<AnalyticsResult> {
        if metrics.is_empty() {
            return Err(MonitoringError::AnalysisFailed);
        }

        let values: Vec<f64> = metrics.iter().map(|m| m.value).collect();
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let median = sorted[sorted.len() / 2];
        let p95_idx = ((sorted.len() as f64 * 0.95) as usize).min(sorted.len() - 1);
        let p99_idx = ((sorted.len() as f64 * 0.99) as usize).min(sorted.len() - 1);

        Ok(AnalyticsResult {
            mean,
            median,
            p95: sorted[p95_idx],
            p99: sorted[p99_idx],
            min: sorted[0],
            max: sorted[sorted.len() - 1],
        })
    }
}

impl Default for AnalyticsEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_analyze_metrics() {
        let engine = AnalyticsEngine::new();
        let metrics = vec![
            Metric {
                metric_name: "cpu".to_string(),
                value: 10.0,
                timestamp: Utc::now(),
                labels: std::collections::HashMap::new(),
            },
            Metric {
                metric_name: "cpu".to_string(),
                value: 50.0,
                timestamp: Utc::now(),
                labels: std::collections::HashMap::new(),
            },
            Metric {
                metric_name: "cpu".to_string(),
                value: 90.0,
                timestamp: Utc::now(),
                labels: std::collections::HashMap::new(),
            },
        ];

        let result = engine.analyze(&metrics).await.unwrap();
        assert!(result.mean > 0.0);
    }
}
