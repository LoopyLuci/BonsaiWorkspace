// Capability-based permissions

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    View,
    BugHunterSweep,
    BugHunterFix,
    ModelChat,
    ModelTrain,
    Deploy,
    Scale,
    Rollback,
    AdminAccess,
    GovernanceVote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    pub user_id: String,
    pub capabilities: Vec<Capability>,
    pub issued_at: u64,
    pub expires_at: u64,
    pub signature: String, // Ed25519 signature
}

impl CapabilityToken {
    pub fn is_valid(&self, now: u64) -> bool {
        now < self.expires_at
    }

    pub fn has_capability(&self, cap: &Capability) -> bool {
        self.capabilities.contains(cap)
    }
}

#[derive(Debug, Clone)]
pub struct Permission {
    pub capability: Capability,
    pub resource: Option<String>, // Optional: specific resource
    pub allowed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_token_validity() {
        let token = CapabilityToken {
            user_id: "user1".into(),
            capabilities: vec![Capability::View],
            issued_at: 1000,
            expires_at: 2000,
            signature: "sig".into(),
        };

        assert!(token.is_valid(1500));
        assert!(!token.is_valid(2500));
    }

    #[test]
    fn test_capability_check() {
        let token = CapabilityToken {
            user_id: "user1".into(),
            capabilities: vec![Capability::View, Capability::ModelChat],
            issued_at: 1000,
            expires_at: 2000,
            signature: "sig".into(),
        };

        assert!(token.has_capability(&Capability::View));
        assert!(token.has_capability(&Capability::ModelChat));
        assert!(!token.has_capability(&Capability::AdminAccess));
    }
}
