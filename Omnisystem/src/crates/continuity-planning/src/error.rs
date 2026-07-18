//! Error types for continuity planning

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ContinuityError {
    #[error("plan not found: {0}")]
    PlanNotFound(uuid::Uuid),

    #[error("compliance check failed: plan does not exist")]
    ComplianceCheckFailed,

    #[error("other error: {0}")]
    Other(String),
}

pub type ContinuityResult<T> = std::result::Result<T, ContinuityError>;
