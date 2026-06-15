/// ETL Event Schema - Structured events for feedback collection and Universe observability
use crate::refiner::RuleMutationProposal;
use crate::RuleConfidenceUpdate;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FeedbackEventType {
    /// User accepted a diagnostic and applied the fix
    DiagnosticAccepted,
    /// User reported a false positive
    FalsePositiveReported,
    /// User dismissed a diagnostic without action
    DiagnosticDismissed,
    /// User ignored the diagnostic but fixed it manually
    DiagnosticIgnoredThenFixed,
}

/// Core feedback event schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEvent {
    pub event_id: String,
    pub event_type: FeedbackEventType,
    pub rule_id: String,
    pub file: String,
    pub line: u32,
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
    pub action: Option<String>,               // "apply_fix", "report_fp", "dismiss"
    pub outcome: Option<String>,              // "success", "build_failed", "test_failed"
    pub explanation: Option<String>,          // For false positives
    pub dismissal_count: Option<u32>,         // How many times dismissed
}

/// Universe event emitter for observability and event logging
pub struct UniverseEventEmitter {
    bridge: crate::universe_bridge::UniverseBridge,
}

impl UniverseEventEmitter {
    pub fn new() -> Self {
        Self {
            bridge: crate::universe_bridge::UniverseBridge::new(),
        }
    }

    /// Emit a confidence update event to Universe
    pub async fn emit_confidence_update(
        &self,
        update: &RuleConfidenceUpdate,
    ) -> anyhow::Result<()> {
        tracing::info!(
            "Emitting confidence update: rule={} confidence={:.2} action={}",
            update.rule_id,
            update.new_confidence,
            update.action
        );

        self.bridge.publish_confidence_update(update).await
    }

    /// Emit a mutation proposal event to Universe
    pub async fn emit_mutation_proposal(
        &self,
        proposal: &RuleMutationProposal,
    ) -> anyhow::Result<()> {
        tracing::info!(
            "Emitting mutation proposal: rule={} expected_improvement={:.2}",
            proposal.rule_id,
            proposal.expected_improvement
        );

        self.bridge.publish_mutation_proposal(proposal).await
    }

    /// Emit a feedback event to Universe
    pub async fn emit_feedback_event(
        &self,
        event: &FeedbackEvent,
    ) -> anyhow::Result<()> {
        tracing::debug!(
            "Emitting feedback event: rule={} type={:?}",
            event.rule_id,
            event.event_type
        );

        self.bridge
            .publish_feedback_received(
                event.rule_id.clone(),
                event.file.clone(),
                event.line,
                format!("{:?}", event.event_type),
            )
            .await
    }

    /// Emit ETL cycle completion event
    pub async fn emit_cycle_complete(
        &self,
        feedback_processed: usize,
        rules_updated: usize,
        proposals_generated: usize,
    ) -> anyhow::Result<()> {
        tracing::info!(
            "ETL cycle complete: {} feedback events, {} rules updated, {} proposals",
            feedback_processed,
            rules_updated,
            proposals_generated
        );

        let cycle_id = uuid::Uuid::new_v4().to_string();
        self.bridge
            .publish_cycle_completed(cycle_id, feedback_processed, rules_updated, 0)
            .await
    }
}

impl Default for UniverseEventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

/// Universe event schema (for future integration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniverseEvent {
    pub event_id: String,
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub source: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_emit_confidence_update() {
        let emitter = UniverseEventEmitter::new();
        let update = RuleConfidenceUpdate {
            rule_id: "test-rule".to_string(),
            old_confidence: 0.65,
            new_confidence: 0.87,
            action: "promote_to_error".to_string(),
            true_positives: 100,
            false_positives: 10,
            dismissed_count: 5,
            timestamp: Utc::now(),
        };

        let result = emitter.emit_confidence_update(&update).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_emit_feedback_event() {
        let emitter = UniverseEventEmitter::new();
        let event = FeedbackEvent {
            event_id: "test-event".to_string(),
            event_type: FeedbackEventType::DiagnosticAccepted,
            rule_id: "test-rule".to_string(),
            file: "test.rs".to_string(),
            line: 42,
            timestamp: Utc::now(),
            user_id: "user-1".to_string(),
            action: Some("apply".to_string()),
            outcome: Some("success".to_string()),
            explanation: None,
            dismissal_count: None,
        };

        let result = emitter.emit_feedback_event(&event).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_feedback_event_types() {
        assert_ne!(
            FeedbackEventType::DiagnosticAccepted,
            FeedbackEventType::FalsePositiveReported
        );
    }
}
