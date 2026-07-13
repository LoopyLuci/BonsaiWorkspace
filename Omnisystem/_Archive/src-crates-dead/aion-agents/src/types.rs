use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    Idle,
    Active,
    Learning,
    Coordinating,
    Executing,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionType {
    Reactive,
    Deliberative,
    Adaptive,
    Emergent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub agent_type: DecisionType,
    pub learning_enabled: bool,
    pub coordination_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Perception {
    pub sensor_data: Vec<f32>,
    pub timestamp: u64,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub action_type: String,
    pub parameters: std::collections::HashMap<String, String>,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub decisions_made: u64,
    pub actions_executed: u64,
    pub success_rate: f32,
    pub learning_progress: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config() {
        let config = AgentConfig {
            id: Uuid::new_v4().to_string(),
            name: "Agent1".to_string(),
            agent_type: DecisionType::Adaptive,
            learning_enabled: true,
            coordination_enabled: true,
        };
        assert_eq!(config.agent_type, DecisionType::Adaptive);
    }

    #[test]
    fn test_perception() {
        let perception = Perception {
            sensor_data: vec![0.1, 0.2, 0.3],
            timestamp: 1000,
            confidence: 0.95,
        };
        assert_eq!(perception.confidence, 0.95);
    }
}
