use crate::Result;

pub struct RuleEngine;

impl RuleEngine {
    pub fn check_encryption_strength(algorithm: &str) -> Result<bool> {
        match algorithm {
            "AES-256" | "ChaCha20" => Ok(true),
            "AES-128" => Ok(false),
            _ => Err(crate::AuditError::RuleViolation("Unknown algorithm".to_string())),
        }
    }

    pub fn check_password_complexity(password: &str) -> bool {
        password.len() >= 12
            && password.chars().any(|c| c.is_uppercase())
            && password.chars().any(|c| c.is_numeric())
            && password.chars().any(|c| !c.is_alphanumeric())
    }

    pub fn validate_certificate_expiry(days_remaining: u32) -> bool {
        days_remaining > 30
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_check() {
        assert!(RuleEngine::check_encryption_strength("AES-256").unwrap());
        assert!(!RuleEngine::check_encryption_strength("AES-128").unwrap());
    }

    #[test]
    fn test_password_complexity() {
        assert!(RuleEngine::check_password_complexity("MyP@ssw0rd123"));
        assert!(!RuleEngine::check_password_complexity("weak"));
    }
}
