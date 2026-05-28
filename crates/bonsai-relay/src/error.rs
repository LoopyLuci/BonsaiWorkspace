use thiserror::Error;

#[derive(Debug, Error)]
pub enum RelayError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid or expired relay token")]
    InvalidToken,
    #[error("session not found")]
    SessionNotFound,
    #[error("session full — both peers already connected")]
    SessionFull,
    #[error("proof-of-work check failed")]
    PowFailed,
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("frame too large: {0} bytes")]
    FrameTooLarge(usize),
}

pub type RelayResult<T> = Result<T, RelayError>;
