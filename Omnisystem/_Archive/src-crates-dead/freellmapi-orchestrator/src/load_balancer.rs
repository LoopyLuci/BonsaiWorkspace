use crate::node::Node;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BalancingStrategy {
    RoundRobin,
    LeastConnections,
    WeightedRandom,
}

pub struct LoadBalancer {
    strategy: BalancingStrategy,
    round_robin_index: usize,
}

impl LoadBalancer {
    pub fn new(strategy: BalancingStrategy) -> Self {
        LoadBalancer {
            strategy,
            round_robin_index: 0,
        }
    }

    pub fn select_node(&mut self, nodes: &[Node]) -> Option<Node> {
        if nodes.is_empty() {
            return None;
        }

        match self.strategy {
            BalancingStrategy::RoundRobin => {
                let node = nodes[self.round_robin_index % nodes.len()].clone();
                self.round_robin_index = (self.round_robin_index + 1) % nodes.len();
                Some(node)
            }
            BalancingStrategy::LeastConnections => {
                nodes
                    .iter()
                    .min_by_key(|n| n.current_load)
                    .cloned()
            }
            BalancingStrategy::WeightedRandom => {
                // Simple random selection for now
                let idx = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() % nodes.len() as u128) as usize;
                Some(nodes[idx].clone())
            }
        }
    }
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self::new(BalancingStrategy::LeastConnections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_robin() {
        let mut lb = LoadBalancer::new(BalancingStrategy::RoundRobin);
        let nodes = vec![
            Node::new("n1".to_string(), "addr1".to_string(), 100),
            Node::new("n2".to_string(), "addr2".to_string(), 100),
        ];

        let first = lb.select_node(&nodes);
        let second = lb.select_node(&nodes);

        assert_eq!(first.as_ref().map(|n| &n.id), Some(&"n1".to_string()));
        assert_eq!(second.as_ref().map(|n| &n.id), Some(&"n2".to_string()));
    }

    #[test]
    fn test_least_connections() {
        let mut lb = LoadBalancer::new(BalancingStrategy::LeastConnections);
        let mut n1 = Node::new("n1".to_string(), "addr1".to_string(), 100);
        let mut n2 = Node::new("n2".to_string(), "addr2".to_string(), 100);
        n1.current_load = 50;
        n2.current_load = 30;

        let selected = lb.select_node(&[n1, n2]);
        assert_eq!(selected.as_ref().map(|n| &n.id), Some(&"n2".to_string()));
    }

    #[test]
    fn test_empty_nodes() {
        let mut lb = LoadBalancer::new(BalancingStrategy::RoundRobin);
        let result = lb.select_node(&[]);
        assert!(result.is_none());
    }
}
