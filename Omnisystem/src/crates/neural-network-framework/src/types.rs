//! Type system for Neural Network Framework

use serde::{Deserialize, Serialize};

/// Tensor data type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DType {
    /// 32-bit floating point
    Float32,
    /// 16-bit floating point
    Float16,
    /// Brain floating point
    BFloat16,
    /// 64-bit floating point
    Float64,
    /// 32-bit integer
    Int32,
    /// 64-bit integer
    Int64,
    /// 8-bit integer
    Int8,
    /// 8-bit unsigned integer
    UInt8,
    /// Boolean
    Bool,
}

impl DType {
    /// Get size in bytes
    pub fn size_bytes(&self) -> usize {
        match self {
            DType::Float32 | DType::Int32 => 4,
            DType::Float16 | DType::BFloat16 => 2,
            DType::Float64 | DType::Int64 => 8,
            DType::Int8 | DType::UInt8 | DType::Bool => 1,
        }
    }

    /// Check if dtype is floating point
    pub fn is_float(&self) -> bool {
        matches!(
            self,
            DType::Float32 | DType::Float16 | DType::BFloat16 | DType::Float64
        )
    }

    /// Check if dtype is integer
    pub fn is_integer(&self) -> bool {
        matches!(self, DType::Int32 | DType::Int64 | DType::Int8 | DType::UInt8)
    }

    /// Promote two dtypes to a common dtype
    pub fn promote(&self, other: &DType) -> DType {
        match (self, other) {
            // Same dtype
            (a, b) if a == b => *a,
            // Float promotion
            (DType::Float64, _) | (_, DType::Float64) => DType::Float64,
            (DType::Float32, _) | (_, DType::Float32) => DType::Float32,
            (DType::Float16, _) | (_, DType::Float16) => DType::Float16,
            // Integer promotion
            (DType::Int64, _) | (_, DType::Int64) => DType::Int64,
            (DType::Int32, _) | (_, DType::Int32) => DType::Int32,
            // Mixed float/int → float
            (_, _) if self.is_float() || other.is_float() => DType::Float32,
            // Default
            (a, _) => *a,
        }
    }
}

impl std::fmt::Display for DType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DType::Float32 => write!(f, "float32"),
            DType::Float16 => write!(f, "float16"),
            DType::BFloat16 => write!(f, "bfloat16"),
            DType::Float64 => write!(f, "float64"),
            DType::Int32 => write!(f, "int32"),
            DType::Int64 => write!(f, "int64"),
            DType::Int8 => write!(f, "int8"),
            DType::UInt8 => write!(f, "uint8"),
            DType::Bool => write!(f, "bool"),
        }
    }
}

/// Tensor type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorType {
    /// Tensor shape
    pub shape: Vec<usize>,
    /// Tensor data type
    pub dtype: DType,
    /// Device (cpu, cuda:0, etc.)
    pub device: String,
    /// Whether gradients are required
    pub requires_grad: bool,
}

impl TensorType {
    /// Create a new tensor type
    pub fn new(shape: Vec<usize>, dtype: DType, device: String) -> Self {
        TensorType {
            shape,
            dtype,
            device,
            requires_grad: false,
        }
    }

    /// Get total number of elements
    pub fn numel(&self) -> usize {
        self.shape.iter().product()
    }

    /// Get size in bytes
    pub fn size_bytes(&self) -> usize {
        self.numel() * self.dtype.size_bytes()
    }

    /// Check if shapes are broadcastable
    pub fn can_broadcast(&self, other: &TensorType) -> bool {
        let (shape1, shape2) = (&self.shape, &other.shape);
        let max_ndim = std::cmp::max(shape1.len(), shape2.len());

        for i in 0..max_ndim {
            let i1 = if shape1.len() > i {
                shape1[shape1.len() - 1 - i]
            } else {
                1
            };
            let i2 = if shape2.len() > i {
                shape2[shape2.len() - 1 - i]
            } else {
                1
            };

            if i1 != i2 && i1 != 1 && i2 != 1 {
                return false;
            }
        }

        true
    }

    /// Compute broadcast shape
    pub fn broadcast_shape(&self, other: &TensorType) -> Option<Vec<usize>> {
        if !self.can_broadcast(other) {
            return None;
        }

        let shape1 = &self.shape;
        let shape2 = &other.shape;
        let max_ndim = std::cmp::max(shape1.len(), shape2.len());

        let mut result = vec![1; max_ndim];

        for i in 0..max_ndim {
            let i1 = if shape1.len() > i {
                shape1[shape1.len() - 1 - i]
            } else {
                1
            };
            let i2 = if shape2.len() > i {
                shape2[shape2.len() - 1 - i]
            } else {
                1
            };

            result[max_ndim - 1 - i] = std::cmp::max(i1, i2);
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dtype_promotion() {
        assert_eq!(
            DType::Float32.promote(&DType::Int32),
            DType::Float32
        );
        assert_eq!(
            DType::Float16.promote(&DType::Float32),
            DType::Float32
        );
    }

    #[test]
    fn test_broadcasting() {
        let t1 = TensorType::new(vec![3, 1, 4], DType::Float32, "cpu".to_string());
        let t2 = TensorType::new(vec![1, 5, 4], DType::Float32, "cpu".to_string());

        assert!(t1.can_broadcast(&t2));
        assert_eq!(
            t1.broadcast_shape(&t2),
            Some(vec![3, 5, 4])
        );
    }
}
