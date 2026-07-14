//! Launcher-specific error types.

use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Error)]
pub enum LauncherError {
    #[error("session not found: {0}")]
    SessionNotFound(Uuid),
}

pub type LauncherResult<T> = std::result::Result<T, LauncherError>;
