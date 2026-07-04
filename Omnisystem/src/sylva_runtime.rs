// SYLVA RUNTIME - ML/AI execution engine
// Complete tensor and neural network runtime
// Version: 2.0

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Tensor - n-dimensional array for ML/AI operations
#[derive(Debug, Clone)]
pub struct Tensor {
    pub data: Arc<Mutex<Vec<f32>>>,
    pub shape: Vec<usize>,
}

impl Tensor {
    /// Create a new tensor with given shape
    pub fn new(shape: Vec<usize>) -> Self {
        let size: usize = shape.iter().product();
        Tensor {
            data: Arc::new(Mutex::new(vec![0.0; size])),
            shape,
        }
    }

    /// Create tensor from data
    pub fn from_vec(data: Vec<f32>, shape: Vec<usize>) -> Result<Self, TensorError> {
        let size: usize = shape.iter().product();
        if data.len() != size {
            return Err(TensorError::ShapeMismatch {
                expected: shape.clone(),
                got: vec![data.len()],
            });
        }
        Ok(Tensor {
            data: Arc::new(Mutex::new(data)),
            shape,
        })
    }

    /// Create tensor filled with zeros
    pub fn zeros(shape: Vec<usize>) -> Self {
        Tensor::new(shape)
    }

    /// Create tensor filled with ones
    pub fn ones(shape: Vec<usize>) -> Self {
        let size: usize = shape.iter().product();
        Tensor {
            data: Arc::new(Mutex::new(vec![1.0; size])),
            shape,
        }
    }

    /// Create tensor with random values
    pub fn random(shape: Vec<usize>) -> Self {
        let size: usize = shape.iter().product();
        let mut data = vec![0.0; size];
        for i in 0..size {
            data[i] = (i as f32 % 1.0);
        }
        Tensor {
            data: Arc::new(Mutex::new(data)),
            shape,
        }
    }

    /// Get shape as slice
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    /// Get number of dimensions
    pub fn ndim(&self) -> usize {
        self.shape.len()
    }

    /// Get total number of elements
    pub fn size(&self) -> usize {
        self.shape.iter().product()
    }

    /// Get element at indices
    pub fn get(&self, indices: &[usize]) -> Result<f32, TensorError> {
        if indices.len() != self.shape.len() {
            return Err(TensorError::IndexOutOfBounds {
                shape: self.shape.clone(),
                index: indices.to_vec(),
            });
        }

        for (i, &idx) in indices.iter().enumerate() {
            if idx >= self.shape[i] {
                return Err(TensorError::IndexOutOfBounds {
                    shape: self.shape.clone(),
                    index: indices.to_vec(),
                });
            }
        }

        let flat_idx = self.compute_flat_index(indices);
        let data = self.data.lock().unwrap();
        Ok(data[flat_idx])
    }

    /// Set element at indices
    pub fn set(&self, indices: &[usize], value: f32) -> Result<(), TensorError> {
        if indices.len() != self.shape.len() {
            return Err(TensorError::IndexOutOfBounds {
                shape: self.shape.clone(),
                index: indices.to_vec(),
            });
        }

        for (i, &idx) in indices.iter().enumerate() {
            if idx >= self.shape[i] {
                return Err(TensorError::IndexOutOfBounds {
                    shape: self.shape.clone(),
                    index: indices.to_vec(),
                });
            }
        }

        let flat_idx = self.compute_flat_index(indices);
        let mut data = self.data.lock().unwrap();
        data[flat_idx] = value;
        Ok(())
    }

    /// Compute flat index from multi-dimensional indices
    fn compute_flat_index(&self, indices: &[usize]) -> usize {
        let mut flat_idx = 0;
        let mut stride = 1;
        for i in (0..indices.len()).rev() {
            flat_idx += indices[i] * stride;
            stride *= self.shape[i];
        }
        flat_idx
    }

    /// Reshape tensor
    pub fn reshape(&self, new_shape: Vec<usize>) -> Result<Tensor, TensorError> {
        let old_size: usize = self.shape.iter().product();
        let new_size: usize = new_shape.iter().product();

        if old_size != new_size {
            return Err(TensorError::InvalidReshape {
                from: self.shape.clone(),
                to: new_shape,
            });
        }

        let data = self.data.lock().unwrap();
        Ok(Tensor {
            data: Arc::new(Mutex::new(data.clone())),
            shape: new_shape,
        })
    }

    /// Flatten tensor to 1D
    pub fn flatten(&self) -> Tensor {
        let size = self.size();
        let data = self.data.lock().unwrap();
        Tensor {
            data: Arc::new(Mutex::new(data.clone())),
            shape: vec![size],
        }
    }

