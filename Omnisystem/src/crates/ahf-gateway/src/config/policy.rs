//! Policy configuration for the Anti-Hallucination Gateway

use crate::error::{GatewayError, GatewayResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Council-signed policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Policy version
    pub version: String,

    /// Grounding score decision thresholds
    pub grounding_accept_threshold: f64,
    pub grounding_escalate_threshold: f64,

    /// Confidence score thresholds
    pub confidence_min_threshold: f64,
    pub confidence_reject_threshold: f64,

    /// Bias score thresholds
    pub bias_warning_threshold: f64,
    pub bias_reject_threshold: f64,

    /// Per-model overrides (model_id -> ModelPolicy)
    pub model_overrides: HashMap<String, ModelPolicy>,

    /// Criticality-based escalation rules
    pub criticality_rules: CriticalityRules,

    /// Whether to enable safety envelopes
    pub enable_safety_envelopes: bool,

    /// Maximum false rejection rate allowed (for alerting)
    pub max_false_rejection_rate: f64,

    /// Policy signature (for council verification)
    pub signature: Option<String>,

    /// Timestamp of policy creation
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            grounding_accept_threshold: 0.8,
            grounding_escalate_threshold: 0.6,
            confidence_min_threshold: 0.5,
            confidence_reject_threshold: 0.3,
            bias_warning_threshold: 0.4,
            bias_reject_threshold: 0.7,
            model_overrides: HashMap::new(),
            criticality_rules: CriticalityRules::default(),
            enable_safety_envelopes: true,
            max_false_rejection_rate: 0.02,
            signature: None,
            created_at: chrono::Utc::now(),
        }
    }
}

/// Per-model policy overrides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPolicy {
    pub model_id: String,
    pub grounding_threshold: Option<f64>,
    pub confidence_threshold: Option<f64>,
    pub bias_threshold: Option<f64>,
    pub enable_shadow_mode: Option<bool>,
}

/// Criticality-based escalation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalityRules {
    /// Low criticality: always accept if grounding_score > 0.5
    pub low_accept_threshold: f64,
    /// Medium criticality: require grounding_score > 0.7
    pub medium_accept_threshold: f64,
    /// High criticality: require grounding_score > 0.8 or escalate
    pub high_accept_threshold: f64,
    /// Critical: always escalate if any doubt
    pub critical_accept_threshold: f64,
}

impl Default for CriticalityRules {
    fn default() -> Self {
        Self {
            low_accept_threshold: 0.5,
            medium_accept_threshold: 0.7,
            high_accept_threshold: 0.8,
            critical_accept_threshold: 0.95,
        }
    }
}

