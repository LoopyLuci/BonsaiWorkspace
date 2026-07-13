//! Policy enforcement and management for the Arbiter
//!
//! Manages council-controlled policies with version control and per-model overrides.

use crate::{AhfError, DecisionEngine};
use ahf_core::{ArbiterPolicy, PolicyRegistry, ModelPolicy};
use tracing::{debug, info};

/// Enforces council policies on decision thresholds
#[derive(Debug, Clone)]
pub struct PolicyEnforcer {
    /// Policy registry with version history
    registry: PolicyRegistry,
}

impl PolicyEnforcer {
    /// Create a new policy enforcer
    pub fn new() -> Self {
        PolicyEnforcer {
            registry: PolicyRegistry::new(),
        }
    }

    /// Create with custom initial policy
    pub fn with_policy(policy: ArbiterPolicy) -> Self {
        let mut registry = PolicyRegistry::new();
        let _ = registry.set_policy(policy);
        PolicyEnforcer { registry }
    }

    /// Get the current active policy
    pub fn get_policy(&self) -> Result<ArbiterPolicy, AhfError> {
        debug!("Getting current policy");
        self.registry.get_policy()
    }

    /// Get policy for a specific model
    pub fn get_model_policy(&self, model_name: &str) -> Result<ArbiterPolicy, AhfError> {
        debug!(model = model_name, "Getting model-specific policy");
        self.registry.get_model_policy(model_name)
    }

    /// Update the active policy (council action)
    pub fn set_policy(&mut self, policy: ArbiterPolicy, set_by: &str) -> Result<(), AhfError> {
        info!(
            grounding_threshold = policy.grounding_threshold,
            confidence_threshold = policy.confidence_threshold,
            bias_threshold = policy.bias_threshold,
            set_by = set_by,
            "Updating arbiter policy"
        );

        self.registry.set_policy(policy.clone())?;

        // Record the change in history
        self.registry.record_change(
            "grounding_threshold".to_string(),
            policy.grounding_threshold,
            set_by.to_string(),
        );
        self.registry.record_change(
            "confidence_threshold".to_string(),
            policy.confidence_threshold,
            set_by.to_string(),
        );
        self.registry.record_change(
            "bias_threshold".to_string(),
            policy.bias_threshold,
            set_by.to_string(),
        );

        Ok(())
    }

    /// Set a model-specific policy override
    pub fn set_model_policy(&mut self, model_policy: ModelPolicy) -> Result<(), AhfError> {
        info!(
            model = model_policy.model_name,
            "Setting model-specific policy"
        );

        self.registry.set_model_policy(model_policy)
    }

    /// Create a decision engine from current policy
    pub fn create_engine(&self) -> Result<DecisionEngine, AhfError> {
        let policy = self.get_policy()?;
        Ok(DecisionEngine::with_thresholds(
            policy.grounding_threshold,
            policy.confidence_threshold,
            policy.bias_threshold,
        ))
    }

    /// Create a decision engine for a specific model
    pub fn create_model_engine(&self, model_name: &str) -> Result<DecisionEngine, AhfError> {
        let policy = self.get_model_policy(model_name)?;
        Ok(DecisionEngine::with_thresholds(
            policy.grounding_threshold,
            policy.confidence_threshold,
            policy.bias_threshold,
        ))
    }

    /// Get policy change history
    pub fn get_policy_history(
        &self,
        threshold_name: &str,
    ) -> Result<Vec<ahf_core::PolicyVersion>, AhfError> {
        self.registry.get_version_history(threshold_name)
    }

