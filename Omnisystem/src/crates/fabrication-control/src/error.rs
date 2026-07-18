//! Error types for fabrication-control.

#[derive(Debug, Clone)]
pub enum FabricationError {
    /// Device lookup/registration failure.
    DeviceError(String),
    /// Job lookup/state failure.
    InvalidJob(String),
    /// Requested material has no known spec.
    UnsupportedMaterial(String),
    /// Catch-all for other errors.
    Other(String),
}

impl std::fmt::Display for FabricationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FabricationError::DeviceError(msg) => write!(f, "device error: {}", msg),
            FabricationError::InvalidJob(msg) => write!(f, "invalid job: {}", msg),
            FabricationError::UnsupportedMaterial(msg) => write!(f, "unsupported material: {}", msg),
            FabricationError::Other(msg) => write!(f, "fabrication error: {}", msg),
        }
    }
}

impl std::error::Error for FabricationError {}

/// Result type used throughout fabrication-control.
pub type Result<T> = std::result::Result<T, FabricationError>;
