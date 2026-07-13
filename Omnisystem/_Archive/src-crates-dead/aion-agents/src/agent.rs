use crate::{AgentConfig, AgentState, Perception, Action, AgentMetrics, Result, AgentError};
use dashmap::DashMap;
use std::sync::Arc;

pub struct Agent {
    config: AgentConfig,
    state: Arc<std::sync::Mutex<AgentState>>,
    perceptions: Arc<DashMap<u64, Perception>>,
    actions: Arc<DashMap<String, Action>>,
    metrics: Arc<std::sync::Mutex<AgentMetrics>>,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            state: Arc::new(std::sync::Mutex::new(AgentState::Idle)),
            perceptions: Arc::new(DashMap::new()),
            actions: Arc::new(DashMap::new()),
            metrics: Arc::new(std::sync::Mutex::new(AgentMetrics {
                decisions_made: 0,
                actions_executed: 0,
                success_rate: 1.0,
                learning_progress: 0.0,
            })),
        }
    }

    pub fn perceive(&self, perception: Perception) -> Result<()> {
        self.perceptions.insert(perception.timestamp, perception);
        tracing::info!("Perception recorded");
        Ok(())
    }

    pub fn decide(&self) -> Result<Action> {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.decisions_made += 1;

        Ok(Action {
            id: uuid::Uuid::new_v4().to_string(),
            action_type: "default".to_string(),
            parameters: std::collections::HashMap::new(),
            priority: 5,
        })
    }

    pub fn execute(&self, action: &Action) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        *state = AgentState::Executing;

        let mut metrics = self.metrics.lock().unwrap();
        metrics.actions_executed += 1;

        *state = AgentState::Idle;
        tracing::info!("Action executed");
        Ok(())
    }

    pub fn get_state(&self) -> AgentState {
        *self.state.lock().unwrap()
    }

    pub fn get_metrics(&self) -> AgentMetrics {
        self.metrics.lock().unwrap().clone()
    }

    pub fn perception_count(&self) -> usize {
        self.perceptions.len()
    }

    pub fn get_id(&self) -> &str {
        &self.config.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let config = AgentConfig {
            id: "a1".to_string(),
            name: "Agent1".to_string(),
            agent_type: crate::DecisionType::Reactive,
            learning_enabled: true,
            coordination_enabled: true,
        };
        let agent = Agent::new(config);
        assert_eq!(agent.get_state(), AgentState::Idle);
    }

    #[test]
    fn test_perception() {
        let config = AgentConfig {
            id: "a1".to_string(),
            name: "Agent1".to_string(),
            agent_type: crate::DecisionType::Reactive,
            learning_enabled: true,
            coordination_enabled: true,
        };
        let agent = Agent::new(config);
        let perception = Perception {
            sensor_data: vec![0.1, 0.2],
            timestamp: 1000,
            confidence: 0.9,
        };
        assert!(agent.perceive(perception).is_ok());
        assert_eq!(agent.perception_count(), 1);
    }

    #[test]
    fn test_decision_and_execution() {
        let config = AgentConfig {
            id: "a1".to_string(),
            name: "Agent1".to_string(),
            agent_type: crate::DecisionType::Deliberative,
            learning_enabled: true,
            coordination_enabled: true,
        };
        let agent = Agent::new(config);
        let action = agent.decide().unwrap();
        assert!(agent.execute(&action).is_ok());
    }
}