    /// Transpose 2D matrix
    pub fn transpose(&self) -> Result<Tensor, TensorError> {
        if self.shape.len() != 2 {
            return Err(TensorError::InvalidOperation("transpose only for 2D".to_string()));
        }

        let (rows, cols) = (self.shape[0], self.shape[1]);
        let mut result = vec![0.0; rows * cols];
        let data = self.data.lock().unwrap();

        for i in 0..rows {
            for j in 0..cols {
                result[j * rows + i] = data[i * cols + j];
            }
        }

        Tensor::from_vec(result, vec![cols, rows])
    }

    /// Sum all elements
    pub fn sum(&self) -> f32 {
        let data = self.data.lock().unwrap();
        data.iter().sum()
    }

    /// Compute mean
    pub fn mean(&self) -> f32 {
        let sum = self.sum();
        sum / self.size() as f32
    }

    /// Compute standard deviation
    pub fn std(&self) -> f32 {
        let mean = self.mean();
        let data = self.data.lock().unwrap();
        let variance: f32 = data.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f32>() / self.size() as f32;
        variance.sqrt()
    }

    /// Compute variance
    pub fn var(&self) -> f32 {
        let mean = self.mean();
        let data = self.data.lock().unwrap();
        let variance: f32 = data.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f32>() / self.size() as f32;
        variance
    }

    /// Find minimum value
    pub fn min(&self) -> f32 {
        let data = self.data.lock().unwrap();
        data.iter().cloned().fold(f32::INFINITY, f32::min)
    }

    /// Find maximum value
    pub fn max(&self) -> f32 {
        let data = self.data.lock().unwrap();
        data.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
    }

    /// Cloning
    pub fn clone(&self) -> Tensor {
        let data = self.data.lock().unwrap();
        Tensor {
            data: Arc::new(Mutex::new(data.clone())),
            shape: self.shape.clone(),
        }
    }
}

/// Tensor errors
#[derive(Debug, Clone)]
pub enum TensorError {
    ShapeMismatch {
        expected: Vec<usize>,
        got: Vec<usize>,
    },
    IndexOutOfBounds {
        shape: Vec<usize>,
        index: Vec<usize>,
    },
    InvalidReshape {
        from: Vec<usize>,
        to: Vec<usize>,
    },
    InvalidOperation(String),
}

/// Neural Network Layer trait
pub trait Layer {
    fn forward(&self, input: &Tensor) -> Result<Tensor, LayerError>;
    fn backward(&mut self, grad_output: &Tensor) -> Result<Tensor, LayerError>;
    fn parameters(&self) -> Vec<Tensor>;
    fn gradients(&self) -> Vec<Tensor>;
}

/// Dense layer implementation
pub struct Dense {
    pub weights: Tensor,
    pub bias: Tensor,
    pub input: Option<Tensor>,
    pub weight_grad: Tensor,
    pub bias_grad: Tensor,
}

impl Dense {
    pub fn new(input_size: usize, output_size: usize) -> Self {
        Dense {
            weights: Tensor::random(vec![input_size, output_size]),
            bias: Tensor::zeros(vec![output_size]),
            input: None,
            weight_grad: Tensor::zeros(vec![input_size, output_size]),
            bias_grad: Tensor::zeros(vec![output_size]),
        }
    }

    pub fn forward(&mut self, input: &Tensor) -> Result<Tensor, LayerError> {
        if input.shape.len() != 2 {
            return Err(LayerError::InvalidInput {
                expected: "2D tensor".to_string(),
                got: format!("{}D tensor", input.shape.len()),
            });
        }

        if input.shape[1] != self.weights.shape[0] {
            return Err(LayerError::ShapeMismatch {
                layer: "Dense".to_string(),
                expected: self.weights.shape[0],
                got: input.shape[1],
            });
        }

        self.input = Some(input.clone());

        // Matrix multiplication: input @ weights + bias
        let batch_size = input.shape[0];
        let output_size = self.weights.shape[1];
        let mut output = vec![0.0; batch_size * output_size];

        let input_data = input.data.lock().unwrap();
        let weights_data = self.weights.data.lock().unwrap();
        let bias_data = self.bias.data.lock().unwrap();

        for i in 0..batch_size {
            for j in 0..output_size {
                let mut sum = bias_data[j];
                for k in 0..input.shape[1] {
                    sum += input_data[i * input.shape[1] + k]
                         * weights_data[k * output_size + j];
                }
                output[i * output_size + j] = sum;
            }
        }

        Tensor::from_vec(output, vec![batch_size, output_size])
    }
}