    /// Validate a policy before setting (check for reasonable values)
    pub fn validate_policy(&self, policy: &ArbiterPolicy) -> Result<(), AhfError> {
        if policy.grounding_threshold < 0.0 || policy.grounding_threshold > 1.0 {
            return Err(AhfError::invalid_configuration(
                "Grounding threshold must be in [0.0, 1.0]",
            ));
        }

        if policy.confidence_threshold < 0.0 || policy.confidence_threshold > 1.0 {
            return Err(AhfError::invalid_configuration(
                "Confidence threshold must be in [0.0, 1.0]",
            ));
        }

        if policy.bias_threshold < 0.0 || policy.bias_threshold > 1.0 {
            return Err(AhfError::invalid_configuration(
                "Bias threshold must be in [0.0, 1.0]",
            ));
        }

        Ok(())
    }
}

impl Default for PolicyEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_enforcer_creation() {
        let enforcer = PolicyEnforcer::new();
        let policy = enforcer.get_policy().unwrap();
        assert_eq!(policy.grounding_threshold, 0.7);
        assert_eq!(policy.confidence_threshold, 0.6);
        assert_eq!(policy.bias_threshold, 0.5);
    }

    #[test]
    fn test_update_policy() {
        let mut enforcer = PolicyEnforcer::new();
        let mut new_policy = ArbiterPolicy::default();
        new_policy.grounding_threshold = 0.8;

        enforcer.set_policy(new_policy.clone(), "test_council").unwrap();

        let policy = enforcer.get_policy().unwrap();
        assert_eq!(policy.grounding_threshold, 0.8);
    }

    #[test]
    fn test_model_policy_override() {
        let mut enforcer = PolicyEnforcer::new();
        let model_policy = ModelPolicy {
            model_name: "gpt-4".to_string(),
            grounding_threshold: Some(0.9),
            confidence_threshold: None,
            bias_threshold: None,
        };

        enforcer.set_model_policy(model_policy).unwrap();

        let policy = enforcer.get_model_policy("gpt-4").unwrap();
        assert_eq!(policy.grounding_threshold, 0.9);
        assert_eq!(policy.confidence_threshold, 0.6); // default
    }

    #[test]
    fn test_create_engine_from_policy() {
        let mut enforcer = PolicyEnforcer::new();
        let mut new_policy = ArbiterPolicy::default();
        new_policy.grounding_threshold = 0.75;
        new_policy.confidence_threshold = 0.65;

        enforcer.set_policy(new_policy, "test_council").unwrap();

        let engine = enforcer.create_engine().unwrap();
        assert_eq!(engine.grounding_threshold, 0.75);
        assert_eq!(engine.confidence_threshold, 0.65);
    }

    #[test]
    fn test_model_engine_creation() {
        let mut enforcer = PolicyEnforcer::new();
        let model_policy = ModelPolicy {
            model_name: "claude-3".to_string(),
            grounding_threshold: Some(0.85),
            confidence_threshold: Some(0.75),
            bias_threshold: None,
        };

        enforcer.set_model_policy(model_policy).unwrap();

        let engine = enforcer.create_model_engine("claude-3").unwrap();
        assert_eq!(engine.grounding_threshold, 0.85);
        assert_eq!(engine.confidence_threshold, 0.75);
    }

    #[test]
    fn test_policy_history() {
        let mut enforcer = PolicyEnforcer::new();
        let mut policy1 = ArbiterPolicy::default();
        policy1.grounding_threshold = 0.7;
        enforcer.set_policy(policy1, "council_v1").unwrap();

        let mut policy2 = ArbiterPolicy::default();
        policy2.grounding_threshold = 0.8;
        enforcer.set_policy(policy2, "council_v2").unwrap();

        let history = enforcer
            .get_policy_history("grounding_threshold")
            .unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].value, 0.7);
        assert_eq!(history[1].value, 0.8);
    }

    #[test]
    fn test_validate_policy() {
        let enforcer = PolicyEnforcer::new();

        let mut valid_policy = ArbiterPolicy::default();
        assert!(enforcer.validate_policy(&valid_policy).is_ok());

        let mut invalid_policy = ArbiterPolicy::default();
        invalid_policy.grounding_threshold = 1.5;
        assert!(enforcer.validate_policy(&invalid_policy).is_err());
    }
}
