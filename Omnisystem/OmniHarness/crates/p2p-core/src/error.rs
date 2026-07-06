use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransferError {
    #[error("no lanes available")]
    NoLanes,
    #[error("lane {0} failed: {1}")]
    LaneFailed(String, String),
    #[error("transfer {0} not found")]
    NotFound(uuid::Uuid),
    #[error("transfer cancelled")]
    Cancelled,
    #[error("reassembly gap timeout for GSN {0}")]
    GapTimeout(u64),
    #[error("chunk too large: {0} bytes (max {1})")]
    ChunkTooLarge(usize, usize),
    #[error("crypto error: {0}")]
    Crypto(#[from] p2p_crypto::error::CryptoError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

pub type TransferResult<T> = Result<T, TransferError>;
