//! CLI demo for aion-agents: perceive/decide/execute cycle plus
//! coordination and consensus voting across a couple of agents.

use aion_agents::agent::Agent;
use aion_agents::consensus::ConsensusEngine;
use aion_agents::coordination::{AgentHandle, CoordinationManager};
use aion_agents::{AgentConfig, DecisionType, Perception};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent = Agent::new(AgentConfig {
        id: "agent-1".to_string(),
        name: "Scout".to_string(),
        agent_type: DecisionType::Adaptive,
        learning_enabled: true,
        coordination_enabled: true,
    });

    agent.perceive(Perception {
        sensor_data: vec![0.4, 0.6],
        timestamp: 1,
        confidence: 0.8,
    })?;

    let action = agent.decide()?;
    println!("Agent decided on action: {} (priority {})", action.action_type, action.priority);
    agent.execute(&action)?;

    let metrics = agent.get_metrics();
    println!(
        "Decisions made: {}, actions executed: {}",
        metrics.decisions_made, metrics.actions_executed
    );

    let coordination = CoordinationManager::new();
    coordination.register_agent(AgentHandle {
        id: agent.get_id().to_string(),
        name: "Scout".to_string(),
        state: "active".to_string(),
    })?;
    println!("Registered agents: {}", coordination.agent_count());

    let consensus = ConsensusEngine::new(2);
    consensus.vote("agent-1".to_string(), true)?;
    consensus.vote("agent-2".to_string(), true)?;
    println!("Consensus reached: {}", consensus.check_consensus());

    Ok(())
}
