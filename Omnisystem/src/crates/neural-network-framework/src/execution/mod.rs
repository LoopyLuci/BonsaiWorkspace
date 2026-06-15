//! Execution engine for running computation graphs

pub mod device;

use crate::error::Result;
use crate::graph::ComputationGraph;
use crate::tensor::Tensor;
use std::collections::HashMap;

/// Execution engine for running computation graphs
pub struct ExecutionEngine {
    pub device: String,
    pub operations: crate::ops::OperationRegistry,
}

impl ExecutionEngine {
    /// Create a new execution engine
    pub fn new(device: &str) -> Result<Self> {
        Ok(ExecutionEngine {
            device: device.to_string(),
            operations: crate::ops::OperationRegistry::new(),
        })
    }

    /// Execute a computation graph
    pub fn execute(
        &self,
        graph: &ComputationGraph,
        inputs: &HashMap<String, Tensor>,
    ) -> Result<HashMap<String, Tensor>> {
        // Validate graph
        graph.validate()?;

        let mut state = HashMap::new();

        // Initialize with inputs
        for (node_id, tensor) in inputs {
            state.insert(node_id.clone(), tensor.clone());
        }

        // Execute nodes in topological order
        for node_id in &graph.node_order {
            if let Some(node) = graph.nodes.get(node_id) {
                // Skip input nodes (already in state)
                if graph.input_nodes.contains(node_id) {
                    continue;
                }

                // Get input tensors
                let mut input_tensors = Vec::new();
                for input_id in &node.inputs {
                    if let Some(tensor) = state.get(input_id) {
                        input_tensors.push(tensor.clone());
                    } else {
                        return Err(crate::error::Error::ExecutionError(
                            format!("Input tensor {} not found", input_id),
                        ));
                    }
                }

                // Execute operation
                let input_refs: Vec<_> = input_tensors.iter().collect();
                let op = self.operations.get(&node.operation)
                    .ok_or_else(|| crate::error::Error::OperationNotFound(node.operation.clone()))?;

                let output = (op.kernel)(&input_refs)?;
                state.insert(node_id.clone(), output);
            }
        }

        // Collect outputs
        let mut outputs = HashMap::new();
        for output_node_id in &graph.output_nodes {
            if let Some(tensor) = state.get(output_node_id) {
                outputs.insert(output_node_id.clone(), tensor.clone());
            }
        }

        Ok(outputs)
    }

    /// Build an execution plan for optimization
    pub fn build_execution_plan(&self, graph: &ComputationGraph) -> Vec<String> {
        graph.node_order.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Node;
    use crate::types::DType;

    #[test]
    fn test_execution_engine_creation() {
        let engine = ExecutionEngine::new("cpu").unwrap();
        assert_eq!(engine.device, "cpu");
    }

    #[test]
    fn test_simple_execution() {
        let mut graph = ComputationGraph::new("test");

        // Create nodes
        let input1 = Node {
            id: "input1".to_string(),
            operation: "input".to_string(),
            inputs: vec![],
            outputs: vec!["add".to_string()],
            attributes: Default::default(),
        };

        let input2 = Node {
            id: "input2".to_string(),
            operation: "input".to_string(),
            inputs: vec![],
            outputs: vec!["add".to_string()],
            attributes: Default::default(),
        };

        let add = Node {
            id: "add".to_string(),
            operation: "add".to_string(),
            inputs: vec!["input1".to_string(), "input2".to_string()],
            outputs: vec![],
            attributes: Default::default(),
        };

        graph.add_node(input1).unwrap();
        graph.add_node(input2).unwrap();
        graph.add_node(add).unwrap();
        graph.mark_input("input1");
        graph.mark_input("input2");
        graph.mark_output("add");

        // Create inputs
        let mut inputs = HashMap::new();
        inputs.insert("input1".to_string(),
            Tensor::ones(vec![2, 3], DType::Float32, "cpu").unwrap());
        inputs.insert("input2".to_string(),
            Tensor::ones(vec![2, 3], DType::Float32, "cpu").unwrap());

        // Execute
        let engine = ExecutionEngine::new("cpu").unwrap();
        let outputs = engine.execute(&graph, &inputs).unwrap();

        assert!(outputs.contains_key("add"));
        let result = &outputs["add"];
        assert_eq!(result.sum().unwrap(), 12.0); // 6 elements * 2.0 each
    }
}
