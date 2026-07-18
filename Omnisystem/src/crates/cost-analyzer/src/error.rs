//! Error types

#[derive(Debug, Clone)]
pub enum CostError {
    /// No pricing model has been registered for the requested resource type
    PricingNotFound,
    /// Other error
    Other(String),
}

impl std::fmt::Display for CostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CostError::PricingNotFound => write!(f, "no pricing model registered for this resource type"),
            CostError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for CostError {}

/// Result type
pub type CostResult<T> = std::result::Result<T, CostError>;
