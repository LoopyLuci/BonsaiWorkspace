use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct CoordinationManager {
    agents: Arc<DashMap<String, AgentHandle>>,
}

#[derive(Debug, Clone)]
pub struct AgentHandle {
    pub id: String,
    pub name: String,
    pub state: String,
}

impl CoordinationManager {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(DashMap::new()),
        }
    }

    pub fn register_agent(&self, agent: AgentHandle) -> Result<()> {
        self.agents.insert(agent.id.clone(), agent);
        tracing::info!("Agent registered in coordination");
        Ok(())
    }

    pub fn get_agent(&self, id: &str) -> Option<AgentHandle> {
        self.agents.get(id).map(|ref_| ref_.value().clone())
    }

    pub fn broadcast_message(&self, message: &str) -> Result<()> {
        tracing::info!("Broadcasting message to {} agents", self.agents.len());
        Ok(())
    }

    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }
}

impl Default for CoordinationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordination_manager() {
        let manager = CoordinationManager::new();
        let agent = AgentHandle {
            id: "a1".to_string(),
            name: "Agent1".to_string(),
            state: "idle".to_string(),
        };
        assert!(manager.register_agent(agent).is_ok());
        assert_eq!(manager.agent_count(), 1);
    }
}
