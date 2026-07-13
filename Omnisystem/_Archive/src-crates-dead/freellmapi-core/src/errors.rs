use thiserror::Error;

#[derive(Error, Debug)]
pub enum FreeLLMAPIError {
    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Tenant not found")]
    TenantNotFound,

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Budget exceeded")]
    BudgetExceeded,

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Internal server error: {0}")]
    InternalServerError(String),

    #[error("Service not found: {0}")]
    ServiceNotFound(String),
}

pub type Result<T> = std::result::Result<T, FreeLLMAPIError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = FreeLLMAPIError::InvalidApiKey;
        assert_eq!(err.to_string(), "Invalid API key");
    }

    #[test]
    fn test_rate_limit_error() {
        let err = FreeLLMAPIError::RateLimitExceeded("100 req/min".to_string());
        assert!(err.to_string().contains("Rate limit exceeded"));
    }
}
