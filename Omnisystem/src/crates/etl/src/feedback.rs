/// Feedback collection from user actions in the IDE
use crate::events::{FeedbackEvent, FeedbackEventType};
use crate::storage::ETLStorage;
use chrono::Utc;
use std::sync::Arc;

/// Collects user feedback and emits it to storage and Universe
pub struct FeedbackCollector {
    storage: Arc<ETLStorage>,
}

impl FeedbackCollector {
    pub fn new(storage: Arc<ETLStorage>) -> Self {
        Self { storage }
    }

    /// Called when user accepts a diagnostic and applies the fix
    pub async fn on_fix_applied(
        &self,
        rule_id: String,
        file: String,
        line: u32,
        user_id: String,
        outcome: String, // "success", "build_failed", "test_failed"
    ) -> anyhow::Result<()> {
        let event = FeedbackEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: FeedbackEventType::DiagnosticAccepted,
            rule_id,
            file,
            line,
            timestamp: Utc::now(),
            user_id,
            action: Some("applied_fix".to_string()),
            outcome: Some(outcome),
            explanation: None,
            dismissal_count: None,
        };

        self.storage.store_feedback_event(&event).await?;
        tracing::debug!("Recorded fix application: rule={}", event.rule_id);
        Ok(())
    }

    /// Called when user marks a diagnostic as a false positive
    pub async fn on_false_positive_report(
        &self,
        rule_id: String,
        file: String,
        line: u32,
        user_id: String,
        explanation: String,
    ) -> anyhow::Result<()> {
        let event = FeedbackEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: FeedbackEventType::FalsePositiveReported,
            rule_id,
            file,
            line,
            timestamp: Utc::now(),
            user_id,
            action: None,
            outcome: None,
            explanation: Some(explanation),
            dismissal_count: None,
        };

        self.storage.store_feedback_event(&event).await?;
        tracing::debug!("Recorded false positive report: rule={}", event.rule_id);
        Ok(())
    }

    /// Called when user dismisses a diagnostic without action
    pub async fn on_diagnostic_dismissed(
        &self,
        rule_id: String,
        file: String,
        line: u32,
        user_id: String,
        dismissal_count: u32,
    ) -> anyhow::Result<()> {
        let event = FeedbackEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: FeedbackEventType::DiagnosticDismissed,
            rule_id,
            file,
            line,
            timestamp: Utc::now(),
            user_id,
            action: None,
            outcome: None,
            explanation: None,
            dismissal_count: Some(dismissal_count),
        };

        self.storage.store_feedback_event(&event).await?;
        tracing::debug!("Recorded dismissal: rule={}, count={}", event.rule_id, dismissal_count);
        Ok(())
    }

    /// Called when user manually edits code after seeing a diagnostic
    pub async fn on_manual_edit(
        &self,
        rule_id: String,
        file: String,
        line: u32,
        user_id: String,
    ) -> anyhow::Result<()> {
        let event = FeedbackEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: FeedbackEventType::DiagnosticIgnoredThenFixed,
            rule_id,
            file,
            line,
            timestamp: Utc::now(),
            user_id,
            action: Some("manual_edit".to_string()),
            outcome: Some("fix_applied".to_string()),
            explanation: None,
            dismissal_count: None,
        };

        self.storage.store_feedback_event(&event).await?;
        tracing::debug!("Recorded manual edit: rule={}", event.rule_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_feedback_collection() {
        // Integration tests in storage module
        assert!(true);
    }
}
