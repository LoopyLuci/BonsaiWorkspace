//! Privacy-specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum PrivacyError {
    #[error("sensitive data detected")]
    SensitiveDataDetected,
    #[error("consent missing")]
    ConsentMissing,
    #[error("deletion failed")]
    DeletionFailed,
}

pub type PrivacyResult<T> = std::result::Result<T, PrivacyError>;
