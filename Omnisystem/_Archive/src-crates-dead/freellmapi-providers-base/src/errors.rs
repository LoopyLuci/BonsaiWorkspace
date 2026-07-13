use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Model not supported: {0}")]
    ModelNotSupported(String),

    #[error("Request translation failed: {0}")]
    TranslationFailed(String),

    #[error("Provider API error: {0}")]
    ApiError(String),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Timeout")]
    Timeout,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Unknown provider error")]
    Unknown,
}
