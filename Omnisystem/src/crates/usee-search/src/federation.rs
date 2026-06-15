use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct SearchFederation {
    nodes: Arc<DashMap<String, FederatedNode>>,
}

#[derive(Debug, Clone)]
pub struct FederatedNode {
    pub id: String,
    pub endpoint: String,
    pub healthy: bool,
}

impl SearchFederation {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(DashMap::new()),
        }
    }

    pub fn register_node(&self, id: String, endpoint: String) -> Result<()> {
        let node = FederatedNode {
            id: id.clone(),
            endpoint,
            healthy: true,
        };
        self.nodes.insert(id, node);
        tracing::info!("Federated node registered");
        Ok(())
    }

    pub fn get_healthy_nodes(&self) -> Vec<FederatedNode> {
        self.nodes
            .iter()
            .filter(|n| n.value().healthy)
            .map(|n| n.value().clone())
            .collect()
    }

    pub fn mark_unhealthy(&self, id: &str) -> Result<()> {
        if let Some(mut node) = self.nodes.get_mut(id) {
            node.healthy = false;
            Ok(())
        } else {
            Err(crate::SearchError::QueryError("Node not found".to_string()))
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl Default for SearchFederation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_federation() {
        let fed = SearchFederation::new();
        assert!(fed.register_node("n1".to_string(), "http://localhost:8080".to_string()).is_ok());
        assert_eq!(fed.node_count(), 1);
    }
}
