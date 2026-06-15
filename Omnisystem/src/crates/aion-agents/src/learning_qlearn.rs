use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct QState {
    pub state_id: String,
    pub q_values: Vec<f32>,
}

pub struct QLearner {
    q_table: Arc<DashMap<String, QState>>,
    learning_rate: f32,
    discount_factor: f32,
}

impl QLearner {
    pub fn new(alpha: f32, gamma: f32) -> Self {
        Self {
            q_table: Arc::new(DashMap::new()),
            learning_rate: alpha,
            discount_factor: gamma,
        }
    }

    pub fn initialize_state(&self, state_id: String, num_actions: usize) {
        let q_state = QState {
            state_id: state_id.clone(),
            q_values: vec![0.0; num_actions],
        };
        self.q_table.insert(state_id, q_state);
    }

    pub fn update_q_value(&self, state: &str, action: usize, reward: f32, next_state: &str) {
        if let Some(mut current) = self.q_table.get_mut(state) {
            if let Some(next) = self.q_table.get(next_state) {
                let max_next = next.q_values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                current.q_values[action] = current.q_values[action]
                    + self.learning_rate * (reward + self.discount_factor * max_next - current.q_values[action]);
            }
        }
    }

    pub fn get_best_action(&self, state: &str) -> Option<usize> {
        self.q_table.get(state).and_then(|q_state| {
            q_state.q_values
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(idx, _)| idx)
        })
    }

    pub fn state_count(&self) -> usize {
        self.q_table.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qlearner_init() {
        let ql = QLearner::new(0.1, 0.9);
        ql.initialize_state("s1".to_string(), 4);
        assert_eq!(ql.state_count(), 1);
    }

    #[test]
    fn test_qlearner_update() {
        let ql = QLearner::new(0.1, 0.9);
        ql.initialize_state("s1".to_string(), 4);
        ql.initialize_state("s2".to_string(), 4);
        ql.update_q_value("s1", 0, 10.0, "s2");
        assert!(ql.get_best_action("s1").is_some());
    }
}
