use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorType {
    Flocking,
    Foraging,
    Schooling,
    Herding,
}

#[derive(Debug, Clone)]
pub struct BehaviorState {
    pub agent_id: String,
    pub behavior: BehaviorType,
    pub intensity: f32,
    pub neighbors: Vec<String>,
}

pub struct CollectiveBehaviorEngine {
    behaviors: Arc<DashMap<String, BehaviorState>>,
}

impl CollectiveBehaviorEngine {
    pub fn new() -> Self {
        Self {
            behaviors: Arc::new(DashMap::new()),
        }
    }

    pub fn register_behavior(&self, behavior: BehaviorState) {
        self.behaviors.insert(behavior.agent_id.clone(), behavior);
    }

    pub fn update_neighbors(&self, agent_id: &str, neighbors: Vec<String>) -> bool {
        if let Some(mut behavior) = self.behaviors.get_mut(agent_id) {
            behavior.neighbors = neighbors;
            true
        } else {
            false
        }
    }

    pub fn update_intensity(&self, agent_id: &str, intensity: f32) -> bool {
        if let Some(mut behavior) = self.behaviors.get_mut(agent_id) {
            behavior.intensity = intensity.clamp(0.0, 1.0);
            true
        } else {
            false
        }
    }

    pub fn get_behavior(&self, agent_id: &str) -> Option<BehaviorState> {
        self.behaviors.get(agent_id).map(|b| b.clone())
    }

    pub fn get_agents_with_behavior(&self, behavior_type: BehaviorType) -> Vec<String> {
        self.behaviors
            .iter()
            .filter(|entry| entry.behavior == behavior_type)
            .map(|entry| entry.agent_id.clone())
            .collect()
    }

    pub fn behavior_count(&self) -> usize {
        self.behaviors.len()
    }
}

impl Default for CollectiveBehaviorEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_behavior_registration() {
        let cbe = CollectiveBehaviorEngine::new();
        let behavior = BehaviorState {
            agent_id: "a1".to_string(),
            behavior: BehaviorType::Flocking,
            intensity: 0.8,
            neighbors: vec![],
        };
        cbe.register_behavior(behavior);
        assert_eq!(cbe.behavior_count(), 1);
    }

    #[test]
    fn test_behavior_update() {
        let cbe = CollectiveBehaviorEngine::new();
        let behavior = BehaviorState {
            agent_id: "a1".to_string(),
            behavior: BehaviorType::Herding,
            intensity: 0.5,
            neighbors: vec![],
        };
        cbe.register_behavior(behavior);
        assert!(cbe.update_intensity("a1", 0.9));
        let updated = cbe.get_behavior("a1").unwrap();
        assert_eq!(updated.intensity, 0.9);
    }
}
