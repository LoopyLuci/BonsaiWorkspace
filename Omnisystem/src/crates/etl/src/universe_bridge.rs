/// Bridge module connecting ETL events to the Universe system event bus.
/// Allows ETL to emit observability events that are consumed by dashboards and monitoring.

use crate::{RuleConfidenceUpdate, refiner::RuleMutationProposal};
use serde::Serialize;

/// Universe event schema that maps to system event bus.
#[derive(Debug, Clone, Serialize)]
pub enum UniverseEvent {
    /// Rule confidence has been updated by ETL.
    RuleConfidenceUpdated {
        rule_id: String,
        old_confidence: f32,
        new_confidence: f32,
        action: String,
    },

    /// A mutation proposal has been generated for a low-confidence rule.
    RuleMutationProposed {
        rule_id: String,
        expected_improvement: f32,
        proposal_id: String,
    },

    /// ETL cycle has started.
    EtlCycleStarted { cycle_id: String },

    /// ETL cycle has completed.
    EtlCycleCompleted {
        cycle_id: String,
        feedback_events_processed: usize,
        rules_updated: usize,
        duration_ms: u64,
    },

    /// ETL cycle failed.
    EtlCycleFailed { cycle_id: String, error: String },

    /// User feedback received on a diagnostic.
    DiagnosticFeedbackReceived {
        rule_id: String,
        file: String,
        line: u32,
        feedback_type: String,
    },
}

/// Bridge to publish ETL events to the Universe system.
pub struct UniverseBridge {
    // In production, this would hold a broadcast channel or reference to SystemEventBus
}

impl UniverseBridge {
    pub fn new() -> Self {
        Self {}
    }

    /// Publish a confidence update event.
    pub async fn publish_confidence_update(
        &self,
        update: &RuleConfidenceUpdate,
    ) -> anyhow::Result<()> {
        let event = UniverseEvent::RuleConfidenceUpdated {
            rule_id: update.rule_id.clone(),
            old_confidence: update.old_confidence,
            new_confidence: update.new_confidence,
            action: update.action.clone(),
        };

        self.publish_event(event).await
    }

    /// Publish a mutation proposal event.
    pub async fn publish_mutation_proposal(
        &self,
        proposal: &RuleMutationProposal,
    ) -> anyhow::Result<()> {
        let event = UniverseEvent::RuleMutationProposed {
            rule_id: proposal.rule_id.clone(),
            expected_improvement: proposal.expected_improvement,
            proposal_id: proposal.proposal_id.clone(),
        };

        self.publish_event(event).await
    }

    /// Publish an ETL cycle started event.
    pub async fn publish_cycle_started(&self, cycle_id: String) -> anyhow::Result<()> {
        let event = UniverseEvent::EtlCycleStarted { cycle_id };
        self.publish_event(event).await
    }

    /// Publish an ETL cycle completed event.
    pub async fn publish_cycle_completed(
        &self,
        cycle_id: String,
        feedback_events_processed: usize,
        rules_updated: usize,
        duration_ms: u64,
    ) -> anyhow::Result<()> {
        let event = UniverseEvent::EtlCycleCompleted {
            cycle_id,
            feedback_events_processed,
            rules_updated,
            duration_ms,
        };

        self.publish_event(event).await
    }

    /// Publish an ETL cycle failed event.
    pub async fn publish_cycle_failed(&self, cycle_id: String, error: String) -> anyhow::Result<()> {
        let event = UniverseEvent::EtlCycleFailed { cycle_id, error };
        self.publish_event(event).await
    }

    /// Publish a diagnostic feedback received event.
    pub async fn publish_feedback_received(
        &self,
        rule_id: String,
        file: String,
        line: u32,
        feedback_type: String,
    ) -> anyhow::Result<()> {
        let event = UniverseEvent::DiagnosticFeedbackReceived {
            rule_id,
            file,
            line,
            feedback_type,
        };

        self.publish_event(event).await
    }

    /// Publish an event to the Universe system.
    async fn publish_event(&self, event: UniverseEvent) -> anyhow::Result<()> {
        // TODO: Wire to actual SystemEventBus when integrated with Tauri
        tracing::info!("Universe event: {:?}", event);
        Ok(())
    }
}

impl Default for UniverseBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_universe_bridge_creation() {
        let bridge = UniverseBridge::new();
        let result = bridge.publish_cycle_started("test-cycle".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_confidence_update_event() {
        let bridge = UniverseBridge::new();
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

        let result = bridge.publish_confidence_update(&update).await;
        assert!(result.is_ok());
    }
}
