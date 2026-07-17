//! Event-system specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum EventError {
    #[error("event not found")]
    EventNotFound,
    #[error("unsubscribe failed")]
    UnsubscribeFailed,
}

pub type EventResult<T> = std::result::Result<T, EventError>;
