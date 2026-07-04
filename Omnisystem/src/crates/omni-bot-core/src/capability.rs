//! Capability-based security model

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};

/// A capability token that authorizes specific actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    pub id: String,
    pub user_id: String,
    pub capabilities: Vec<String>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub signature: String,  // BLAKE3 signature
}

impl CapabilityToken {
    /// Create a new capability token
    pub fn new(user_id: String, capabilities: Vec<String>) -> Self {
        let now = Utc::now();
        let expires_at = now + Duration::hours(24);

        let id = format!("cap-{}", uuid::Uuid::new_v4());
        let hash = blake3::hash(id.as_bytes());
        let signature = hash.to_hex().to_string();
        
        Self {
            id,
            user_id,
            capabilities,
            issued_at: now,
            expires_at,
            signature,
        }
    }
    
    /// Check if token is still valid
    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }
    
    /// Check if token grants a specific capability
    pub fn has_capability(&self, capability: &str) -> bool {
        if !self.is_valid() {
            return false;
        }

        self.capabilities.iter().any(|c| {
            if c == "*" {
                // Global wildcard grants everything
                return true;
            }
            if c == capability {
                // Exact match
                return true;
            }
            if c.ends_with(":*") {
                // Prefix wildcard (e.g., "SERVICE:*" grants "SERVICE:start")
                let prefix = &c[..c.len() - 1]; // Remove the '*'
                return capability.starts_with(prefix);
            }
            false
        })
    }
}

/// User capability set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub token: CapabilityToken,
}

impl Capability {
    pub fn new(user_id: String, capabilities: Vec<String>) -> Self {
        Self {
            token: CapabilityToken::new(user_id, capabilities),
        }
    }
    
    pub fn is_valid(&self) -> bool {
        self.token.is_valid()
    }
    
    pub fn can(&self, capability: &str) -> bool {
        self.token.has_capability(capability)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_capability_token_creation() {
        let token = CapabilityToken::new(
            "alice".to_string(),
            vec!["SERVICE:start".to_string()],
        );
        assert!(token.is_valid());
        assert!(token.has_capability("SERVICE:start"));
        assert!(!token.has_capability("SERVICE:destroy"));
    }
    
    #[test]
    fn test_capability_wildcard() {
        let token = CapabilityToken::new(
            "alice".to_string(),
            vec!["SERVICE:*".to_string()],
        );
        assert!(token.has_capability("SERVICE:start"));
        assert!(token.has_capability("SERVICE:stop"));
        assert!(!token.has_capability("ENVIRONMENT:create"));
    }
}
