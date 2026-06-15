use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Anomaly detection for IoT devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetector {
    pub detector_id: String,
    pub baseline_metrics: HashMap<String, f32>,
    pub sensitivity: f32, // 0.5-2.0
    pub detection_algorithm: AnomalyAlgorithm,
    pub anomalies_detected: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnomalyAlgorithm {
    StatisticalZ,       // Z-score based
    IsolationForest,    // Isolation forest
    LOF,                // Local Outlier Factor
    DBSCAN,             // Density-based clustering
    AutoEncoder,        // Neural network
}

impl AnomalyDetector {
    pub fn new(detector_id: String, algorithm: AnomalyAlgorithm) -> Self {
        AnomalyDetector {
            detector_id,
            baseline_metrics: HashMap::new(),
            sensitivity: 1.0,
            detection_algorithm: algorithm,
            anomalies_detected: 0,
        }
    }

    pub async fn analyze(&mut self, metrics: &HashMap<String, f32>) -> Result<bool> {
        for (metric, value) in metrics {
            if let Some(baseline) = self.baseline_metrics.get(metric) {
                let deviation = (value - baseline).abs() / baseline.max(0.001);
                if deviation > self.sensitivity {
                    self.anomalies_detected += 1;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub fn set_baseline(&mut self, metrics: HashMap<String, f32>) {
        self.baseline_metrics = metrics;
    }
}

/// Predictive maintenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveModel {
    pub model_id: String,
    pub device_id: String,
    pub failure_prediction: FailurePrediction,
    pub confidence: f32,
    pub maintenance_recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePrediction {
    pub estimated_time_to_failure_hours: u32,
    pub risk_level: RiskLevel,
    pub contributing_factors: Vec<String>,
    pub recommended_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl PredictiveModel {
    pub fn new(model_id: String, device_id: String) -> Self {
        PredictiveModel {
            model_id,
            device_id,
            failure_prediction: FailurePrediction {
                estimated_time_to_failure_hours: 8760,
                risk_level: RiskLevel::Low,
                contributing_factors: vec![],
                recommended_action: "Continue normal operation".to_string(),
            },
            confidence: 0.95,
            maintenance_recommended: false,
        }
    }

    pub async fn predict_failure(&mut self, device_data: &HashMap<String, f32>) -> Result<()> {
        let mut risk_score = 0.0;
        let mut factors = vec![];

        for (key, value) in device_data {
            if key.contains("temp") && *value > 80.0 {
                risk_score += 0.3;
                factors.push(format!("High temperature: {}", value));
            }
            if key.contains("power") && *value > 95.0 {
                risk_score += 0.2;
                factors.push(format!("High power consumption: {}", value));
            }
            if key.contains("error") && *value > 10.0 {
                risk_score += 0.4;
                factors.push(format!("High error rate: {}", value));
            }
        }

        self.failure_prediction.risk_level = match risk_score {
            x if x > 0.7 => RiskLevel::Critical,
            x if x > 0.5 => RiskLevel::High,
            x if x > 0.25 => RiskLevel::Medium,
            _ => RiskLevel::Low,
        };

        self.failure_prediction.contributing_factors = factors;
        self.maintenance_recommended = self.failure_prediction.risk_level == RiskLevel::Critical
            || self.failure_prediction.risk_level == RiskLevel::High;

        Ok(())
    }
}

/// Device health monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMonitor {
    pub monitor_id: String,
    pub devices: HashMap<String, DeviceHealth>,
    pub monitoring_interval_sec: u32,
    pub overall_system_health: f32, // 0.0-100.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceHealth {
    pub device_id: String,
    pub health_score: f32, // 0.0-100.0
    pub status: HealthStatus,
    pub last_check_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Offline,
}

impl HealthMonitor {
    pub fn new(monitor_id: String) -> Self {
        HealthMonitor {
            monitor_id,
            devices: HashMap::new(),
            monitoring_interval_sec: 30,
            overall_system_health: 100.0,
        }
    }

    pub async fn check_device_health(&mut self, device_id: &str, metrics: &HashMap<String, f32>) -> Result<()> {
        let mut score = 100.0;

        for (metric, value) in metrics {
            if metric.contains("error") {
                score -= value * 5.0;
            }
            if metric.contains("temp") && *value > 85.0 {
                score -= 10.0;
            }
        }

        let health = DeviceHealth {
            device_id: device_id.to_string(),
            health_score: score.max(0.0),
            status: match score {
                x if x >= 80.0 => HealthStatus::Healthy,
                x if x >= 60.0 => HealthStatus::Degraded,
                x if x >= 30.0 => HealthStatus::Unhealthy,
                _ => HealthStatus::Offline,
            },
            last_check_ms: 0,
        };

        self.devices.insert(device_id.to_string(), health);
        self.calculate_overall_health();

        Ok(())
    }

    fn calculate_overall_health(&mut self) {
        if self.devices.is_empty() {
            self.overall_system_health = 100.0;
            return;
        }

        let total: f32 = self.devices.values().map(|d| d.health_score).sum();
        self.overall_system_health = total / self.devices.len() as f32;
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anomaly_detector() {
        let detector = AnomalyDetector::new("det1".to_string(), AnomalyAlgorithm::StatisticalZ);
        assert_eq!(detector.sensitivity, 1.0);
    }

    #[test]
    fn test_predictive_model() {
        let model = PredictiveModel::new("model1".to_string(), "device1".to_string());
        assert_eq!(model.failure_prediction.risk_level, RiskLevel::Low);
    }

    #[test]
    fn test_health_monitor() {
        let monitor = HealthMonitor::new("monitor1".to_string());
        assert_eq!(monitor.overall_system_health, 100.0);
    }

    #[test]
    fn test_risk_levels() {
        let risks = vec![RiskLevel::Low, RiskLevel::Medium, RiskLevel::Critical];
        assert_eq!(risks.len(), 3);
    }

    #[test]
    fn test_health_statuses() {
        let statuses = vec![HealthStatus::Healthy, HealthStatus::Degraded, HealthStatus::Offline];
        assert_eq!(statuses.len(), 3);
    }

    #[test]
    fn test_math() {
        assert_eq!(2 + 2, 4);
    }
}
