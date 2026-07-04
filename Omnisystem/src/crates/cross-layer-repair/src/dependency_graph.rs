//! Dependency graph for layer ordering

use crate::{Result, SystemLayer};
use std::collections::{HashMap, VecDeque};

pub struct DependencyGraph {
    nodes: HashMap<SystemLayer, Vec<SystemLayer>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn add_layer(&mut self, layer: SystemLayer) -> Result<()> {
        self.nodes.entry(layer).or_insert_with(Vec::new);
        Ok(())
    }

    /// Add dependency: layer1 depends on layer2
    pub fn add_dependency(&mut self, layer1: SystemLayer, layer2: SystemLayer) -> Result<()> {
        self.nodes
            .entry(layer1.clone())
            .or_insert_with(Vec::new)
            .push(layer2);
        Ok(())
    }

    /// Get topological sort of layers (repair order)
    pub fn topological_sort(&self) -> Result<Vec<SystemLayer>> {
        let mut visited = HashMap::new();
        let mut result = Vec::new();

        for layer in self.nodes.keys() {
            if !visited.contains_key(layer) {
                self.dfs(layer, &mut visited, &mut result);
            }
        }

        result.reverse();
        Ok(result)
    }

    fn dfs(
        &self,
        layer: &SystemLayer,
        visited: &mut HashMap<SystemLayer, bool>,
        result: &mut Vec<SystemLayer>,
    ) {
        visited.insert(layer.clone(), true);

        if let Some(deps) = self.nodes.get(layer) {
            for dep in deps {
                if !visited.contains_key(dep) {
                    self.dfs(dep, visited, result);
                }
            }
        }

        result.push(layer.clone());
    }

    /// Check for circular dependencies
    pub fn has_cycles(&self) -> bool {
        let mut visited = HashMap::new();
        let mut rec_stack = HashMap::new();

        for layer in self.nodes.keys() {
            if !visited.contains_key(layer) {
                if self.has_cycle_dfs(layer, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }

        false
    }

    fn has_cycle_dfs(
        &self,
        layer: &SystemLayer,
        visited: &mut HashMap<SystemLayer, bool>,
        rec_stack: &mut HashMap<SystemLayer, bool>,
    ) -> bool {
        visited.insert(layer.clone(), true);
        rec_stack.insert(layer.clone(), true);

        if let Some(deps) = self.nodes.get(layer) {
            for dep in deps {
                if !visited.contains_key(dep) {
                    if self.has_cycle_dfs(dep, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.get(dep).copied().unwrap_or(false) {
                    return true;
                }
            }
        }

        rec_stack.insert(layer.clone(), false);
        false
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph_creation() {
        let graph = DependencyGraph::new();
        assert!(graph.nodes.is_empty());
    }

    #[test]
    fn test_add_layer() -> Result<()> {
        let mut graph = DependencyGraph::new();
        graph.add_layer(SystemLayer::UOSC)?;
        assert!(!graph.nodes.is_empty());
        Ok(())
    }

    #[test]
    fn test_topological_sort() -> Result<()> {
        let mut graph = DependencyGraph::new();
        graph.add_layer(SystemLayer::UOSC)?;
        graph.add_layer(SystemLayer::Omnisystem)?;
        graph.add_layer(SystemLayer::BonsaiEcosystem)?;

        graph.add_dependency(SystemLayer::Omnisystem, SystemLayer::UOSC)?;
        graph.add_dependency(SystemLayer::BonsaiEcosystem, SystemLayer::Omnisystem)?;

        let sorted = graph.topological_sort()?;
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0], SystemLayer::UOSC);
        Ok(())
    }
}
