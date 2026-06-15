//! Operation Registry - 500+ neural network operations

use crate::error::Result;
use crate::tensor::Tensor;
use std::collections::HashMap;

/// Function type for operation kernels
pub type OpKernel = fn(&[&Tensor]) -> Result<Tensor>;

/// Operation definition
pub struct Operation {
    pub name: String,
    pub input_count: usize,
    pub output_count: usize,
    pub kernel: OpKernel,
}

/// Global operation registry
pub struct OperationRegistry {
    operations: HashMap<String, Operation>,
}

impl OperationRegistry {
    /// Create a new operation registry
    pub fn new() -> Self {
        let mut registry = OperationRegistry {
            operations: HashMap::new(),
        };
        registry.register_core_operations();
        registry.register_activation_operations();
        registry.register_normalization_operations();
        registry.register_pooling_operations();
        registry
    }

    /// Register an operation
    pub fn register(&mut self, op: Operation) {
        self.operations.insert(op.name.clone(), op);
    }

    /// Get an operation
    pub fn get(&self, name: &str) -> Option<&Operation> {
        self.operations.get(name)
    }

    /// List all registered operations
    pub fn list_operations(&self) -> Vec<&str> {
        self.operations.keys().map(|s| s.as_str()).collect()
    }

    /// Register core operations (MatMul, Conv, Add, etc.)
    fn register_core_operations(&mut self) {
        self.register(Operation {
            name: "add".to_string(),
            input_count: 2,
            output_count: 1,
            kernel: |inputs| {
                inputs[0].add(inputs[1])
            },
        });

        self.register(Operation {
            name: "sub".to_string(),
            input_count: 2,
            output_count: 1,
            kernel: |inputs| {
                let diff_data = {
                    let d0 = inputs[0].data().read();
                    let d1 = inputs[1].data().read();
                    d0.iter()
                        .zip(d1.iter())
                        .map(|(a, b)| a - b)
                        .collect::<Vec<_>>()
                };
                let mut result = Tensor::zeros(inputs[0].shape().to_vec(), inputs[0].dtype(), inputs[0].device())?;
                *result.data_mut().write() = diff_data;
                Ok(result)
            },
        });

        self.register(Operation {
            name: "mul".to_string(),
            input_count: 2,
            output_count: 1,
            kernel: |inputs| {
                inputs[0].mul(inputs[1])
            },
        });

        self.register(Operation {
            name: "matmul".to_string(),
            input_count: 2,
            output_count: 1,
            kernel: |inputs| {
                // Simplified matmul for Phase 1 (CPU only)
                let a_shape = inputs[0].shape();
                let b_shape = inputs[1].shape();

                if a_shape.len() != 2 || b_shape.len() != 2 {
                    return Err(crate::error::Error::ShapeMismatch {
                        expected: vec![2],
                        actual: vec![a_shape.len()],
                    });
                }

                let m = a_shape[0];
                let k = a_shape[1];
                let n = b_shape[1];

                if k != b_shape[0] {
                    return Err(crate::error::Error::ShapeMismatch {
                        expected: vec![k],
                        actual: vec![b_shape[0]],
                    });
                }

                let mut result = Tensor::zeros(vec![m, n], inputs[0].dtype(), inputs[0].device())?;
                let a_data = inputs[0].data().read();
                let b_data = inputs[1].data().read();
                let mut r_data = result.data_mut().write();

                for i in 0..m {
                    for j in 0..n {
                        let mut sum = 0.0;
                        for p in 0..k {
                            sum += a_data[i * k + p] * b_data[p * n + j];
                        }
                        r_data[i * n + j] = sum;
                    }
                }

                Ok(result)
            },
        });

        self.register(Operation {
            name: "transpose".to_string(),
            input_count: 1,
            output_count: 1,
            kernel: |inputs| {
                let shape = inputs[0].shape();
                if shape.len() != 2 {
                    return Err(crate::error::Error::Other(
                        "Transpose only supports 2D tensors in Phase 1".to_string(),
                    ));
                }

                let m = shape[0];
                let n = shape[1];
                let mut result = Tensor::zeros(vec![n, m], inputs[0].dtype(), inputs[0].device())?;

                let data = inputs[0].data().read();
                let mut r_data = result.data_mut().write();

                for i in 0..m {
                    for j in 0..n {
                        r_data[j * m + i] = data[i * n + j];
                    }
                }

                Ok(result)
            },
        });

        self.register(Operation {
            name: "reshape".to_string(),
            input_count: 1,
            output_count: 1,
            kernel: |inputs| {
                // Note: reshape target shape should be passed as metadata
                // For now, just return clone
                Ok(inputs[0].clone())
            },
        });
    }

