//! Error types

#[derive(Debug, Clone)]
pub enum RecoveryError {
    /// Recovery plan not found
    PlanNotFound,
    /// Invalid recovery point
    InvalidRecoveryPoint,
    /// Execution failed
    ExecutionFailed,
    /// Other error
    Other(String),
}

impl std::fmt::Display for RecoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecoveryError::PlanNotFound => write!(f, "recovery plan not found"),
            RecoveryError::InvalidRecoveryPoint => write!(f, "invalid recovery point"),
            RecoveryError::ExecutionFailed => write!(f, "execution failed"),
            RecoveryError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for RecoveryError {}

/// Result type
pub type RecoveryResult<T> = std::result::Result<T, RecoveryError>;
