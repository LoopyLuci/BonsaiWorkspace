//! Universal Language Layer Error Handling

use thiserror::Error;
use std::fmt;

/// ULL Result type
pub type Result<T> = std::result::Result<T, UllError>;

/// Universal Language Layer errors
#[derive(Error, Debug, Clone)]
pub enum UllError {
    #[error("Language not found: {0}")]
    LanguageNotFound(String),

    #[error("Language not initialized: {0}")]
    LanguageNotInitialized(String),

    #[error("FFI error: {0}")]
    FfiError(String),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Type conversion error: {0}")]
    TypeConversion(String),

    #[error("Function not found: {0}")]
    FunctionNotFound(String),

    #[error("Bridge error: {0}")]
    BridgeError(String),

    #[error("Memory error: {0}")]
    MemoryError(String),

    #[error("Async error: {0}")]
    AsyncError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl UllError {
    /// Create a new FFI error
    pub fn ffi(msg: impl Into<String>) -> Self {
        Self::FfiError(msg.into())
    }

    /// Create a new type mismatch error
    pub fn type_mismatch(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::TypeMismatch {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create a new type conversion error
    pub fn type_conversion(msg: impl Into<String>) -> Self {
        Self::TypeConversion(msg.into())
    }

    /// Create a new bridge error
    pub fn bridge(msg: impl Into<String>) -> Self {
        Self::BridgeError(msg.into())
    }

    /// Create a new language not found error
    pub fn language_not_found(lang: impl Into<String>) -> Self {
        Self::LanguageNotFound(lang.into())
    }

    /// Get error code for FFI/network transmission
    pub fn code(&self) -> u32 {
        match self {
            Self::LanguageNotFound(_) => 1001,
            Self::LanguageNotInitialized(_) => 1002,
            Self::FfiError(_) => 2001,
            Self::TypeMismatch { .. } => 3001,
            Self::TypeConversion(_) => 3002,
            Self::FunctionNotFound(_) => 4001,
            Self::BridgeError(_) => 5001,
            Self::MemoryError(_) => 6001,
            Self::AsyncError(_) => 7001,
            Self::SerializationError(_) => 8001,
            Self::RuntimeError(_) => 9001,
            Self::ConfigError(_) => 9002,
            Self::PermissionDenied(_) => 9003,
            Self::Timeout(_) => 9004,
            Self::InvalidArgument(_) => 9005,
            Self::Internal(_) => 9999,
        }
    }

    /// Convert to error code and message for FFI
    pub fn to_ffi_error(&self) -> (u32, String) {
        (self.code(), self.to_string())
    }
}

/// FFI-safe error representation for cross-language calls
#[repr(C)]
pub struct FfiError {
    pub code: u32,
    pub message: *const u8,
}

impl From<UllError> for FfiError {
    fn from(err: UllError) -> Self {
        let (code, msg) = err.to_ffi_error();
        let message = Box::leak(msg.into_bytes().into_boxed_slice());
        Self {
            code,
            message: message.as_ptr(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(UllError::LanguageNotFound("test".to_string()).code(), 1001);
        assert_eq!(UllError::FfiError("test".to_string()).code(), 2001);
        assert_eq!(UllError::TypeMismatch {
            expected: "u32".to_string(),
            actual: "String".to_string()
        }.code(), 3001);
    }
}
