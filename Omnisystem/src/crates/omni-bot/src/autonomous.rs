use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct AutonomousEngine {
    decision_queue: Arc<std::sync::Mutex<Vec<Decision>>>,
    executed_decisions: Arc<DashMap<String, bool>>,
}

#[derive(Debug, Clone)]
pub struct Decision {
    pub id: String,
    pub action_type: String,
    pub parameters: std::collections::HashMap<String, String>,
    pub confidence: f32,
}

impl AutonomousEngine {
    pub fn new() -> Self {
        Self {
            decision_queue: Arc::new(std::sync::Mutex::new(Vec::new())),
            executed_decisions: Arc::new(DashMap::new()),
        }
    }

    pub fn enqueue_decision(&self, decision: Decision) {
        let mut queue = self.decision_queue.lock().unwrap();
        queue.push(decision);
    }

    pub async fn execute_next(&self) -> Result<Option<String>> {
        let mut queue = self.decision_queue.lock().unwrap();
        if let Some(decision) = queue.pop() {
            tracing::info!("Executing decision: {}", decision.action_type);
            self.executed_decisions.insert(decision.id.clone(), true);
            Ok(Some(decision.action_type))
        } else {
            Ok(None)
        }
    }

    pub fn decision_count(&self) -> usize {
        self.executed_decisions.len()
    }
}

impl Default for AutonomousEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_autonomous_engine() {
        let engine = AutonomousEngine::new();
        let decision = Decision {
            id: "d1".to_string(),
            action_type: "control_iot".to_string(),
            parameters: std::collections::HashMap::new(),
            confidence: 0.95,
        };
        engine.enqueue_decision(decision);
        assert!(engine.execute_next().await.is_ok());
        assert_eq!(engine.decision_count(), 1);
    }
}
