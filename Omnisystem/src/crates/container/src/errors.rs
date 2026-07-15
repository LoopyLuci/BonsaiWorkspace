use thiserror::Error;

pub type Result<T> = std::result::Result<T, BcfError>;

#[derive(Error, Debug)]
pub enum BcfError {
    #[error("Blueprint validation error: {0}")]
    BlueprintValidation(String),

    #[error("Image error: {0}")]
    ImageError(String),

    #[error("Scheduling error: {0}")]
    SchedulingError(String),

    #[error("Vault error: {0}")]
    VaultError(String),

    #[error("Networking error: {0}")]
    NetworkingError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    #[error("Container not found: {0}")]
    ContainerNotFound(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    #[error("Not enough memory")]
    InsufficientMemory,

    #[error("Not enough CPU")]
    InsufficientCpu,

    #[error("No healthy endpoints")]
    NoHealthyEndpoints,

    #[error("Circuit breaker open")]
    CircuitBreakerOpen,

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Hash mismatch")]
    HashMismatch,

    #[error("Timeout")]
    Timeout,

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Unknown error")]
    Unknown,
}
