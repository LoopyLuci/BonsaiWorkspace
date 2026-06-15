//! Error types for the formal verifier
//!
//! Provides comprehensive error handling for parsing, validation, consistency checking,
//! and axiom proof verification failures.

use thiserror::Error;
use serde::{Deserialize, Serialize};

/// Result type for verifier operations
pub type VerifyResult<T> = Result<T, VerifierError>;

/// Comprehensive error type for formal verifier operations
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum VerifierError {
    /// Output parsing failed (malformed JSON, invalid format)
    #[error("Output parsing failed: {reason}")]
    ParseError { reason: String },

    /// Schema validation failed (type mismatch, constraint violation)
    #[error("Schema validation failed: {reason}")]
    SchemaValidation { reason: String },

    /// Constraint violation (invariant check failed)
    #[error("Constraint violation: {reason}")]
    ConstraintViolation { reason: String },

    /// Session consistency violated (contradiction detected)
    #[error("Session consistency violated: {reason}")]
    ConsistencyViolation { reason: String },

    /// Temporal constraint violation (ordering constraint failed)
    #[error("Temporal constraint violation: {reason}")]
    TemporalViolation { reason: String },

    /// Axiom proof verification failed
    #[error("Axiom proof verification failed: {reason}")]
    ProofVerificationFailed { reason: String },

    /// Proof hash mismatch (proof tampering detected)
    #[error("Proof hash mismatch: {reason}")]
    ProofHashMismatch { reason: String },

    /// Axiom theorem not found
    #[error("Axiom theorem not found: {reason}")]
    TheoremNotFound { reason: String },

    /// Invalid configuration (missing schema, invalid constraints)
    #[error("Invalid configuration: {reason}")]
    InvalidConfiguration { reason: String },

    /// Serialization error (serde failure)
    #[error("Serialization error: {reason}")]
    SerializationError { reason: String },

    /// Timeout (operation exceeded time limit)
    #[error("Timeout: {reason}")]
    Timeout { reason: String },

    /// Internal error (unexpected state)
    #[error("Internal error: {reason}")]
    Internal { reason: String },

    /// Conversion to AhfError for integration with core framework
    #[error("Framework error: {reason}")]
    FrameworkError { reason: String },
}

impl VerifierError {
    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::ParseError { .. }
                | Self::SchemaValidation { .. }
                | Self::ConstraintViolation { .. }
                | Self::Timeout { .. }
        )
    }

    /// Check if this error requires immediate escalation
    pub fn requires_escalation(&self) -> bool {
        matches!(
            self,
            Self::ConsistencyViolation { .. }
                | Self::ProofVerificationFailed { .. }
                | Self::ProofHashMismatch { .. }
                | Self::TemporalViolation { .. }
        )
    }

    /// Convert to AhfError for integration with core framework
    pub fn to_ahf_error(&self) -> ahf_core::AhfError {
        ahf_core::AhfError::VerificationFailed {
            reason: self.to_string(),
        }
    }
}

impl From<serde_json::Error> for VerifierError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError {
            reason: err.to_string(),
        }
    }
}

impl From<jsonschema::error::ValidationError<'_>> for VerifierError {
    fn from(err: jsonschema::error::ValidationError) -> Self {
        Self::SchemaValidation {
            reason: err.to_string(),
        }
    }
}

impl From<VerifierError> for ahf_core::AhfError {
    fn from(err: VerifierError) -> Self {
        err.to_ahf_error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_creation() {
        let err = VerifierError::ParseError {
            reason: "invalid JSON".to_string(),
        };
        assert!(err.is_recoverable());
        assert!(!err.requires_escalation());
    }

    #[test]
    fn test_consistency_violation_escalation() {
        let err = VerifierError::ConsistencyViolation {
            reason: "contradicts prior fact".to_string(),
        };
        assert!(!err.is_recoverable());
        assert!(err.requires_escalation());
    }

    #[test]
    fn test_error_serialization() {
        let err = VerifierError::SchemaValidation {
            reason: "type mismatch".to_string(),
        };
        let json = serde_json::to_string(&err).expect("serialization failed");
        assert!(json.contains("type mismatch"));
    }

    #[test]
    fn test_to_ahf_error() {
        let err = VerifierError::ParseError {
            reason: "malformed".to_string(),
        };
        let ahf_err = err.to_ahf_error();
        assert!(matches!(
            ahf_err,
            ahf_core::AhfError::VerificationFailed { .. }
        ));
    }

    #[test]
    fn test_serde_json_error_conversion() {
        let json_err: VerifierError =
            serde_json::from_str::<serde_json::Value>("invalid").unwrap_err().into();
        assert!(matches!(json_err, VerifierError::SerializationError { .. }));
    }

    #[test]
    fn test_error_display() {
        let err = VerifierError::ConsistencyViolation {
            reason: "X contradicts Y".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("contradicts"));
    }
}
