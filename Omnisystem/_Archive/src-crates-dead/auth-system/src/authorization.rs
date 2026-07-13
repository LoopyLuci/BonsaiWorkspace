use crate::{AuthError, AuthResult, Permission};
use dashmap::DashMap;
use std::sync::Arc;

pub struct AuthorizationManager {
    permissions: Arc<DashMap<String, Vec<Permission>>>,
}

impl AuthorizationManager {
    pub fn new() -> Self {
        Self {
            permissions: Arc::new(DashMap::new()),
        }
    }

    pub async fn grant_permission(&self, user_id: &str, permission: &Permission) -> AuthResult<()> {
        self.permissions
            .entry(user_id.to_string())
            .or_insert_with(Vec::new)
            .push(permission.clone());
        Ok(())
    }

    pub async fn check_permission(&self, user_id: &str, resource: &str, action: &str) -> AuthResult<bool> {
        if let Some(perms) = self.permissions.get(user_id) {
            Ok(perms.iter().any(|p| p.resource == resource && p.action == action))
        } else {
            Ok(false)
        }
    }

    pub async fn revoke_permission(&self, user_id: &str, resource: &str) -> AuthResult<()> {
        if let Some(mut perms) = self.permissions.get_mut(user_id) {
            perms.retain(|p| p.resource != resource);
            Ok(())
        } else {
            Err(AuthError::PermissionDenied)
        }
    }

    pub fn permission_count(&self) -> usize {
        self.permissions.len()
    }
}

impl Default for AuthorizationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_grant_permission() {
        let manager = AuthorizationManager::new();
        let perm = Permission {
            permission_id: "p1".to_string(),
            resource: "/api/users".to_string(),
            action: "read".to_string(),
        };

        manager.grant_permission("u1", &perm).await.unwrap();
        assert_eq!(manager.permission_count(), 1);
    }

    #[tokio::test]
    async fn test_check_permission() {
        let manager = AuthorizationManager::new();
        let perm = Permission {
            permission_id: "p1".to_string(),
            resource: "/api/users".to_string(),
            action: "read".to_string(),
        };

        manager.grant_permission("u1", &perm).await.unwrap();
        let allowed = manager.check_permission("u1", "/api/users", "read").await.unwrap();
        assert!(allowed);
    }

    #[tokio::test]
    async fn test_revoke_permission() {
        let manager = AuthorizationManager::new();
        let perm = Permission {
            permission_id: "p1".to_string(),
            resource: "/api/users".to_string(),
            action: "read".to_string(),
        };

        manager.grant_permission("u1", &perm).await.unwrap();
        manager.revoke_permission("u1", "/api/users").await.unwrap();
        let allowed = manager.check_permission("u1", "/api/users", "read").await.unwrap();
        assert!(!allowed);
    }
}
