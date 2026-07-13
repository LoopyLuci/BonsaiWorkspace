use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PolicyGradient {
    pub state_id: String,
    pub action_probs: Vec<f32>,
    pub baseline: f32,
}

pub struct PolicyOptimizer {
    policies: Arc<DashMap<String, PolicyGradient>>,
    learning_rate: f32,
}

impl PolicyOptimizer {
    pub fn new(lr: f32) -> Self {
        Self {
            policies: Arc::new(DashMap::new()),
            learning_rate: lr,
        }
    }

    pub fn initialize_policy(&self, state_id: String, num_actions: usize) {
        let prob = 1.0 / num_actions as f32;
        let policy = PolicyGradient {
            state_id: state_id.clone(),
            action_probs: vec![prob; num_actions],
            baseline: 0.0,
        };
        self.policies.insert(state_id, policy);
    }

    pub fn update_policy(&self, state: &str, action: usize, advantage: f32) {
        if let Some(mut policy) = self.policies.get_mut(state) {
            policy.action_probs[action] = (policy.action_probs[action]
                + self.learning_rate * advantage).max(0.01).min(0.99);
            policy.baseline += self.learning_rate * advantage;
        }
    }

    pub fn get_action_probability(&self, state: &str, action: usize) -> Option<f32> {
        self.policies.get(state).map(|policy| {
            if action < policy.action_probs.len() {
                policy.action_probs[action]
            } else {
                0.0
            }
        })
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_init() {
        let po = PolicyOptimizer::new(0.01);
        po.initialize_policy("s1".to_string(), 4);
        assert_eq!(po.policy_count(), 1);
    }

    #[test]
    fn test_policy_update() {
        let po = PolicyOptimizer::new(0.01);
        po.initialize_policy("s1".to_string(), 4);
        po.update_policy("s1", 0, 5.0);
        let prob = po.get_action_probability("s1", 0).unwrap();
        assert!(prob > 0.25);
    }
}
