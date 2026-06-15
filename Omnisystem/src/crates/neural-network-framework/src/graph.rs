//! Computation Graph - DAG representation of neural networks

use crate::error::Result;
use crate::tensor::Tensor;
use std::collections::HashMap;

/// Node in the computation graph
#[derive(Clone)]
pub struct Node {
    pub id: String,
    pub operation: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub attributes: HashMap<String, String>,
}

/// Edge connecting two nodes
#[derive(Clone)]
pub struct Edge {
    pub from_node: String,
    pub to_node: String,
    pub output_index: usize,
    pub input_index: usize,
}

/// Directed Acyclic Graph (DAG) for computation
pub struct ComputationGraph {
    pub id: String,
    pub nodes: HashMap<String, Node>,
    pub edges: Vec<Edge>,
    pub input_nodes: Vec<String>,
    pub output_nodes: Vec<String>,
    pub node_order: Vec<String>,
}

impl ComputationGraph {
    /// Create a new computation graph
    pub fn new(id: &str) -> Self {
        ComputationGraph {
            id: id.to_string(),
            nodes: HashMap::new(),
            edges: Vec::new(),
            input_nodes: Vec::new(),
            output_nodes: Vec::new(),
            node_order: Vec::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: Node) -> Result<()> {
        self.nodes.insert(node.id.clone(), node);
        self.topological_sort();
        Ok(())
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, edge: Edge) -> Result<()> {
        self.edges.push(edge);
        Ok(())
    }

    /// Mark a node as input
    pub fn mark_input(&mut self, node_id: &str) {
        if !self.input_nodes.contains(&node_id.to_string()) {
            self.input_nodes.push(node_id.to_string());
        }
    }

    /// Mark a node as output
    pub fn mark_output(&mut self, node_id: &str) {
        if !self.output_nodes.contains(&node_id.to_string()) {
            self.output_nodes.push(node_id.to_string());
        }
    }

    /// Topological sort of nodes
    pub fn topological_sort(&mut self) {
        let mut visited = std::collections::HashSet::new();
        let mut stack = Vec::new();

        for node_id in self.nodes.keys() {
            if !visited.contains(node_id) {
                self.dfs(node_id, &mut visited, &mut stack);
            }
        }

        self.node_order = stack.into_iter().rev().collect();
    }

    fn dfs(&self, node_id: &str, visited: &mut std::collections::HashSet<String>, stack: &mut Vec<String>) {
        visited.insert(node_id.to_string());

        if let Some(node) = self.nodes.get(node_id) {
            for input_id in &node.inputs {
                if !visited.contains(input_id) {
                    self.dfs(input_id, visited, stack);
                }
            }
        }

        stack.push(node_id.to_string());
    }

    /// Get total number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get total number of edges
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Validate the graph
    pub fn validate(&self) -> Result<()> {
        // Check for cycles
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        for node_id in self.nodes.keys() {
            if !visited.contains(node_id) {
                self.check_cycle(node_id, &mut visited, &mut rec_stack)?;
            }
        }

        // Check that all inputs exist
        for node in self.nodes.values() {
            for input_id in &node.inputs {
                if !self.nodes.contains_key(input_id) {
                    return Err(crate::error::Error::InvalidGraph(
                        format!("Input node {} not found", input_id),
                    ));
                }
            }
        }

        Ok(())
    }

    fn check_cycle(&self, node_id: &str, visited: &mut std::collections::HashSet<String>, rec_stack: &mut std::collections::HashSet<String>) -> Result<()> {
        visited.insert(node_id.to_string());
        rec_stack.insert(node_id.to_string());

        if let Some(node) = self.nodes.get(node_id) {
            for input_id in &node.inputs {
                if !visited.contains(input_id) {
                    self.check_cycle(input_id, visited, rec_stack)?;
                } else if rec_stack.contains(input_id) {
                    return Err(crate::error::Error::InvalidGraph(
                        "Cycle detected in graph".to_string(),
                    ));
                }
            }
        }

        rec_stack.remove(node_id);
        Ok(())
    }

    /// Generate a simple string representation
    pub fn to_string_repr(&self) -> String {
        let mut s = format!("ComputationGraph ({})\n", self.id);
        s.push_str(&format!("Nodes: {}, Edges: {}\n", self.nodes.len(), self.edges.len()));
        s.push_str("Execution Order:\n");
        for node_id in &self.node_order {
            if let Some(node) = self.nodes.get(node_id) {
                s.push_str(&format!("  {} : {}\n", node.id, node.operation));
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let graph = ComputationGraph::new("test");
        assert_eq!(graph.id, "test");
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph = ComputationGraph::new("test");
        let node = Node {
            id: "input".to_string(),
            operation: "input".to_string(),
            inputs: vec![],
            outputs: vec!["add".to_string()],
            attributes: HashMap::new(),
        };
        graph.add_node(node).unwrap();
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_graph_validation() {
        let mut graph = ComputationGraph::new("test");
        let node1 = Node {
            id: "input".to_string(),
            operation: "input".to_string(),
            inputs: vec![],
            outputs: vec![],
            attributes: HashMap::new(),
        };
        graph.add_node(node1).unwrap();
        assert!(graph.validate().is_ok());
    }
}
