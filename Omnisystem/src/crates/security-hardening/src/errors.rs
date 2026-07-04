use thiserror::Error;

pub type Result<T> = std::result::Result<T, SecurityError>;

#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("SBOM generation failed: {0}")]
    SbomGenerationFailed(String),

    #[error("Secret detected: {0}")]
    SecretDetected(String),

    #[error("Vulnerability found: {0}")]
    VulnerabilityFound(String),

    #[error("Supply chain verification failed: {0}")]
    SupplyChainVerificationFailed(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Key management error: {0}")]
    KeyManagementError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Unknown error")]
    Unknown,
}
