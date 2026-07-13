use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    pub user_id: String,
    pub can_read_files: bool,
    pub can_execute_commands: bool,
    pub can_access_system: bool,
}

pub struct SecurityManager {
    policies: Arc<DashMap<String, SecurityPolicy>>,
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(DashMap::new()),
        }
    }

    pub fn set_policy(&self, user_id: String, policy: SecurityPolicy) {
        self.policies.insert(user_id, policy);
    }

    pub fn get_policy(&self, user_id: &str) -> Option<SecurityPolicy> {
        self.policies.get(user_id).map(|p| p.clone())
    }

    pub fn check_permission(&self, user_id: &str, permission: &str) -> bool {
        if let Some(policy) = self.policies.get(user_id) {
            match permission {
                "read_files" => policy.can_read_files,
                "execute_commands" => policy.can_execute_commands,
                "access_system" => policy.can_access_system,
                _ => false,
            }
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_policy() {
        let sm = SecurityManager::new();
        let policy = SecurityPolicy {
            user_id: "user1".to_string(),
            can_read_files: true,
            can_execute_commands: false,
            can_access_system: false,
        };
        sm.set_policy("user1".to_string(), policy);
        assert!(sm.check_permission("user1", "read_files"));
        assert!(!sm.check_permission("user1", "execute_commands"));
    }
}
