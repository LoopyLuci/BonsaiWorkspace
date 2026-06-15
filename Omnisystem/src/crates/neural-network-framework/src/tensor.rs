//! Tensor data structure and operations

use crate::error::{Error, Result};
use crate::types::{DType, TensorType};
use std::sync::Arc;
use parking_lot::RwLock;

/// Unique identifier for tensors
pub type TensorId = u64;

static TENSOR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn next_tensor_id() -> TensorId {
    TENSOR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}

/// Tensor - N-dimensional array with automatic differentiation support
pub struct Tensor {
    /// Unique identifier
    id: TensorId,
    /// Shape of the tensor
    shape: Vec<usize>,
    /// Data type
    dtype: DType,
    /// Device placement (cpu, cuda:0, etc.)
    device: String,
    /// Tensor data (stored as f32 for simplicity in Phase 1)
    data: Arc<RwLock<Vec<f32>>>,
    /// Whether gradients are required
    requires_grad: bool,
    /// Gradient tensor (computed during backward pass)
    gradient: Arc<RwLock<Option<Box<Tensor>>>>,
}

impl Tensor {
    /// Create a new tensor with given shape, dtype, and device
    pub fn new(shape: Vec<usize>, dtype: DType, device: &str) -> Result<Self> {
        let numel: usize = shape.iter().product();
        let data = vec![0.0; numel];

        Ok(Tensor {
            id: next_tensor_id(),
            shape,
            dtype,
            device: device.to_string(),
            data: Arc::new(RwLock::new(data)),
            requires_grad: false,
            gradient: Arc::new(RwLock::new(None)),
        })
    }

    /// Create a tensor filled with zeros
    pub fn zeros(shape: Vec<usize>, dtype: DType, device: &str) -> Result<Self> {
        Self::new(shape, dtype, device)
    }

    /// Create a tensor filled with ones
    pub fn ones(shape: Vec<usize>, dtype: DType, device: &str) -> Result<Self> {
        let numel: usize = shape.iter().product();
        let data = vec![1.0; numel];

        Ok(Tensor {
            id: next_tensor_id(),
            shape,
            dtype,
            device: device.to_string(),
            data: Arc::new(RwLock::new(data)),
            requires_grad: false,
            gradient: Arc::new(RwLock::new(None)),
        })
    }

    /// Create a tensor with random values
    pub fn randn(shape: Vec<usize>, dtype: DType, device: &str) -> Result<Self> {
        use rand::Rng;
        let numel: usize = shape.iter().product();
        let mut rng = rand::thread_rng();
        let data: Vec<f32> = (0..numel)
            .map(|_| rng.gen_range(-1.0..1.0))
            .collect();

        Ok(Tensor {
            id: next_tensor_id(),
            shape,
            dtype,
            device: device.to_string(),
            data: Arc::new(RwLock::new(data)),
            requires_grad: false,
            gradient: Arc::new(RwLock::new(None)),
        })
    }

    /// Get tensor ID
    pub fn id(&self) -> TensorId {
        self.id
    }

    /// Get tensor shape
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    /// Get tensor data type
    pub fn dtype(&self) -> DType {
        self.dtype
    }

    /// Get tensor device
    pub fn device(&self) -> &str {
        &self.device
    }

    /// Get total number of elements
    pub fn numel(&self) -> usize {
        self.shape.iter().product()
    }

    /// Get tensor size in bytes
    pub fn size_bytes(&self) -> usize {
        self.numel() * self.dtype.size_bytes()
    }

    /// Check if gradients are required
    pub fn requires_grad(&self) -> bool {
        self.requires_grad
    }

    /// Set requires_grad flag
    pub fn set_requires_grad(&mut self, requires_grad: bool) {
        self.requires_grad = requires_grad;
    }

    /// Get tensor data (read-only)
    pub fn data(&self) -> std::sync::Arc<parking_lot::RwLock<Vec<f32>>> {
        self.data.clone()
    }

    /// Get mutable reference to tensor data
    pub fn data_mut(&mut self) -> std::sync::Arc<parking_lot::RwLock<Vec<f32>>> {
        self.data.clone()
    }

    /// Get gradient
    pub fn gradient(&self) -> std::sync::Arc<parking_lot::RwLock<Option<Box<Tensor>>>> {
        self.gradient.clone()
    }

    /// Set gradient
    pub fn set_gradient(&mut self, grad: Option<Box<Tensor>>) {
        *self.gradient.write() = grad;
    }

    /// Get tensor as TensorType
    pub fn tensor_type(&self) -> TensorType {
        TensorType {
            shape: self.shape.clone(),
            dtype: self.dtype,
            device: self.device.clone(),
            requires_grad: self.requires_grad,
        }
    }

