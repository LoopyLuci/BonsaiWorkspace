use crate::{Agent, Result};
use dashmap::DashMap;
use std::sync::Arc;

pub struct SwarmController {
    agents: Arc<DashMap<String, Agent>>,
    consensus_state: Arc<std::sync::Mutex<ConsensusState>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusState {
    Idle,
    Proposing,
    Voting,
    Agreed,
}

impl SwarmController {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(DashMap::new()),
            consensus_state: Arc::new(std::sync::Mutex::new(ConsensusState::Idle)),
        }
    }

    pub fn add_agent(&self, agent: Agent) -> Result<()> {
        self.agents.insert(agent.get_id().to_string(), agent);
        tracing::info!("Agent added to swarm");
        Ok(())
    }

    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    pub async fn propose_consensus(&self) -> Result<bool> {
        let mut state = self.consensus_state.lock().unwrap();
        *state = ConsensusState::Proposing;
        
        let agent_count = self.agents.len();
        let votes_needed = (agent_count / 2) + 1;
        
        *state = ConsensusState::Voting;
        
        // Simulate voting (in real system, would collect votes from agents)
        let mut votes = 0;
        for agent in self.agents.iter() {
            if agent.get_state() == crate::AgentState::Active {
                votes += 1;
            }
        }
        
        if votes >= votes_needed {
            *state = ConsensusState::Agreed;
            Ok(true)
        } else {
            *state = ConsensusState::Idle;
            Ok(false)
        }
    }

    pub fn get_consensus_state(&self) -> ConsensusState {
        *self.consensus_state.lock().unwrap()
    }
}

impl Default for SwarmController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_swarm_controller() {
        let swarm = SwarmController::new();
        let config = crate::AgentConfig {
            id: "a1".to_string(),
            name: "Agent1".to_string(),
            agent_type: crate::DecisionType::Reactive,
            learning_enabled: true,
            coordination_enabled: true,
        };
        let agent = Agent::new(config);
        assert!(swarm.add_agent(agent).is_ok());
        assert_eq!(swarm.agent_count(), 1);
    }
}
