//! Capability-Based Database Security
//!
//! Actors receive scoped DbCapability tokens that limit what they can do.
//! No ambient authority — an actor cannot touch the database unless explicitly granted.


/// A capability token that grants access to a specific table and operations
#[derive(Debug, Clone)]
pub struct DbCapability {
    pub table: String,
    pub operations: CapabilitySet,
}

#[derive(Debug, Clone)]
pub struct CapabilitySet {
    pub read: bool,
    pub write: bool,
    pub delete: bool,
}

impl CapabilitySet {
    pub fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            delete: false,
        }
    }

    pub fn read_write() -> Self {
        Self {
            read: true,
            write: true,
            delete: false,
        }
    }

    pub fn full() -> Self {
        Self {
            read: true,
            write: true,
            delete: true,
        }
    }
}

/// A capability token that can be passed to actors
#[derive(Debug, Clone)]
pub struct CapabilityToken {
    pub id: uuid::Uuid,
    pub capability: DbCapability,
    pub issued_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl CapabilityToken {
    pub fn new(table: String, operations: CapabilitySet) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            capability: DbCapability { table, operations },
            issued_at: chrono::Utc::now(),
            expires_at: None,
        }
    }

    pub fn with_expiration(mut self, expires_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn is_valid(&self) -> bool {
        if let Some(expires) = self.expires_at {
            chrono::Utc::now() < expires
        } else {
            true
        }
    }

    pub fn can_read(&self) -> bool {
        self.is_valid() && self.capability.operations.read
    }

    pub fn can_write(&self) -> bool {
        self.is_valid() && self.capability.operations.write
    }

    pub fn can_delete(&self) -> bool {
        self.is_valid() && self.capability.operations.delete
    }

    pub fn grants_access_to(&self, table: &str) -> bool {
        self.capability.table == table
    }
}

/// Policy-based access control
#[derive(Debug, Clone)]
pub struct AccessPolicy {
    pub actor_id: String,
    pub table: String,
    pub operations: CapabilitySet,
    pub row_level_filters: Vec<RowLevelPolicy>,
}

/// Row-level access policy
#[derive(Debug, Clone)]
pub struct RowLevelPolicy {
    pub name: String,
    pub predicate: String, // SQL predicate or Aether expression
}

impl RowLevelPolicy {
    pub fn new(name: String, predicate: String) -> Self {
        Self { name, predicate }
    }
}

/// Policy engine for enforcing access control
pub struct PolicyEngine {
    pub policies: Vec<AccessPolicy>,
    pub row_policies: Vec<RowLevelPolicy>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
            row_policies: Vec::new(),
        }
    }

    /// Check if an actor can perform an operation on a table
    pub fn check_access(&self, actor_id: &str, table: &str, operation: &str) -> bool {
        self.policies
            .iter()
            .filter(|p| p.actor_id == actor_id && p.table == table)
            .any(|p| match operation {
                "read" => p.operations.read,
                "write" => p.operations.write,
                "delete" => p.operations.delete,
                _ => false,
            })
    }

    /// Get row-level filters that should be applied to queries
    pub fn get_row_filters(&self, table: &str) -> Vec<String> {
        self.row_policies
            .iter()
            .filter(|p| p.name.starts_with(&format!("{}_", table)))
            .map(|p| p.predicate.clone())
            .collect()
    }

    /// Add a policy
    pub fn add_policy(&mut self, policy: AccessPolicy) {
        self.policies.push(policy);
    }

    /// Add a row-level policy
    pub fn add_row_policy(&mut self, policy: RowLevelPolicy) {
        self.row_policies.push(policy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_token() {
        let token = CapabilityToken::new(
            "User".to_string(),
            CapabilitySet::read_write(),
        );

        assert!(token.is_valid());
        assert!(token.can_read());
        assert!(token.can_write());
        assert!(!token.can_delete());
        assert!(token.grants_access_to("User"));
        assert!(!token.grants_access_to("Post"));
    }

    #[test]
    fn test_policy_engine() {
        let mut engine = PolicyEngine::new();

        let policy = AccessPolicy {
            actor_id: "actor1".to_string(),
            table: "User".to_string(),
            operations: CapabilitySet::read_only(),
            row_level_filters: Vec::new(),
        };

        engine.add_policy(policy);

        assert!(engine.check_access("actor1", "User", "read"));
        assert!(!engine.check_access("actor1", "User", "write"));
        assert!(!engine.check_access("actor2", "User", "read"));
    }
}
