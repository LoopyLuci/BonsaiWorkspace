use aion_agents::*;

#[test]
fn test_full_agent_workflow() {
    let config = AgentConfig {
        id: "agent1".to_string(),
        name: "Test Agent".to_string(),
        agent_type: DecisionType::Adaptive,
        learning_enabled: true,
        coordination_enabled: true,
    };

    let agent = agent::Agent::new(config);

    let perception = Perception {
        sensor_data: vec![0.5, 0.6, 0.7],
        timestamp: 1000,
        confidence: 0.92,
    };

    agent.perceive(perception).unwrap();
    assert_eq!(agent.perception_count(), 1);

    let action = agent.decide().unwrap();
    agent.execute(&action).unwrap();

    let metrics = agent.get_metrics();
    assert!(metrics.decisions_made > 0);
    assert!(metrics.actions_executed > 0);
}

#[test]
fn test_learning_engine() {
    let engine = learning::LearningEngine::new();
    
    engine.learn("skill1".to_string(), 0.75).unwrap();
    engine.learn("skill2".to_string(), 0.85).unwrap();
    
    assert_eq!(engine.knowledge_size(), 2);
    assert_eq!(engine.recall("skill1"), Some(0.75));
}

#[test]
fn test_coordination() {
    let manager = coordination::CoordinationManager::new();
    
    let agent1 = coordination::AgentHandle {
        id: "a1".to_string(),
        name: "Agent1".to_string(),
        state: "active".to_string(),
    };

    let agent2 = coordination::AgentHandle {
        id: "a2".to_string(),
        name: "Agent2".to_string(),
        state: "active".to_string(),
    };

    manager.register_agent(agent1).unwrap();
    manager.register_agent(agent2).unwrap();

    assert_eq!(manager.agent_count(), 2);
    assert!(manager.broadcast_message("sync").is_ok());
}
