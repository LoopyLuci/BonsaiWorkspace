//! Error types

#[derive(Debug, Clone)]
pub enum ComplianceError {
    /// Referenced policy does not exist
    PolicyNotFound,
    /// Other error
    Other(String),
}

impl std::fmt::Display for ComplianceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComplianceError::PolicyNotFound => write!(f, "policy not found"),
            ComplianceError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for ComplianceError {}

/// Result type
pub type ComplianceResult<T> = std::result::Result<T, ComplianceError>;
