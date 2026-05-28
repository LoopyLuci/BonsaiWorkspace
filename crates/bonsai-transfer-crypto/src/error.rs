use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("key generation failed: {0}")]
    KeyGenFailed(String),
    #[error("handshake failed: {0}")]
    HandshakeFailed(String),
    #[error("encryption error: {0}")]
    EncryptionError(String),
    #[error("decryption error: authentication tag mismatch")]
    DecryptionFailed,
    #[error("invalid key material: {0}")]
    InvalidKey(String),
    #[error("KDF error: {0}")]
    KdfError(String),
    #[error("invalid mnemonic: {0}")]
    InvalidMnemonic(String),
    #[error("serialization error: {0}")]
    SerError(String),
}

pub type CryptoResult<T> = Result<T, CryptoError>;
