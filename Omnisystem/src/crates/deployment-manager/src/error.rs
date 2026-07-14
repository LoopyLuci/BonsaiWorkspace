//! Deployment-manager specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum DeploymentError {
    #[error("status check failed")]
    StatusCheckFailed,
}

pub type DeploymentResult<T> = std::result::Result<T, DeploymentError>;
