//! Policy management for the Anti-Hallucination Framework
//!
//! Defines council-controlled policy storage and retrieval with version control.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::AhfError;

/// Versioned policy threshold
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyVersion {
    /// Version number
    pub version: u32,
    /// Threshold value
    pub value: f64,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Council member who set this
    pub set_by: String,
}

/// Arbiter policy with thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbiterPolicy {
    /// Minimum grounding score
    pub grounding_threshold: f64,
    /// Minimum model confidence
    pub confidence_threshold: f64,
    /// Maximum bias score allowed
    pub bias_threshold: f64,
    /// Model-specific policies
    pub per_model_policies: HashMap<String, ModelPolicy>,
}

impl Default for ArbiterPolicy {
    fn default() -> Self {
        ArbiterPolicy {
            grounding_threshold: 0.7,
            confidence_threshold: 0.6,
            bias_threshold: 0.5,
            per_model_policies: HashMap::new(),
        }
    }
}

/// Model-specific policy overrides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPolicy {
    pub model_name: String,
    pub grounding_threshold: Option<f64>,
    pub confidence_threshold: Option<f64>,
    pub bias_threshold: Option<f64>,
}

/// Registry for storing and retrieving policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRegistry {
    /// Current active policy
    pub current_policy: ArbiterPolicy,
    /// History of policy versions
    pub policy_history: Vec<ArbiterPolicy>,
    /// Version tracking
    versions: HashMap<String, Vec<PolicyVersion>>,
}

impl Default for PolicyRegistry {
    fn default() -> Self {
        PolicyRegistry {
            current_policy: ArbiterPolicy::default(),
            policy_history: vec![ArbiterPolicy::default()],
            versions: HashMap::new(),
        }
    }
}

impl PolicyRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the current policy
    pub fn set_policy(&mut self, policy: ArbiterPolicy) -> Result<(), AhfError> {
        self.policy_history.push(self.current_policy.clone());
        self.current_policy = policy;
        Ok(())
    }

    /// Get the current policy
    pub fn get_policy(&self) -> Result<ArbiterPolicy, AhfError> {
        Ok(self.current_policy.clone())
    }

    /// Get policy for a specific model
    pub fn get_model_policy(&self, model_name: &str) -> Result<ArbiterPolicy, AhfError> {
        let mut policy = self.current_policy.clone();

        if let Some(model_policy) = policy.per_model_policies.get(model_name) {
            if let Some(threshold) = model_policy.grounding_threshold {
                policy.grounding_threshold = threshold;
            }
            if let Some(threshold) = model_policy.confidence_threshold {
                policy.confidence_threshold = threshold;
            }
            if let Some(threshold) = model_policy.bias_threshold {
                policy.bias_threshold = threshold;
            }
        }

        Ok(policy)
    }

    /// Set a model-specific policy
    pub fn set_model_policy(&mut self, model_policy: ModelPolicy) -> Result<(), AhfError> {
        let _ = self.current_policy
            .per_model_policies
            .insert(model_policy.model_name.clone(), model_policy);
        Ok(())
    }

    /// Get version history for a threshold
    pub fn get_version_history(&self, key: &str) -> Result<Vec<PolicyVersion>, AhfError> {
        self.versions
            .get(key)
            .cloned()
            .ok_or_else(|| AhfError::policy_not_found(format!("No history for {}", key)))
    }

    /// Record a policy change in history
    pub fn record_change(&mut self, key: String, value: f64, set_by: String) {
        let version = PolicyVersion {
            version: self.versions.get(&key).map(|h| h.len() as u32).unwrap_or(0) + 1,
            value,
            timestamp: chrono::Utc::now(),
            set_by,
        };

        self.versions
            .entry(key)
            .or_insert_with(Vec::new)
            .push(version);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy() {
        let policy = ArbiterPolicy::default();
        assert_eq!(policy.grounding_threshold, 0.7);
        assert_eq!(policy.confidence_threshold, 0.6);
        assert_eq!(policy.bias_threshold, 0.5);
    }

    #[test]
    fn test_policy_registry_creation() {
        let registry = PolicyRegistry::new();
        assert_eq!(registry.policy_history.len(), 1);
        let policy = registry.get_policy().unwrap();
        assert_eq!(policy.grounding_threshold, 0.7);
    }

    #[test]
    fn test_policy_update() {
        let mut registry = PolicyRegistry::new();
        let mut new_policy = ArbiterPolicy::default();
        new_policy.grounding_threshold = 0.8;

        registry.set_policy(new_policy.clone()).unwrap();
        assert_eq!(registry.policy_history.len(), 2);
        let current = registry.get_policy().unwrap();
        assert_eq!(current.grounding_threshold, 0.8);
    }

    #[test]
    fn test_model_policy() {
        let mut registry = PolicyRegistry::new();
        let model_policy = ModelPolicy {
            model_name: "gpt-4".to_string(),
            grounding_threshold: Some(0.9),
            confidence_threshold: None,
            bias_threshold: None,
        };

        registry.set_model_policy(model_policy).unwrap();
        let policy = registry.get_model_policy("gpt-4").unwrap();
        assert_eq!(policy.grounding_threshold, 0.9);
        assert_eq!(policy.confidence_threshold, 0.6); // default
    }

    #[test]
    fn test_version_history() {
        let mut registry = PolicyRegistry::new();
        registry.record_change("grounding_threshold".to_string(), 0.7, "council".to_string());
        registry.record_change("grounding_threshold".to_string(), 0.8, "council".to_string());

        let history = registry.get_version_history("grounding_threshold").unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].value, 0.7);
        assert_eq!(history[1].value, 0.8);
    }
}
