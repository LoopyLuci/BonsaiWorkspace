//! Protocol-specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ProtocolError {
    #[error("protocol not found")]
    ProtocolNotFound,
    #[error("required capability missing")]
    CapabilityMissing,
    #[error("protocol selection failed")]
    SelectionFailed,
}

pub type ProtocolResult<T> = std::result::Result<T, ProtocolError>;
