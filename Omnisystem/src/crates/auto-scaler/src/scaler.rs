use crate::{ScalingPolicy, MetricSnapshot, ScalingDecision, ScalingAction, ScalingResult, ScalingError, DemandPrediction};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct AutoScaler {
    policies: Arc<DashMap<String, ScalingPolicy>>,
    metrics: Arc<DashMap<Uuid, MetricSnapshot>>,
    decisions: Arc<DashMap<Uuid, ScalingDecision>>,
}

impl AutoScaler {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(DashMap::new()),
            metrics: Arc::new(DashMap::new()),
            decisions: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_policy(&self, policy: &ScalingPolicy) -> ScalingResult<()> {
        self.policies.insert(policy.service_name.clone(), policy.clone());
        Ok(())
    }

    pub async fn record_metrics(&self, metric: &MetricSnapshot) -> ScalingResult<()> {
        self.metrics.insert(metric.snapshot_id, metric.clone());
        Ok(())
    }

    pub async fn evaluate_scaling(&self, service_name: &str, current_replicas: u32) -> ScalingResult<ScalingDecision> {
        let policy = self.policies
            .get(service_name)
            .map(|p| p.clone())
            .ok_or(ScalingError::PolicyNotFound)?;

        let mut desired_replicas = current_replicas;
        let mut action = ScalingAction::NoChange;
        let mut reason = "Within target metrics".to_string();

        let mut avg_cpu = 0u32;
        let mut avg_memory = 0u32;
        let mut count = 0;

        for entry in self.metrics.iter() {
            let metric = entry.value();
            if metric.service_name == service_name {
                avg_cpu += metric.cpu_percent;
                avg_memory += metric.memory_percent;
                count += 1;
            }
        }

        if count > 0 {
            avg_cpu /= count as u32;
            avg_memory /= count as u32;

            if avg_cpu > policy.target_cpu_percent || avg_memory > policy.target_memory_percent {
                if current_replicas < policy.max_replicas {
                    desired_replicas = (current_replicas + 1).min(policy.max_replicas);
                    action = ScalingAction::ScaleUp;
                    reason = format!("CPU: {}%, Memory: {}%", avg_cpu, avg_memory);
                }
            } else if avg_cpu < (policy.target_cpu_percent / 2) && avg_memory < (policy.target_memory_percent / 2) {
                if current_replicas > policy.min_replicas {
                    desired_replicas = (current_replicas.saturating_sub(1)).max(policy.min_replicas);
                    action = ScalingAction::ScaleDown;
                    reason = "Low utilization, scaling down".to_string();
                }
            }
        }

        let decision = ScalingDecision {
            decision_id: Uuid::new_v4(),
            service_name: service_name.to_string(),
            current_replicas,
            desired_replicas,
            action,
            reason,
        };

        self.decisions.insert(decision.decision_id, decision.clone());
        Ok(decision)
    }

    pub async fn predict_demand(&self, service_name: &str) -> ScalingResult<DemandPrediction> {
        let mut request_count = 0u64;
        let mut metric_count = 0;

        for entry in self.metrics.iter() {
            let metric = entry.value();
            if metric.service_name == service_name {
                request_count += metric.request_count;
                metric_count += 1;
            }
        }

        let predicted_load = if metric_count > 0 {
            (request_count as f32) / (metric_count as f32)
        } else {
            0.0
        };

        let predicted_replicas = ((predicted_load / 1000.0).ceil() as u32).max(1);

        Ok(DemandPrediction {
            prediction_id: Uuid::new_v4(),
            service_name: service_name.to_string(),
            predicted_replicas,
            confidence_percent: 85,
            predicted_load,
        })
    }

    pub fn decision_count(&self) -> usize {
        self.decisions.len()
    }
}

impl Default for AutoScaler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_policy() {
        let scaler = AutoScaler::new();
        let policy = ScalingPolicy {
            policy_id: Uuid::new_v4(),
            service_name: "api-service".to_string(),
            min_replicas: 2,
            max_replicas: 10,
            target_cpu_percent: 75,
            target_memory_percent: 80,
        };

        scaler.register_policy(&policy).await.unwrap();
    }

    #[tokio::test]
    async fn test_record_metrics() {
        let scaler = AutoScaler::new();
        let metric = MetricSnapshot {
            snapshot_id: Uuid::new_v4(),
            service_name: "api-service".to_string(),
            cpu_percent: 50,
            memory_percent: 60,
            request_count: 5000,
            response_time_ms: 150,
        };

        scaler.record_metrics(&metric).await.unwrap();
    }

    #[tokio::test]
    async fn test_evaluate_scaling_scale_up() {
        let scaler = AutoScaler::new();
        let policy = ScalingPolicy {
            policy_id: Uuid::new_v4(),
            service_name: "api".to_string(),
            min_replicas: 2,
            max_replicas: 10,
            target_cpu_percent: 70,
            target_memory_percent: 75,
        };

        scaler.register_policy(&policy).await.unwrap();

        let metric = MetricSnapshot {
            snapshot_id: Uuid::new_v4(),
            service_name: "api".to_string(),
            cpu_percent: 85,
            memory_percent: 90,
            request_count: 10000,
            response_time_ms: 200,
        };

        scaler.record_metrics(&metric).await.unwrap();

        let decision = scaler.evaluate_scaling("api", 2).await.unwrap();
        assert_eq!(decision.action, ScalingAction::ScaleUp);
    }

    #[tokio::test]
    async fn test_predict_demand() {
        let scaler = AutoScaler::new();
        let metric = MetricSnapshot {
            snapshot_id: Uuid::new_v4(),
            service_name: "web".to_string(),
            cpu_percent: 40,
            memory_percent: 50,
            request_count: 5000,
            response_time_ms: 100,
        };

        scaler.record_metrics(&metric).await.unwrap();

        let prediction = scaler.predict_demand("web").await.unwrap();
        assert!(prediction.predicted_replicas > 0);
    }
}