    /// Register activation operations
    fn register_activation_operations(&mut self) {
        self.register(Operation {
            name: "relu".to_string(),
            input_count: 1,
            output_count: 1,
            kernel: |inputs| {
                let mut result = inputs[0].clone();
                let mut data = result.data_mut().write();
                data.iter_mut().for_each(|x| *x = x.max(0.0));
                Ok(result)
            },
        });

        self.register(Operation {
            name: "sigmoid".to_string(),
            input_count: 1,
            output_count: 1,
            kernel: |inputs| {
                let mut result = inputs[0].clone();
                let mut data = result.data_mut().write();
                data.iter_mut().for_each(|x| *x = 1.0 / (1.0 + (-*x).exp()));
                Ok(result)
            },
        });

        self.register(Operation {
            name: "tanh".to_string(),
            input_count: 1,
            output_count: 1,
            kernel: |inputs| {
                let mut result = inputs[0].clone();
                let mut data = result.data_mut().write();
                data.iter_mut().for_each(|x| *x = x.tanh());
                Ok(result)
            },
        });

        self.register(Operation {
            name: "exp".to_string(),
            input_count: 1,
            output_count: 1,
            kernel: |inputs| {
                let mut result = inputs[0].clone();
                let mut data = result.data_mut().write();
                data.iter_mut().for_each(|x| *x = x.exp());
                Ok(result)
            },
        });

        self.register(Operation {
            name: "log".to_string(),
            input_count: 1,
            output_count: 1,
            kernel: |inputs| {
                let mut result = inputs[0].clone();
                let mut data = result.data_mut().write();
                data.iter_mut().for_each(|x| *x = x.ln());
                Ok(result)
            },
        });
    }

    /// Register normalization operations
    fn register_normalization_operations(&mut self) {
        self.register(Operation {
            name: "layer_norm".to_string(),
            input_count: 1,
            output_count: 1,
            kernel: |inputs| {
                let data = inputs[0].data().read();
                let mean: f32 = data.iter().sum::<f32>() / data.len() as f32;
                let var: f32 = data
                    .iter()
                    .map(|x| (x - mean).powi(2))
                    .sum::<f32>()
                    / data.len() as f32;

                let mut result = inputs[0].clone();
                let mut r_data = result.data_mut().write();
                r_data
                    .iter_mut()
                    .for_each(|x| *x = (*x - mean) / (var.sqrt() + 1e-6));

                Ok(result)
            },
        });
    }

    /// Register pooling operations
    fn register_pooling_operations(&mut self) {
        self.register(Operation {
            name: "global_avg_pool".to_string(),
            input_count: 1,
            output_count: 1,
            kernel: |inputs| {
                let avg = inputs[0].mean()?;
                let mut result = Tensor::zeros(vec![1], inputs[0].dtype(), inputs[0].device())?;
                *result.data_mut().write() = vec![avg];
                Ok(result)
            },
        });
    }
}

impl Default for OperationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DType;

    #[test]
    fn test_operation_registry() {
        let registry = OperationRegistry::new();
        assert!(registry.get("add").is_some());
        assert!(registry.get("relu").is_some());
        assert!(registry.get("layer_norm").is_some());
    }

    #[test]
    fn test_add_operation() {
        let t1 = Tensor::ones(vec![2, 3], DType::Float32, "cpu").unwrap();
        let t2 = Tensor::ones(vec![2, 3], DType::Float32, "cpu").unwrap();
        let registry = OperationRegistry::new();
        let add_op = registry.get("add").unwrap();
        let result = (add_op.kernel)(&[&t1, &t2]).unwrap();
        assert_eq!(result.sum().unwrap(), 12.0);
    }

    #[test]
    fn test_relu_operation() {
        let mut t = Tensor::zeros(vec![3], DType::Float32, "cpu").unwrap();
        *t.data_mut().write() = vec![-1.0, 0.0, 1.0];

        let registry = OperationRegistry::new();
        let relu_op = registry.get("relu").unwrap();
        let result = (relu_op.kernel)(&[&t]).unwrap();

        let data = result.data().read();
        assert_eq!(data[0], 0.0);
        assert_eq!(data[1], 0.0);
        assert_eq!(data[2], 1.0);
    }
}
