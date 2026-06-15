//! Fault scenarios and cross-application interaction scenarios

mod fault;
mod interaction;

pub use fault::{FaultScenario, FaultScenarioExecutor};
pub use interaction::{InteractionScenario, InteractionScenarioExecutor};


/// Scenario execution result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScenarioResult {
    pub scenario_id: String,
    pub name: String,
    pub success: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub recovery_time_ms: Option<u64>,
}

/// Trait for all scenarios
pub trait Scenario: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_result_creation() {
        let result = ScenarioResult {
            scenario_id: "fault-01".to_string(),
            name: "Application Crash".to_string(),
            success: true,
            duration_ms: 5000,
            error: None,
            recovery_time_ms: Some(1000),
        };

        assert_eq!(result.scenario_id, "fault-01");
        assert!(result.success);
    }
}
