//! Error types and handling for Neural Network Framework

use thiserror::Error;

/// Result type for NNF operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for Neural Network Framework
#[derive(Error, Debug)]
pub enum Error {
    #[error("Shape mismatch: expected {expected:?}, got {actual:?}")]
    ShapeMismatch {
        expected: Vec<usize>,
        actual: Vec<usize>,
    },

    #[error("Dtype mismatch: expected {expected}, got {actual}")]
    DtypeMismatch { expected: String, actual: String },

    #[error("Operation not found: {0}")]
    OperationNotFound(String),

    #[error("Device error: {0}")]
    DeviceError(String),

    #[error("Allocation failed: requested {bytes} bytes")]
    AllocationFailed { bytes: usize },

    #[error("Out of memory")]
    OutOfMemory,

    #[error("Broadcasting error: {0}")]
    BroadcastingError(String),

    #[error("Gradient computation error: {0}")]
    GradientError(String),

    #[error("Type inference error: {0}")]
    TypeInferenceError(String),

    #[error("Invalid graph: {0}")]
    InvalidGraph(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Unknown error: {0}")]
    Other(String),
}

/// Convert from anyhow::Error
impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Other(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_mismatch_error() {
        let err = Error::ShapeMismatch {
            expected: vec![2, 3],
            actual: vec![3, 2],
        };
        assert!(err.to_string().contains("Shape mismatch"));
    }

    #[test]
    fn test_out_of_memory_error() {
        let err = Error::OutOfMemory;
        assert_eq!(err.to_string(), "Out of memory");
    }
}
