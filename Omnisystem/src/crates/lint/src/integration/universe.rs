/// Integration with Bonsai Universe event bus.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintEvent {
    pub event_type: LintEventType,
    pub timestamp: SystemTime,
    pub workspace: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LintEventType {
    LintStarted,
    LintCompleted,
    DiagnosticGenerated,
    RuleActivated,
    RuleDeactivated,
}

impl LintEvent {
    pub fn lint_started(workspace: impl Into<String>) -> Self {
        Self {
            event_type: LintEventType::LintStarted,
            timestamp: SystemTime::now(),
            workspace: workspace.into(),
            data: serde_json::json!({}),
        }
    }

    pub fn lint_completed(workspace: impl Into<String>, summary: serde_json::Value) -> Self {
        Self {
            event_type: LintEventType::LintCompleted,
            timestamp: SystemTime::now(),
            workspace: workspace.into(),
            data: summary,
        }
    }
}

pub async fn emit_event(event: LintEvent) {
    // TODO: Send to Universe event bus
    tracing::info!("Lint event: {:?}", event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lint_event_creation() {
        let event = LintEvent::lint_started("my-workspace");
        assert_eq!(event.workspace, "my-workspace");
    }
}
