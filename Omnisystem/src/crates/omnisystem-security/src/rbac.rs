use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    Read,
    Write,
    Delete,
    Admin,
}

#[derive(Debug, Clone)]
pub struct Role {
    pub name: String,
    pub permissions: Vec<Permission>,
}

pub struct RoleBasedAccessControl {
    roles: Arc<DashMap<String, Role>>,
}

impl RoleBasedAccessControl {
    pub fn new() -> Self {
        Self {
            roles: Arc::new(DashMap::new()),
        }
    }

    pub fn add_role(&self, role: Role) {
        self.roles.insert(role.name.clone(), role);
    }

    pub fn has_permission(&self, role: &str, permission: &Permission) -> bool {
        self.roles.get(role)
            .map(|r| r.permissions.contains(permission))
            .unwrap_or(false)
    }

    pub fn role_count(&self) -> usize {
        self.roles.len()
    }
}

impl Default for RoleBasedAccessControl {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_rbac() {
        let rbac = RoleBasedAccessControl::new();
        let role = Role {
            name: "admin".to_string(),
            permissions: vec![Permission::Admin],
        };
        rbac.add_role(role);
        assert!(rbac.has_permission("admin", &Permission::Admin));
    }
}