    /// Clone tensor (shallow copy of data)
    pub fn clone(&self) -> Self {
        Tensor {
            id: next_tensor_id(),
            shape: self.shape.clone(),
            dtype: self.dtype,
            device: self.device.clone(),
            data: self.data.clone(),
            requires_grad: self.requires_grad,
            gradient: Arc::new(RwLock::new(None)),
        }
    }

    /// Reshape tensor
    pub fn reshape(&self, new_shape: Vec<usize>) -> Result<Self> {
        let numel_old: usize = self.shape.iter().product();
        let numel_new: usize = new_shape.iter().product();

        if numel_old != numel_new {
            return Err(Error::ShapeMismatch {
                expected: vec![numel_old],
                actual: vec![numel_new],
            });
        }

        Ok(Tensor {
            id: next_tensor_id(),
            shape: new_shape,
            dtype: self.dtype,
            device: self.device.clone(),
            data: self.data.clone(),
            requires_grad: self.requires_grad,
            gradient: Arc::new(RwLock::new(None)),
        })
    }

    /// Add two tensors
    pub fn add(&self, other: &Tensor) -> Result<Self> {
        if self.shape != other.shape {
            return Err(Error::ShapeMismatch {
                expected: self.shape.clone(),
                actual: other.shape.clone(),
            });
        }

        let self_data = self.data.read();
        let other_data = other.data.read();

        let result_data: Vec<f32> = self_data
            .iter()
            .zip(other_data.iter())
            .map(|(a, b)| a + b)
            .collect();

        let mut result = Tensor {
            id: next_tensor_id(),
            shape: self.shape.clone(),
            dtype: self.dtype,
            device: self.device.clone(),
            data: Arc::new(RwLock::new(result_data)),
            requires_grad: self.requires_grad || other.requires_grad,
            gradient: Arc::new(RwLock::new(None)),
        };

        if result.requires_grad {
            // TODO: Store gradient function for backward pass
        }

        Ok(result)
    }

    /// Element-wise multiply
    pub fn mul(&self, other: &Tensor) -> Result<Self> {
        if self.shape != other.shape {
            return Err(Error::ShapeMismatch {
                expected: self.shape.clone(),
                actual: other.shape.clone(),
            });
        }

        let self_data = self.data.read();
        let other_data = other.data.read();

        let result_data: Vec<f32> = self_data
            .iter()
            .zip(other_data.iter())
            .map(|(a, b)| a * b)
            .collect();

        Ok(Tensor {
            id: next_tensor_id(),
            shape: self.shape.clone(),
            dtype: self.dtype,
            device: self.device.clone(),
            data: Arc::new(RwLock::new(result_data)),
            requires_grad: self.requires_grad || other.requires_grad,
            gradient: Arc::new(RwLock::new(None)),
        })
    }

    /// Scalar multiply
    pub fn scalar_mul(&self, scalar: f32) -> Result<Self> {
        let self_data = self.data.read();
        let result_data: Vec<f32> = self_data.iter().map(|a| a * scalar).collect();

        Ok(Tensor {
            id: next_tensor_id(),
            shape: self.shape.clone(),
            dtype: self.dtype,
            device: self.device.clone(),
            data: Arc::new(RwLock::new(result_data)),
            requires_grad: self.requires_grad,
            gradient: Arc::new(RwLock::new(None)),
        })
    }

    /// Sum all elements
    pub fn sum(&self) -> Result<f32> {
        let data = self.data.read();
        Ok(data.iter().sum())
    }

    /// Mean of all elements
    pub fn mean(&self) -> Result<f32> {
        let data = self.data.read();
        let sum: f32 = data.iter().sum();
        Ok(sum / self.numel() as f32)
    }
}

impl Clone for Tensor {
    fn clone(&self) -> Self {
        Tensor::clone(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_creation() {
        let tensor = Tensor::zeros(vec![2, 3, 4], DType::Float32, "cpu")
            .expect("Failed to create tensor");
        assert_eq!(tensor.shape(), &[2, 3, 4]);
        assert_eq!(tensor.numel(), 24);
        assert_eq!(tensor.dtype(), DType::Float32);
    }

    #[test]
    fn test_tensor_add() {
        let t1 = Tensor::ones(vec![2, 3], DType::Float32, "cpu").unwrap();
        let t2 = Tensor::ones(vec![2, 3], DType::Float32, "cpu").unwrap();
        let t3 = t1.add(&t2).unwrap();

        let sum = t3.sum().unwrap();
        assert!((sum - 12.0).abs() < 1e-6);
    }

    #[test]
    fn test_tensor_reshape() {
        let t1 = Tensor::zeros(vec![2, 3], DType::Float32, "cpu").unwrap();
        let t2 = t1.reshape(vec![6]).unwrap();
        assert_eq!(t2.shape(), &[6]);
    }
}
