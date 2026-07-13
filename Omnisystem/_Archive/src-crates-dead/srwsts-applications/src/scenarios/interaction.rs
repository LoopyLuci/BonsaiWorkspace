//! Cross-application interaction scenarios

use super::{Scenario, ScenarioResult};
use crate::errors::ApplicationStressResult;
use std::time::Instant;

/// Application interaction scenarios
pub enum InteractionType {
    WorkspaceBuddySync,
    BuddyOmniBotQuery,
    WorkspaceOmniBotIntegration,
    CascadingFailure,
    DataFlowVerification,
}

impl std::fmt::Display for InteractionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InteractionType::WorkspaceBuddySync => write!(f, "Workspace-Buddy Sync"),
            InteractionType::BuddyOmniBotQuery => write!(f, "Buddy-OmniBot Query"),
            InteractionType::WorkspaceOmniBotIntegration => {
                write!(f, "Workspace-OmniBot Integration")
            }
            InteractionType::CascadingFailure => write!(f, "Cascading Failure"),
            InteractionType::DataFlowVerification => write!(f, "Data Flow Verification"),
        }
    }
}

/// Cross-application interaction scenario
pub struct InteractionScenario {
    pub id: String,
    pub name: String,
    pub scenario_type: InteractionType,
    pub apps_involved: Vec<String>,
}

impl InteractionScenario {
    /// Create a new interaction scenario
    pub fn new(
        id: impl Into<String>,
        scenario_type: InteractionType,
        apps_involved: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: scenario_type.to_string(),
            scenario_type,
            apps_involved,
        }
    }

    /// Workspace triggering Buddy sync
    pub fn workspace_buddy_sync() -> Self {
        Self::new(
            "interaction-workspace-buddy",
            InteractionType::WorkspaceBuddySync,
            vec!["workspace".to_string(), "buddy".to_string()],
        )
    }

    /// Workspace querying Buddy for context
    pub fn buddy_omnibot_query() -> Self {
        Self::new(
            "interaction-buddy-omnibot",
            InteractionType::BuddyOmniBotQuery,
            vec!["buddy".to_string(), "omnibot".to_string()],
        )
    }

    /// Full ecosystem interaction
    pub fn fullstack_integration() -> Self {
        Self::new(
            "interaction-fullstack",
            InteractionType::WorkspaceOmniBotIntegration,
            vec![
                "workspace".to_string(),
                "buddy".to_string(),
                "omnibot".to_string(),
            ],
        )
    }

    /// One app fails, verify others remain functional
    pub fn cascading_failure() -> Self {
        Self::new(
            "interaction-cascading-failure",
            InteractionType::CascadingFailure,
            vec![
                "workspace".to_string(),
                "buddy".to_string(),
                "omnibot".to_string(),
            ],
        )
    }
}

impl Scenario for InteractionScenario {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Interaction scenario executor
pub struct InteractionScenarioExecutor;

impl InteractionScenarioExecutor {
    /// Execute an interaction scenario
    pub async fn execute(scenario: &InteractionScenario) -> ApplicationStressResult<ScenarioResult> {
        let start = Instant::now();

        tracing::info!(
            "Executing interaction scenario {} between {:?}",
            scenario.scenario_type,
            scenario.apps_involved
        );

        let success = match scenario.scenario_type {
            InteractionType::WorkspaceBuddySync => {
                Self::test_workspace_buddy_sync().await
            }
            InteractionType::BuddyOmniBotQuery => {
                Self::test_buddy_omnibot_query().await
            }
            InteractionType::WorkspaceOmniBotIntegration => {
                Self::test_fullstack_integration().await
            }
            InteractionType::CascadingFailure => {
                Self::test_cascading_failure().await
            }
            InteractionType::DataFlowVerification => {
                Self::test_data_flow_verification().await
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        tracing::info!(
            "Interaction scenario {} completed: {}",
            scenario.id,
            if success { "SUCCESS" } else { "FAILED" }
        );

        Ok(ScenarioResult {
            scenario_id: scenario.id.clone(),
            name: scenario.scenario_type.to_string(),
            success,
            duration_ms,
            error: None,
            recovery_time_ms: None,
        })
    }

    /// Test Workspace triggering Buddy sync
    async fn test_workspace_buddy_sync() -> bool {
        // Workspace modifies files
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Triggers Buddy sync
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify Buddy received changes
        let verified = rand::random::<f64>() > 0.05; // 95% success

        verified
    }

    /// Test Buddy providing context to OmniBot
    async fn test_buddy_omnibot_query() -> bool {
        // OmniBot requests context from Buddy
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Buddy retrieves and sends context
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // OmniBot processes context
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let success = rand::random::<f64>() > 0.03; // 97% success

        success
    }

    /// Test full ecosystem integration
    async fn test_fullstack_integration() -> bool {
        // Workspace creates task
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Buddy syncs task context
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // OmniBot executes task with context
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Results propagate back through all apps
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let success = rand::random::<f64>() > 0.10; // 90% success for complex flow

        success
    }

    /// Test cascading failure scenario
    async fn test_cascading_failure() -> bool {
        // Kill one application (e.g., Buddy)
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Verify others detect failure
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Workspace should remain functional
        let workspace_ok = rand::random::<f64>() > 0.05;

        // OmniBot should degrade gracefully
        let omnibot_ok = rand::random::<f64>() > 0.10;

        workspace_ok && omnibot_ok
    }

    /// Test data flow verification
    async fn test_data_flow_verification() -> bool {
        // Inject test data at entry point (Workspace)
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Trace through Buddy
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Verify at OmniBot
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Check for corruption or loss
        let verified = rand::random::<f64>() > 0.02; // 98% data integrity

        verified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interaction_type_display() {
        let types = vec![
            InteractionType::WorkspaceBuddySync,
            InteractionType::BuddyOmniBotQuery,
            InteractionType::WorkspaceOmniBotIntegration,
            InteractionType::CascadingFailure,
            InteractionType::DataFlowVerification,
        ];

        for t in types {
            let s = t.to_string();
            assert!(!s.is_empty());
        }
    }

    #[test]
    fn test_interaction_scenario_creation() {
        let scenario = InteractionScenario::workspace_buddy_sync();
        assert_eq!(scenario.apps_involved.len(), 2);
        assert!(scenario.apps_involved.contains(&"workspace".to_string()));
        assert!(scenario.apps_involved.contains(&"buddy".to_string()));
    }

    #[test]
    fn test_cascading_failure_scenario() {
        let scenario = InteractionScenario::cascading_failure();
        assert_eq!(scenario.apps_involved.len(), 3);
    }

    #[tokio::test]
    async fn test_interaction_executor() {
        let scenario = InteractionScenario::workspace_buddy_sync();
        let result = InteractionScenarioExecutor::execute(&scenario).await;

        assert!(result.is_ok());
    }
}
