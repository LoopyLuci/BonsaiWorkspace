/// Role-Based Access Control (RBAC)
///
/// Fine-grained permission management with role hierarchy

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::info;

/// Permissions (fine-grained capabilities)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    // Cluster management
    NodeJoin,
    NodeLeave,
    NodeRemove,

    // Data operations
    Read,
    Write,
    Delete,

    // Leadership
    ElectLeader,
    VoteOnLeader,

    // Replication
    Replicate,
    Restore,

    // Backup
    CreateBackup,
    DeleteBackup,

    // Security
    ManageCertificates,
    ManageKeys,
}

/// Roles with associated permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    Admin,      // All permissions
    Leader,     // Leadership + replication + read/write
    Replica,    // Replication + read only
    Auditor,    // Read only
    Guest,      // Minimal permissions
}

impl Role {
    /// Get permissions for role
    pub fn permissions(&self) -> HashSet<Permission> {
        match self {
            Role::Admin => {
                vec![
                    Permission::NodeJoin,
                    Permission::NodeLeave,
                    Permission::NodeRemove,
                    Permission::Read,
                    Permission::Write,
                    Permission::Delete,
                    Permission::ElectLeader,
                    Permission::VoteOnLeader,
                    Permission::Replicate,
                    Permission::Restore,
                    Permission::CreateBackup,
                    Permission::DeleteBackup,
                    Permission::ManageCertificates,
                    Permission::ManageKeys,
                ]
                .into_iter()
                .collect()
            }
            Role::Leader => {
                vec![
                    Permission::Read,
                    Permission::Write,
                    Permission::ElectLeader,
                    Permission::Replicate,
                    Permission::CreateBackup,
                ]
                .into_iter()
                .collect()
            }
            Role::Replica => {
                vec![Permission::Read, Permission::Replicate]
                    .into_iter()
                    .collect()
            }
            Role::Auditor => vec![Permission::Read].into_iter().collect(),
            Role::Guest => HashSet::new(),
        }
    }
}

/// User with role assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: String,
    pub role: Role,
    pub created_at: u64,
}

/// RBAC Manager
pub struct RBACManager {
    users: HashMap<String, User>,
}

impl RBACManager {
    /// Create RBAC manager
    pub fn new() -> Result<Self> {
        info!("Initializing RBAC Manager");
        Ok(Self {
            users: HashMap::new(),
        })
    }

    /// Add user with role
    pub fn add_user(&mut self, user_id: String, role: Role) -> Result<()> {
        info!("Adding user: {} with role: {:?}", user_id, role);

        let user = User {
            user_id: user_id.clone(),
            role,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.users.insert(user_id, user);
        Ok(())
    }

    /// Check permission
    pub fn has_permission(&self, user_id: &str, permission: Permission) -> Result<bool> {
        match self.users.get(user_id) {
            Some(user) => {
                let has_perm = user.role.permissions().contains(&permission);
                info!(
                    "Permission check: {} {:?}: {}",
                    user_id, permission, has_perm
                );
                Ok(has_perm)
            }
            None => {
                info!("User not found: {}", user_id);
                Ok(false)
            }
        }
    }

    /// Update user role
    pub fn update_role(&mut self, user_id: &str, new_role: Role) -> Result<()> {
        if let Some(user) = self.users.get_mut(user_id) {
            info!("Updating role for user {}: {:?}", user_id, new_role);
            user.role = new_role;
            Ok(())
        } else {
            Err(crate::ClusterError::Network(format!(
                "User not found: {}",
                user_id
            )))
        }
    }

    /// Get user
    pub fn get_user(&self, user_id: &str) -> Option<User> {
        self.users.get(user_id).cloned()
    }

    /// List all users
    pub fn list_users(&self) -> Vec<User> {
        self.users.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_permissions() {
        let admin_perms = Role::Admin.permissions();
        assert!(admin_perms.contains(&Permission::NodeJoin));
        assert!(admin_perms.contains(&Permission::ManageCertificates));

        let auditor_perms = Role::Auditor.permissions();
        assert!(auditor_perms.contains(&Permission::Read));
        assert!(!auditor_perms.contains(&Permission::Write));
    }

    #[test]
    fn test_rbac_manager() {
        let mut mgr = RBACManager::new().unwrap();
        mgr.add_user("user1".to_string(), Role::Admin).unwrap();

        assert!(mgr.has_permission("user1", Permission::Read).unwrap());
        assert!(mgr.has_permission("user1", Permission::Write).unwrap());
    }

    #[test]
    fn test_permission_denial() {
        let mut mgr = RBACManager::new().unwrap();
        mgr.add_user("auditor1".to_string(), Role::Auditor)
            .unwrap();

        assert!(mgr.has_permission("auditor1", Permission::Read).unwrap());
        assert!(!mgr.has_permission("auditor1", Permission::Write).unwrap());
    }
}
