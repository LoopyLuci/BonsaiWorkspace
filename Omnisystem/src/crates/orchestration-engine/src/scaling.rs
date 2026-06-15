use crate::{DeploymentId, OrchestrationResult, ScalingPolicy};
use dashmap::DashMap;
use std::sync::Arc;

pub struct AutoScaler {
    policies: Arc<DashMap<String, ScalingPolicy>>,
    last_scale_time: Arc<DashMap<String, chrono::DateTime<chrono::Utc>>>,
}

impl AutoScaler {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(DashMap::new()),
            last_scale_time: Arc::new(DashMap::new()),
        }
    }

    pub async fn set_scaling_policy(
        &self,
        deployment_id: &DeploymentId,
        policy: &ScalingPolicy,
    ) -> OrchestrationResult<()> {
        self.policies.insert(deployment_id.0.clone(), policy.clone());
        Ok(())
    }

    pub async fn get_scaling_policy(
        &self,
        deployment_id: &DeploymentId,
    ) -> OrchestrationResult<ScalingPolicy> {
        self.policies
            .get(&deployment_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| crate::OrchestrationError::ConfigurationError(
                "Scaling policy not found".to_string(),
            ))
    }

    pub async fn evaluate_scaling(
        &self,
        deployment_id: &DeploymentId,
        current_replicas: u32,
        cpu_usage_percent: f64,
        memory_usage_percent: f64,
    ) -> OrchestrationResult<Option<u32>> {
        let policy = self.get_scaling_policy(deployment_id).await?;

        let now = chrono::Utc::now();
        let last_scale = self
            .last_scale_time
            .get(&deployment_id.0)
            .map(|entry| *entry);

        if cpu_usage_percent > policy.scale_up_threshold * 100.0
            || memory_usage_percent > policy.scale_up_threshold * 100.0
        {
            if let Some(last_time) = last_scale {
                let elapsed = (now - last_time).num_seconds() as u64;
                if elapsed < policy.scale_up_cooldown_secs {
                    return Ok(None);
                }
            }

            let new_replicas = std::cmp::min(current_replicas + 1, policy.max_replicas);
            if new_replicas > current_replicas {
                self.last_scale_time.insert(deployment_id.0.clone(), now);
                return Ok(Some(new_replicas));
            }
        } else if cpu_usage_percent < policy.scale_down_threshold * 100.0
            && memory_usage_percent < policy.scale_down_threshold * 100.0
        {
            if let Some(last_time) = last_scale {
                let elapsed = (now - last_time).num_seconds() as u64;
                if elapsed < policy.scale_down_cooldown_secs {
                    return Ok(None);
                }
            }

            let new_replicas = std::cmp::max(current_replicas.saturating_sub(1), policy.min_replicas);
            if new_replicas < current_replicas {
                self.last_scale_time.insert(deployment_id.0.clone(), now);
                return Ok(Some(new_replicas));
            }
        }

        Ok(None)
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
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
    async fn test_set_scaling_policy() {
        let scaler = AutoScaler::new();
        let deployment_id = DeploymentId("test-deployment".to_string());
        let policy = ScalingPolicy::default();

        scaler.set_scaling_policy(&deployment_id, &policy).await.unwrap();
        assert_eq!(scaler.policy_count(), 1);
    }

    #[tokio::test]
    async fn test_get_scaling_policy() {
        let scaler = AutoScaler::new();
        let deployment_id = DeploymentId("test-deployment".to_string());
        let policy = ScalingPolicy::default();

        scaler.set_scaling_policy(&deployment_id, &policy).await.unwrap();
        let retrieved = scaler.get_scaling_policy(&deployment_id).await.unwrap();

        assert_eq!(retrieved.target_cpu_percent, policy.target_cpu_percent);
    }

    #[tokio::test]
    async fn test_scale_up_decision() {
        let scaler = AutoScaler::new();
        let deployment_id = DeploymentId("test-deployment".to_string());
        let policy = ScalingPolicy::default();

        scaler.set_scaling_policy(&deployment_id, &policy).await.unwrap();

        let result = scaler
            .evaluate_scaling(&deployment_id, 2, 85.0, 70.0)
            .await
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_scale_down_decision() {
        let scaler = AutoScaler::new();
        let deployment_id = DeploymentId("test-deployment".to_string());
        let policy = ScalingPolicy::default();

        scaler.set_scaling_policy(&deployment_id, &policy).await.unwrap();

        let result = scaler
            .evaluate_scaling(&deployment_id, 5, 20.0, 25.0)
            .await
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap(), 4);
    }

    #[tokio::test]
    async fn test_no_scaling_decision() {
        let scaler = AutoScaler::new();
        let deployment_id = DeploymentId("test-deployment".to_string());
        let policy = ScalingPolicy::default();

        scaler.set_scaling_policy(&deployment_id, &policy).await.unwrap();

        let result = scaler
            .evaluate_scaling(&deployment_id, 3, 50.0, 50.0)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_max_replicas_limit() {
        let scaler = AutoScaler::new();
        let deployment_id = DeploymentId("test-deployment".to_string());
        let policy = ScalingPolicy::default();

        scaler.set_scaling_policy(&deployment_id, &policy).await.unwrap();

        let result = scaler
            .evaluate_scaling(&deployment_id, 10, 85.0, 70.0)
            .await
            .unwrap();

        assert!(result.is_none()); // Should not scale beyond max_replicas
    }

    #[tokio::test]
    async fn test_min_replicas_limit() {
        let scaler = AutoScaler::new();
        let deployment_id = DeploymentId("test-deployment".to_string());
        let policy = ScalingPolicy::default();

        scaler.set_scaling_policy(&deployment_id, &policy).await.unwrap();

        let result = scaler
            .evaluate_scaling(&deployment_id, 1, 20.0, 25.0)
            .await
            .unwrap();

        assert!(result.is_none()); // Should not scale below min_replicas
    }
}