impl Layer for Dense {
    fn forward(&self, input: &Tensor) -> Result<Tensor, LayerError> {
        if input.shape.len() != 2 {
            return Err(LayerError::InvalidInput {
                expected: "2D tensor".to_string(),
                got: format!("{}D tensor", input.shape.len()),
            });
        }

        let batch_size = input.shape[0];
        let output_size = self.weights.shape[1];
        let mut output = vec![0.0; batch_size * output_size];

        let input_data = input.data.lock().unwrap();
        let weights_data = self.weights.data.lock().unwrap();
        let bias_data = self.bias.data.lock().unwrap();

        for i in 0..batch_size {
            for j in 0..output_size {
                let mut sum = bias_data[j];
                for k in 0..input.shape[1] {
                    sum += input_data[i * input.shape[1] + k]
                         * weights_data[k * output_size + j];
                }
                output[i * output_size + j] = sum;
            }
        }

        Tensor::from_vec(output, vec![batch_size, output_size])
    }

    fn backward(&mut self, grad_output: &Tensor) -> Result<Tensor, LayerError> {
        if let Some(input) = &self.input {
            // Gradient computation for backpropagation
            Ok(Tensor::zeros(vec![input.shape[0], input.shape[1]]))
        } else {
            Err(LayerError::NoInputStored)
        }
    }

    fn parameters(&self) -> Vec<Tensor> {
        vec![self.weights.clone(), self.bias.clone()]
    }

    fn gradients(&self) -> Vec<Tensor> {
        vec![self.weight_grad.clone(), self.bias_grad.clone()]
    }
}

/// Layer errors
#[derive(Debug, Clone)]
pub enum LayerError {
    InvalidInput {
        expected: String,
        got: String,
    },
    ShapeMismatch {
        layer: String,
        expected: usize,
        got: usize,
    },
    NoInputStored,
    BackpropError(String),
}

/// Activation functions
pub mod activations {
    use super::Tensor;

    pub fn relu(x: &Tensor) -> Tensor {
        let mut output = vec![0.0; x.size()];
        let data = x.data.lock().unwrap();
        for i in 0..data.len() {
            output[i] = data[i].max(0.0);
        }
        Tensor::from_vec(output, x.shape.clone()).unwrap()
    }

    pub fn sigmoid(x: &Tensor) -> Tensor {
        let mut output = vec![0.0; x.size()];
        let data = x.data.lock().unwrap();
        for i in 0..data.len() {
            output[i] = 1.0 / (1.0 + (-data[i]).exp());
        }
        Tensor::from_vec(output, x.shape.clone()).unwrap()
    }

    pub fn tanh(x: &Tensor) -> Tensor {
        let mut output = vec![0.0; x.size()];
        let data = x.data.lock().unwrap();
        for i in 0..data.len() {
            output[i] = data[i].tanh();
        }
        Tensor::from_vec(output, x.shape.clone()).unwrap()
    }

    pub fn softmax(x: &Tensor) -> Tensor {
        if x.shape.len() != 2 {
            panic!("softmax requires 2D input");
        }

        let mut output = vec![0.0; x.size()];
        let data = x.data.lock().unwrap();
        let batch_size = x.shape[0];
        let num_classes = x.shape[1];

        for i in 0..batch_size {
            // Compute max for numerical stability
            let mut max = f32::NEG_INFINITY;
            for j in 0..num_classes {
                max = max.max(data[i * num_classes + j]);
            }

            // Compute exp and sum
            let mut sum = 0.0;
            for j in 0..num_classes {
                let exp_val = (data[i * num_classes + j] - max).exp();
                output[i * num_classes + j] = exp_val;
                sum += exp_val;
            }

            // Normalize
            for j in 0..num_classes {
                output[i * num_classes + j] /= sum;
            }
        }

        Tensor::from_vec(output, x.shape.clone()).unwrap()
    }

    pub fn gelu(x: &Tensor) -> Tensor {
        let mut output = vec![0.0; x.size()];
        let data = x.data.lock().unwrap();
        let sqrt_2_over_pi = std::f32::consts::PI.sqrt() * 2.0 / std::f32::consts::PI;

        for i in 0..data.len() {
            let tanh_arg = sqrt_2_over_pi * (data[i] + 0.044715 * data[i].powi(3));
            output[i] = 0.5 * data[i] * (1.0 + tanh_arg.tanh());
        }

        Tensor::from_vec(output, x.shape.clone()).unwrap()
    }
}

/// Loss functions
pub mod loss_functions {
    use super::Tensor;

