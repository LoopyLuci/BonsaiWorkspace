use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct MeshNetworkController {
    nodes: Arc<DashMap<String, MeshNode>>,
    routes: Arc<DashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone)]
pub struct MeshNode {
    pub id: String,
    pub neighbors: Vec<String>,
    pub depth: u32,
}

impl MeshNetworkController {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(DashMap::new()),
            routes: Arc::new(DashMap::new()),
        }
    }

    pub fn add_node(&self, id: String, neighbors: Vec<String>) -> Result<()> {
        let node = MeshNode {
            id: id.clone(),
            neighbors,
            depth: 0,
        };
        self.nodes.insert(id, node);
        tracing::info!("Mesh node added");
        Ok(())
    }

    pub fn find_route(&self, source: &str, dest: &str) -> Option<Vec<String>> {
        self.routes.get(&format!("{}→{}", source, dest))
            .map(|ref_| ref_.value().clone())
    }

    pub fn register_route(&self, source: String, dest: String, path: Vec<String>) {
        self.routes.insert(format!("{}→{}", source, dest), path);
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl Default for MeshNetworkController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_network() {
        let mesh = MeshNetworkController::new();
        assert!(mesh.add_node("n1".to_string(), vec!["n2".to_string()]).is_ok());
        assert_eq!(mesh.node_count(), 1);
    }
}
