//! Error types for the emulation engine

use thiserror::Error;

/// Result type for emulation operations
pub type EmulationResult<T> = Result<T, EmulationError>;

/// Emulation engine errors
#[derive(Debug, Clone, Error)]
pub enum EmulationError {
    /// Memory access out of bounds
    #[error("Memory address out of bounds: {0:#x}")]
    AddressOutOfBounds(u64),

    /// Invalid core ID
    #[error("Invalid core ID: {0}")]
    InvalidCoreId(usize),

    /// Invalid interface ID
    #[error("Invalid network interface ID: {0}")]
    InvalidInterfaceId(usize),

    /// Emulation not running
    #[error("Emulation not running")]
    NotRunning,

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// Storage device error
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(String),

    /// Cache error
    #[error("Cache error: {0}")]
    CacheError(String),

    /// Timeout
    #[error("Timeout")]
    Timeout,

    /// Generic emulation error
    #[error("Emulation error: {0}")]
    Other(String),
}

impl From<std::io::Error> for EmulationError {
    fn from(err: std::io::Error) -> Self {
        EmulationError::IoError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_out_of_bounds_error() {
        let err = EmulationError::AddressOutOfBounds(0xdeadbeef);
        assert!(err.to_string().contains("deadbeef"));
    }

    #[test]
    fn test_invalid_core_id_error() {
        let err = EmulationError::InvalidCoreId(99);
        assert!(err.to_string().contains("99"));
    }
}
