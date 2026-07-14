//! Error types

#[derive(Debug, Clone)]
pub enum Error {
    /// Other error
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

/// Result type
pub type Result<T> = std::result::Result<T, Error>;

/// Errors raised by the encryption engine and key manager
#[derive(Debug, Clone)]
pub enum EncryptionError {
    /// The supplied key or plaintext was invalid
    InvalidKey,
    /// Decryption of the given ciphertext failed
    DecryptionFailed,
    /// No key found for the given key id
    KeyNotFound,
    /// Other error
    Other(String),
}

impl std::fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptionError::InvalidKey => write!(f, "invalid key or plaintext"),
            EncryptionError::DecryptionFailed => write!(f, "decryption failed"),
            EncryptionError::KeyNotFound => write!(f, "key not found"),
            EncryptionError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for EncryptionError {}

/// Result type for encryption operations
pub type EncryptionResult<T> = std::result::Result<T, EncryptionError>;
