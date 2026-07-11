//! Error types

#[derive(Debug, Clone)]
pub enum ChargebackError {
    /// Invalid allocation percentage
    InvalidAllocationPercentage,
    /// Invoice generation failed
    InvoiceGenerationFailed,
    /// Other error
    Other(String),
}

impl std::fmt::Display for ChargebackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChargebackError::InvalidAllocationPercentage => write!(f, "invalid allocation percentage"),
            ChargebackError::InvoiceGenerationFailed => write!(f, "invoice generation failed"),
            ChargebackError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for ChargebackError {}

/// Result type
pub type ChargebackResult<T> = std::result::Result<T, ChargebackError>;
