/// Rule confidence adjuster - applies confidence updates to rule registry
use crate::{RuleConfidenceUpdate, lint_integration::EtlRegistryClient};
use tracing::{debug, info};

/// Applies confidence score updates to the rule registry
pub struct RuleConfidenceAdjuster {
    registry_client: Option<EtlRegistryClient>,
}

impl RuleConfidenceAdjuster {
    pub fn new() -> Self {
        Self {
            registry_client: Some(EtlRegistryClient::new()),
        }
    }

    pub fn with_client(registry_client: EtlRegistryClient) -> Self {
        Self {
            registry_client: Some(registry_client),
        }
    }

    /// Apply a confidence update to a rule
    pub async fn apply_update(&self, update: &RuleConfidenceUpdate) -> anyhow::Result<()> {
        info!(
            "Applying confidence update to rule {}: {:.2} -> {:.2}",
            update.rule_id, update.old_confidence, update.new_confidence
        );

        if let Some(client) = &self.registry_client {
            // Wire to the registry client
            client.apply_update(update).await?;
        }

        // Apply severity changes based on action
        match update.action.as_str() {
            "promote_to_error" => {
                debug!("Promoting rule {} to error severity", update.rule_id);
            }
            "demote_to_hint" => {
                debug!("Demoting rule {} to hint severity", update.rule_id);
            }
            "disable" => {
                debug!("Disabling rule {}", update.rule_id);
            }
            _ => {
                debug!("Keeping rule {} at current severity", update.rule_id);
            }
        }

        Ok(())
    }
}

impl Default for RuleConfidenceAdjuster {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_adjuster_apply_update() {
        let adjuster = RuleConfidenceAdjuster::new();
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

        let result = adjuster.apply_update(&update).await;
        assert!(result.is_ok());
    }
}