impl PolicyConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate policy configuration
    pub fn validate(&self) -> GatewayResult<()> {
        // Validate main thresholds
        if self.grounding_accept_threshold < 0.0 || self.grounding_accept_threshold > 1.0 {
            return Err(GatewayError::config_error(
                "grounding_accept_threshold must be between 0.0 and 1.0",
            ));
        }

        if self.grounding_escalate_threshold > self.grounding_accept_threshold {
            return Err(GatewayError::config_error(
                "grounding_escalate_threshold must be <= grounding_accept_threshold",
            ));
        }

        if self.confidence_min_threshold < 0.0 || self.confidence_min_threshold > 1.0 {
            return Err(GatewayError::config_error(
                "confidence_min_threshold must be between 0.0 and 1.0",
            ));
        }

        if self.bias_warning_threshold > self.bias_reject_threshold {
            return Err(GatewayError::config_error(
                "bias_warning_threshold must be <= bias_reject_threshold",
            ));
        }

        if self.max_false_rejection_rate < 0.0 || self.max_false_rejection_rate > 1.0 {
            return Err(GatewayError::config_error(
                "max_false_rejection_rate must be between 0.0 and 1.0",
            ));
        }

        // Validate model overrides
        for (_, model_policy) in &self.model_overrides {
            if let Some(threshold) = model_policy.grounding_threshold {
                if threshold < 0.0 || threshold > 1.0 {
                    return Err(GatewayError::config_error(
                        "model override grounding_threshold must be between 0.0 and 1.0",
                    ));
                }
            }
        }

        // Validate criticality rules
        if self.criticality_rules.low_accept_threshold > self.criticality_rules.medium_accept_threshold
            || self.criticality_rules.medium_accept_threshold > self.criticality_rules.high_accept_threshold
            || self.criticality_rules.high_accept_threshold > self.criticality_rules.critical_accept_threshold
        {
            return Err(GatewayError::config_error(
                "criticality thresholds must be in ascending order",
            ));
        }

        Ok(())
    }

    /// Get threshold for a specific model
    pub fn get_grounding_threshold(&self, model_id: &str) -> f64 {
        self.model_overrides
            .get(model_id)
            .and_then(|p| p.grounding_threshold)
            .unwrap_or(self.grounding_accept_threshold)
    }

    /// Get confidence threshold for a specific model
    pub fn get_confidence_threshold(&self, model_id: &str) -> f64 {
        self.model_overrides
            .get(model_id)
            .and_then(|p| p.confidence_threshold)
            .unwrap_or(self.confidence_min_threshold)
    }

    /// Get bias threshold for a specific model
    pub fn get_bias_threshold(&self, model_id: &str) -> f64 {
        self.model_overrides
            .get(model_id)
            .and_then(|p| p.bias_threshold)
            .unwrap_or(self.bias_reject_threshold)
    }

    /// Check if shadow mode is enabled for a model
    pub fn is_shadow_mode_enabled(&self, model_id: &str) -> bool {
        self.model_overrides
            .get(model_id)
            .and_then(|p| p.enable_shadow_mode)
            .unwrap_or(false)
    }

    /// Add model override
    pub fn add_model_override(&mut self, policy: ModelPolicy) {
        self.model_overrides.insert(policy.model_id.clone(), policy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_default() {
        let policy = PolicyConfig::default();
        assert_eq!(policy.version, "1.0.0");
        assert_eq!(policy.grounding_accept_threshold, 0.8);
    }

    #[test]
    fn test_policy_validation() {
        let mut policy = PolicyConfig::default();
        assert!(policy.validate().is_ok());

        policy.grounding_accept_threshold = 1.5;
        assert!(policy.validate().is_err());

        policy.grounding_accept_threshold = 0.8;
        policy.grounding_escalate_threshold = 0.9;
        assert!(policy.validate().is_err());
    }

    #[test]
    fn test_model_overrides() {
        let mut policy = PolicyConfig::default();
        let model_policy = ModelPolicy {
            model_id: "gpt-4".to_string(),
            grounding_threshold: Some(0.85),
            confidence_threshold: Some(0.7),
            bias_threshold: None,
            enable_shadow_mode: Some(true),
        };

        policy.add_model_override(model_policy);

        assert_eq!(policy.get_grounding_threshold("gpt-4"), 0.85);
        assert_eq!(policy.get_grounding_threshold("other"), 0.8);
        assert_eq!(policy.get_confidence_threshold("gpt-4"), 0.7);
        assert!(policy.is_shadow_mode_enabled("gpt-4"));
    }

    #[test]
    fn test_criticality_rules() {
        let rules = CriticalityRules::default();
        assert!(rules.low_accept_threshold < rules.medium_accept_threshold);
        assert!(rules.medium_accept_threshold < rules.high_accept_threshold);
        assert!(rules.high_accept_threshold < rules.critical_accept_threshold);
    }

    #[test]
    fn test_policy_serialization() {
        let policy = PolicyConfig::default();
        let json = serde_json::to_string(&policy).expect("serialization failed");
        let policy2: PolicyConfig = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(policy.version, policy2.version);
    }
}
