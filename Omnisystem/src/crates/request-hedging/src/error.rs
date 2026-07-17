//! Request-hedging specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum HedgingError {
    #[error("consensus not reached")]
    ConsensusNotReached,
    #[error("invalid max hedges")]
    InvalidMaxHedges,
    #[error("invalid hedge delay")]
    InvalidHedgeDelay,
    #[error("invalid configuration: {0}")]
    InvalidConfiguration(String),
    #[error("internal error: {0}")]
    Internal(String),
}

pub type HedgingResult<T> = std::result::Result<T, HedgingError>;
