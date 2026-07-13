use crate::{ModelPerformance, DataDrift, PredictionDrift, HealthCheck, HealthStatus, AnomalyRecord, MonitoringError, MonitoringResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct ModelMonitor {
    performance_metrics: Arc<DashMap<Uuid, ModelPerformance>>,
    data_drifts: Arc<DashMap<Uuid, DataDrift>>,
    prediction_drifts: Arc<DashMap<Uuid, PredictionDrift>>,
    health_checks: Arc<DashMap<Uuid, HealthCheck>>,
    anomalies: Arc<DashMap<Uuid, AnomalyRecord>>,
}

impl ModelMonitor {
    pub fn new() -> Self {
        Self {
            performance_metrics: Arc::new(DashMap::new()),
            data_drifts: Arc::new(DashMap::new()),
            prediction_drifts: Arc::new(DashMap::new()),
            health_checks: Arc::new(DashMap::new()),
            anomalies: Arc::new(DashMap::new()),
        }
    }

    pub async fn record_performance(&self, perf: &ModelPerformance) -> MonitoringResult<()> {
        self.performance_metrics.insert(perf.perf_id, perf.clone());
        Ok(())
    }

    pub async fn detect_data_drift(&self, model_id: Uuid, feature_name: &str, drift_score: f64) -> MonitoringResult<DataDrift> {
        let drift_detected = drift_score > 0.3;

        let drift = DataDrift {
            drift_id: Uuid::new_v4(),
            model_id,
            feature_name: feature_name.to_string(),
            drift_score,
            drift_detected,
            detected_at: Utc::now(),
        };

        self.data_drifts.insert(drift.drift_id, drift.clone());
        Ok(drift)
    }

    pub async fn detect_prediction_drift(&self, model_id: Uuid, divergence: f64) -> MonitoringResult<PredictionDrift> {
        let drift = PredictionDrift {
            prediction_id: Uuid::new_v4(),
            model_id,
            distribution_divergence: divergence,
            baseline_distribution: "normal".to_string(),
            current_distribution: "shifted".to_string(),
        };

        self.prediction_drifts.insert(drift.prediction_id, drift.clone());
        Ok(drift)
    }

    pub async fn perform_health_check(&self, model_id: Uuid) -> MonitoringResult<HealthCheck> {
        let status = if let Some(perf) = self.get_latest_performance(model_id).await? {
            if perf.accuracy > 0.9 {
                HealthStatus::Healthy
            } else if perf.accuracy > 0.75 {
                HealthStatus::Degraded
            } else {
                HealthStatus::Unhealthy
            }
        } else {
            HealthStatus::Healthy
        };

        let check = HealthCheck {
            check_id: Uuid::new_v4(),
            model_id,
            timestamp: Utc::now(),
            status,
            message: "Health check performed".to_string(),
        };

        self.health_checks.insert(check.check_id, check.clone());
        Ok(check)
    }

    pub async fn detect_anomaly(&self, model_id: Uuid, input_signature: &str, anomaly_score: f64) -> MonitoringResult<AnomalyRecord> {
        let anomaly = AnomalyRecord {
            anomaly_id: Uuid::new_v4(),
            model_id,
            input_signature: input_signature.to_string(),
            anomaly_score,
            detected_at: Utc::now(),
        };

        self.anomalies.insert(anomaly.anomaly_id, anomaly.clone());
        Ok(anomaly)
    }

    pub async fn get_latest_performance(&self, model_id: Uuid) -> MonitoringResult<Option<ModelPerformance>> {
        let mut latest = None;

        for entry in self.performance_metrics.iter() {
            if entry.value().model_id == model_id {
                latest = Some(entry.value().clone());
            }
        }

        Ok(latest)
    }

    pub fn metric_count(&self) -> usize {
        self.performance_metrics.len()
    }
}

impl Default for ModelMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_performance() {
        let monitor = ModelMonitor::new();
        let perf = ModelPerformance {
            perf_id: Uuid::new_v4(),
            model_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            accuracy: 0.95,
            precision: 0.94,
            recall: 0.93,
            f1_score: 0.935,
        };

        monitor.record_performance(&perf).await.unwrap();
        assert_eq!(monitor.metric_count(), 1);
    }

    #[tokio::test]
    async fn test_detect_data_drift() {
        let monitor = ModelMonitor::new();
        let model_id = Uuid::new_v4();

        let drift = monitor.detect_data_drift(model_id, "age", 0.45).await.unwrap();
        assert!(drift.drift_detected);
    }

    #[tokio::test]
    async fn test_detect_prediction_drift() {
        let monitor = ModelMonitor::new();
        let model_id = Uuid::new_v4();

        let drift = monitor.detect_prediction_drift(model_id, 0.55).await.unwrap();
        assert_eq!(drift.distribution_divergence, 0.55);
    }

    #[tokio::test]
    async fn test_perform_health_check() {
        let monitor = ModelMonitor::new();
        let model_id = Uuid::new_v4();

        let perf = ModelPerformance {
            perf_id: Uuid::new_v4(),
            model_id,
            timestamp: Utc::now(),
            accuracy: 0.92,
            precision: 0.91,
            recall: 0.90,
            f1_score: 0.905,
        };

        monitor.record_performance(&perf).await.unwrap();
        let check = monitor.perform_health_check(model_id).await.unwrap();
        assert_eq!(check.status, HealthStatus::Healthy);
    }
}