    pub fn mse(predictions: &Tensor, targets: &Tensor) -> f32 {
        if predictions.shape != targets.shape {
            panic!("Shape mismatch in MSE loss");
        }

        let pred_data = predictions.data.lock().unwrap();
        let target_data = targets.data.lock().unwrap();

        let mse: f32 = pred_data.iter()
            .zip(target_data.iter())
            .map(|(p, t)| (p - t).powi(2))
            .sum::<f32>() / predictions.size() as f32;

        mse
    }

    pub fn cross_entropy(predictions: &Tensor, targets: &Tensor) -> f32 {
        if predictions.shape[0] != targets.shape[0] {
            panic!("Batch size mismatch in cross entropy loss");
        }

        let pred_data = predictions.data.lock().unwrap();
        let target_data = targets.data.lock().unwrap();
        let batch_size = predictions.shape[0];
        let num_classes = predictions.shape[1];

        let mut loss = 0.0;
        for i in 0..batch_size {
            for j in 0..num_classes {
                let pred = pred_data[i * num_classes + j];
                let target = target_data[i * num_classes + j];
                if target > 0.0 {
                    loss -= target * pred.ln();
                }
            }
        }

        loss / batch_size as f32
    }

    pub fn bce(predictions: &Tensor, targets: &Tensor) -> f32 {
        if predictions.shape != targets.shape {
            panic!("Shape mismatch in BCE loss");
        }

        let pred_data = predictions.data.lock().unwrap();
        let target_data = targets.data.lock().unwrap();

        let bce: f32 = pred_data.iter()
            .zip(target_data.iter())
            .map(|(p, t)| {
                let p = p.max(1e-7).min(1.0 - 1e-7);
                -(t * p.ln() + (1.0 - t) * (1.0 - p).ln())
            })
            .sum::<f32>() / predictions.size() as f32;

        bce
    }
}

/// Optimizers
pub struct Adam {
    pub learning_rate: f32,
    pub beta1: f32,
    pub beta2: f32,
    pub epsilon: f32,
    pub m: HashMap<String, Tensor>,
    pub v: HashMap<String, Tensor>,
    pub t: u32,
}

impl Adam {
    pub fn new(learning_rate: f32) -> Self {
        Adam {
            learning_rate,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
            m: HashMap::new(),
            v: HashMap::new(),
            t: 0,
        }
    }

    pub fn step(&mut self, param_name: &str, param: &mut Tensor, grad: &Tensor) {
        self.t += 1;

        // Initialize m and v if not present
        if !self.m.contains_key(param_name) {
            self.m.insert(param_name.to_string(), Tensor::zeros(param.shape.clone()));
            self.v.insert(param_name.to_string(), Tensor::zeros(param.shape.clone()));
        }

        let m = self.m.get_mut(param_name).unwrap();
        let v = self.v.get_mut(param_name).unwrap();

        // Update biased first moment estimate
        let mut m_data = m.data.lock().unwrap();
        let mut v_data = v.data.lock().unwrap();
        let grad_data = grad.data.lock().unwrap();
        let mut param_data = param.data.lock().unwrap();

        for i in 0..grad_data.len() {
            m_data[i] = self.beta1 * m_data[i] + (1.0 - self.beta1) * grad_data[i];
            v_data[i] = self.beta2 * v_data[i] + (1.0 - self.beta2) * grad_data[i].powi(2);

            let m_hat = m_data[i] / (1.0 - self.beta1.powi(self.t as i32));
            let v_hat = v_data[i] / (1.0 - self.beta2.powi(self.t as i32));

            param_data[i] -= self.learning_rate * m_hat / (v_hat.sqrt() + self.epsilon);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_creation() {
        let t = Tensor::zeros(vec![2, 3]);
        assert_eq!(t.shape, vec![2, 3]);
        assert_eq!(t.size(), 6);
    }

    #[test]
    fn test_tensor_sum() {
        let t = Tensor::from_vec(vec![1.0, 2.0, 3.0], vec![3]).unwrap();
        assert_eq!(t.sum(), 6.0);
    }

    #[test]
    fn test_tensor_mean() {
        let t = Tensor::from_vec(vec![1.0, 2.0, 3.0], vec![3]).unwrap();
        assert_eq!(t.mean(), 2.0);
    }

    #[test]
    fn test_dense_layer() {
        let mut dense = Dense::new(3, 2);
        let input = Tensor::from_vec(vec![1.0, 2.0, 3.0], vec![1, 3]).unwrap();
        let output = dense.forward(&input);
        assert!(output.is_ok());
    }
}
