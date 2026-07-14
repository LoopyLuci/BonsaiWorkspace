/// Integration module connecting ETL with bonsai-lint RuleRegistry
/// Allows ETL to update rule confidence and severity in the rule registry.

use crate::RuleConfidenceUpdate;

/// Trait for rule registry implementations (allows different backends).
pub trait RuleRegistryClient: Send + Sync {
    /// Update a rule's confidence score.
    fn update_confidence(&self, rule_id: &str, confidence: f32) -> anyhow::Result<()>;

    /// Update a rule's severity.
    fn set_severity(&self, rule_id: &str, severity: &str) -> anyhow::Result<()>;

    /// Enable or disable a rule.
    fn set_enabled(&self, rule_id: &str, enabled: bool) -> anyhow::Result<()>;
}

/// Default implementation that applies ETL updates to the rule registry.
pub struct EtlRegistryClient {
    // In a real deployment, this would hold a reference to the actual RuleRegistry.
    // For now, it's a placeholder that can be filled in when integrated with bonsai-lint.
}

impl EtlRegistryClient {
    pub fn new() -> Self {
        Self {}
    }

    /// Apply a confidence update to the rule registry.
    pub async fn apply_update(&self, update: &RuleConfidenceUpdate) -> anyhow::Result<()> {
        tracing::info!(
            "Applying update to rule {}: confidence {:.2} → {:.2}, action: {}",
            update.rule_id,
            update.old_confidence,
            update.new_confidence,
            update.action
        );

        // TODO: Wire to actual RuleRegistry when integrated
        // self.update_confidence(&update.rule_id, update.new_confidence)?;
        // self.apply_action(&update.rule_id, &update.action)?;

        Ok(())
    }

    fn apply_action(&self, rule_id: &str, action: &str) -> anyhow::Result<()> {
        match action {
            "promote_to_error" => {
                tracing::debug!("Promoting rule {} to error severity", rule_id);
                // self.set_severity(rule_id, "error")?;
            }
            "keep_as_warning" => {
                tracing::debug!("Keeping rule {} at warning severity", rule_id);
            }
            "demote_to_hint" => {
                tracing::debug!("Demoting rule {} to hint severity", rule_id);
                // self.set_severity(rule_id, "hint")?;
            }
            "mark_as_experimental" => {
                tracing::debug!("Marking rule {} as experimental", rule_id);
                // self.set_severity(rule_id, "note")?;
            }
            "disable" => {
                tracing::debug!("Disabling rule {}", rule_id);
                // self.set_enabled(rule_id, false)?;
            }
            _ => {}
        }
        Ok(())
    }
}

impl Default for EtlRegistryClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_etl_registry_client() {
        let client = EtlRegistryClient::new();
        let update = RuleConfidenceUpdate {
            rule_id: "test-rule".to_string(),
            old_confidence: 0.65,
            new_confidence: 0.87,
            action: "promote_to_error".to_string(),
            true_positives: 100,
            false_positives: 10,
            dismissed_count: 5,
            timestamp: chrono::Utc::now(),
        };

        let result = client.apply_update(&update).await;
        assert!(result.is_ok());
    }
}
