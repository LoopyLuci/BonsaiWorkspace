use crate::{DecisionType, Perception, Action, Result};

pub struct DecisionEngine {
    decision_type: DecisionType,
}

impl DecisionEngine {
    pub fn new(decision_type: DecisionType) -> Self {
        Self { decision_type }
    }

    pub fn decide(&self, perception: &Perception) -> Result<Action> {
        match self.decision_type {
            DecisionType::Reactive => self.reactive_decision(perception),
            DecisionType::Deliberative => self.deliberative_decision(perception),
            DecisionType::Adaptive => self.adaptive_decision(perception),
            DecisionType::Emergent => self.emergent_decision(perception),
        }
    }

    fn reactive_decision(&self, _perception: &Perception) -> Result<Action> {
        Ok(Action {
            id: uuid::Uuid::new_v4().to_string(),
            action_type: "reactive".to_string(),
            parameters: std::collections::HashMap::new(),
            priority: 5,
        })
    }

    fn deliberative_decision(&self, _perception: &Perception) -> Result<Action> {
        Ok(Action {
            id: uuid::Uuid::new_v4().to_string(),
            action_type: "deliberative".to_string(),
            parameters: std::collections::HashMap::new(),
            priority: 7,
        })
    }

    fn adaptive_decision(&self, _perception: &Perception) -> Result<Action> {
        Ok(Action {
            id: uuid::Uuid::new_v4().to_string(),
            action_type: "adaptive".to_string(),
            parameters: std::collections::HashMap::new(),
            priority: 6,
        })
    }

    fn emergent_decision(&self, _perception: &Perception) -> Result<Action> {
        Ok(Action {
            id: uuid::Uuid::new_v4().to_string(),
            action_type: "emergent".to_string(),
            parameters: std::collections::HashMap::new(),
            priority: 4,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_engine_types() {
        let types = vec![
            DecisionType::Reactive,
            DecisionType::Deliberative,
            DecisionType::Adaptive,
            DecisionType::Emergent,
        ];
        for dt in types {
            let engine = DecisionEngine::new(dt);
            let perception = Perception {
                sensor_data: vec![0.1],
                timestamp: 1000,
                confidence: 0.9,
            };
            assert!(engine.decide(&perception).is_ok());
        }
    }
}
