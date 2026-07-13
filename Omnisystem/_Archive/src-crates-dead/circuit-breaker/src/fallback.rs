use crate::{CircuitBreakerError, CircuitBreakerResult, FallbackPolicy};
use dashmap::DashMap;
use std::sync::Arc;

pub struct FallbackManager {
    policies: Arc<DashMap<String, FallbackPolicy>>,
}

impl FallbackManager {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_fallback(&self, policy: &FallbackPolicy) -> CircuitBreakerResult<()> {
        self.policies.insert(policy.policy_id.clone(), policy.clone());
        Ok(())
    }

    pub async fn enable_fallback(&self, policy_id: &str) -> CircuitBreakerResult<()> {
        if let Some(mut policy) = self.policies.get_mut(policy_id) {
            policy.fallback_enabled = true;
            Ok(())
        } else {
            Err(CircuitBreakerError::InvalidConfiguration)
        }
    }

    pub async fn disable_fallback(&self, policy_id: &str) -> CircuitBreakerResult<()> {
        if let Some(mut policy) = self.policies.get_mut(policy_id) {
            policy.fallback_enabled = false;
            Ok(())
        } else {
            Err(CircuitBreakerError::InvalidConfiguration)
        }
    }

    pub async fn is_fallback_enabled(&self, policy_id: &str) -> CircuitBreakerResult<bool> {
        if let Some(policy) = self.policies.get(policy_id) {
            Ok(policy.fallback_enabled)
        } else {
            Err(CircuitBreakerError::InvalidConfiguration)
        }
    }

    pub async fn get_policy(&self, policy_id: &str) -> CircuitBreakerResult<FallbackPolicy> {
        self.policies
            .get(policy_id)
            .map(|entry| entry.clone())
            .ok_or(CircuitBreakerError::InvalidConfiguration)
    }

    pub async fn update_success_rate(
        &self,
        policy_id: &str,
        success_rate: f64,
    ) -> CircuitBreakerResult<()> {
        if let Some(mut policy) = self.policies.get_mut(policy_id) {
            policy.fallback_success_rate = success_rate.max(0.0).min(1.0);
            Ok(())
        } else {
            Err(CircuitBreakerError::InvalidConfiguration)
        }
    }

    pub async fn get_success_rate(&self, policy_id: &str) -> CircuitBreakerResult<f64> {
        if let Some(policy) = self.policies.get(policy_id) {
            Ok(policy.fallback_success_rate)
        } else {
            Err(CircuitBreakerError::InvalidConfiguration)
        }
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }
}

impl Default for FallbackManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_fallback() {
        let manager = FallbackManager::new();
        let policy = FallbackPolicy {
            policy_id: "policy-1".to_string(),
            fallback_enabled: true,
            fallback_timeout_ms: 5000,
            max_fallback_calls: 10,
            fallback_success_rate: 0.95,
        };

        manager.register_fallback(&policy).await.unwrap();
        assert_eq!(manager.policy_count(), 1);
    }

    #[tokio::test]
    async fn test_enable_fallback() {
        let manager = FallbackManager::new();
        let policy = FallbackPolicy {
            policy_id: "policy-1".to_string(),
            fallback_enabled: false,
            fallback_timeout_ms: 5000,
            max_fallback_calls: 10,
            fallback_success_rate: 0.95,
        };

        manager.register_fallback(&policy).await.unwrap();
        manager.enable_fallback("policy-1").await.unwrap();

        let enabled = manager.is_fallback_enabled("policy-1").await.unwrap();
        assert!(enabled);
    }

    #[tokio::test]
    async fn test_disable_fallback() {
        let manager = FallbackManager::new();
        let policy = FallbackPolicy {
            policy_id: "policy-1".to_string(),
            fallback_enabled: true,
            fallback_timeout_ms: 5000,
            max_fallback_calls: 10,
            fallback_success_rate: 0.95,
        };

        manager.register_fallback(&policy).await.unwrap();
        manager.disable_fallback("policy-1").await.unwrap();

        let enabled = manager.is_fallback_enabled("policy-1").await.unwrap();
        assert!(!enabled);
    }

    #[tokio::test]
    async fn test_update_success_rate() {
        let manager = FallbackManager::new();
        let policy = FallbackPolicy {
            policy_id: "policy-1".to_string(),
            fallback_enabled: true,
            fallback_timeout_ms: 5000,
            max_fallback_calls: 10,
            fallback_success_rate: 0.95,
        };

        manager.register_fallback(&policy).await.unwrap();
        manager.update_success_rate("policy-1", 0.99).await.unwrap();

        let success_rate = manager.get_success_rate("policy-1").await.unwrap();
        assert_eq!(success_rate, 0.99);
    }

    #[tokio::test]
    async fn test_get_policy() {
        let manager = FallbackManager::new();
        let policy = FallbackPolicy {
            policy_id: "policy-1".to_string(),
            fallback_enabled: true,
            fallback_timeout_ms: 5000,
            max_fallback_calls: 10,
            fallback_success_rate: 0.95,
        };

        manager.register_fallback(&policy).await.unwrap();
        let retrieved = manager.get_policy("policy-1").await.unwrap();

        assert_eq!(retrieved.policy_id, "policy-1");
    }

    #[tokio::test]
    async fn test_policy_not_found() {
        let manager = FallbackManager::new();
        let result = manager.get_policy("nonexistent").await;

        assert!(result.is_err());
    }
}
