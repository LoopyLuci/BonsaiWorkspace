// User management

use crate::{Platform, UserId, Capability};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    Viewer,
    Operator,
    Admin,
    Council,
}

impl UserRole {
    pub fn capabilities(&self) -> Vec<Capability> {
        match self {
            UserRole::Viewer => vec![Capability::View],
            UserRole::Operator => vec![
                Capability::View,
                Capability::BugHunterSweep,
                Capability::ModelChat,
                Capability::Deploy,
            ],
            UserRole::Admin => vec![
                Capability::View,
                Capability::BugHunterSweep,
                Capability::ModelChat,
                Capability::Deploy,
                Capability::AdminAccess,
            ],
            UserRole::Council => vec![
                Capability::View,
                Capability::BugHunterSweep,
                Capability::ModelChat,
                Capability::Deploy,
                Capability::AdminAccess,
                Capability::GovernanceVote,
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub role: UserRole,
    pub platform: Platform,
    pub created_at: u64,
}

impl User {
    pub fn new(id: UserId, role: UserRole, platform: Platform) -> Self {
        Self {
            id,
            role,
            platform,
            created_at: chrono::Utc::now().timestamp() as u64,
        }
    }

    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.role.capabilities().contains(capability)
    }

    pub fn promote(&mut self, role: UserRole) {
        self.role = role;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_role_capabilities() {
        let viewer_caps = UserRole::Viewer.capabilities();
        assert!(viewer_caps.contains(&Capability::View));
        assert!(!viewer_caps.contains(&Capability::AdminAccess));

        let admin_caps = UserRole::Admin.capabilities();
        assert!(admin_caps.contains(&Capability::View));
        assert!(admin_caps.contains(&Capability::AdminAccess));
    }

    #[test]
    fn test_user_permissions() {
        let id = UserId::telegram("123");
        let mut user = User::new(id, UserRole::Viewer, Platform::Telegram);
        assert!(!user.has_capability(&Capability::AdminAccess));

        user.promote(UserRole::Admin);
        assert!(user.has_capability(&Capability::AdminAccess));
    }
}
