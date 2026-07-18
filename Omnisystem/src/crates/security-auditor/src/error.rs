//! Error types for the security auditor.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditError {
    /// A security rule was violated or could not be evaluated (e.g. an
    /// unrecognized encryption algorithm was passed to the rule engine).
    RuleViolation(String),
    /// A referenced policy or finding could not be found.
    NotFound(String),
    /// Any other auditor error.
    Other(String),
}

impl std::fmt::Display for AuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditError::RuleViolation(msg) => write!(f, "rule violation: {}", msg),
            AuditError::NotFound(msg) => write!(f, "not found: {}", msg),
            AuditError::Other(msg) => write!(f, "audit error: {}", msg),
        }
    }
}

impl std::error::Error for AuditError {}

/// Result type used throughout the auditor.
pub type Result<T> = std::result::Result<T, AuditError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_messages() {
        assert_eq!(
            AuditError::RuleViolation("bad algo".into()).to_string(),
            "rule violation: bad algo"
        );
    }
}
