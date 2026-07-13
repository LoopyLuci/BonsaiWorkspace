/// Capability-Based Security System
/// Fine-grained access control without Unix-style permissions

use dashmap::DashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Capability - Bearer token for specific rights
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Capability {
    pub id: String,
    pub owner: String,
    pub right: AccessRight,
    pub target: String,
    pub expires_at: Option<u64>,
}

/// Access Rights
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessRight {
    // Process rights
    ProcessCreate,
    ProcessTerminate,
    ProcessInspect,
    ProcessCommunicate,

    // Memory rights
    MemoryAllocate,
    MemoryWrite,
    MemoryRead,
    MemoryExecute,
    MemoryProtect,

    // File rights
    FileRead,
    FileWrite,
    FileExecute,
    FileDelete,

    // Network rights
    NetworkBind,
    NetworkConnect,
    NetworkListen,
    NetworkSend,
    NetworkReceive,

    // System rights
    SystemControl,
    SystemMonitor,
    SystemConfigure,

    // Device rights
    DeviceAccess,
    DeviceControl,

    // Capability rights (meta)
    CapabilityGrant,
    CapabilityRevoke,
    CapabilityDelegate,
}

impl Capability {
    pub fn new(owner: String, right: AccessRight, target: String) -> Self {
        Capability {
            id: Uuid::new_v4().to_string(),
            owner,
            right,
            target,
            expires_at: None,
        }
    }

    pub fn with_expiry(mut self, expiry_time: u64) -> Self {
        self.expires_at = Some(expiry_time);
        self
    }

    pub fn is_valid(&self, current_time: u64) -> bool {
        match self.expires_at {
            None => true,
            Some(expiry) => current_time < expiry,
        }
    }
}

/// Capability Set - Collection of capabilities for a principal
#[derive(Debug, Clone)]
pub struct CapabilitySet {
    pub principal: String,
    capabilities: Arc<DashMap<String, Capability>>,
}

impl CapabilitySet {
    pub fn new(principal: String) -> Self {
        CapabilitySet {
            principal,
            capabilities: Arc::new(DashMap::new()),
        }
    }

    pub fn add(&self, capability: Capability) -> anyhow::Result<()> {
        if capability.owner != self.principal {
            return Err(anyhow::anyhow!("Capability owner mismatch"));
        }
        self.capabilities.insert(capability.id.clone(), capability);
        Ok(())
    }

    pub fn has_right(&self, right: AccessRight, target: &str, current_time: u64) -> bool {
        self.capabilities.iter().any(|cap| {
            cap.value().right == right
                && cap.value().target == target
                && cap.value().is_valid(current_time)
        })
    }

    pub fn revoke(&self, capability_id: &str) -> bool {
        self.capabilities.remove(capability_id).is_some()
    }

    pub fn list_capabilities(&self) -> Vec<Capability> {
        self.capabilities.iter().map(|cap| cap.clone()).collect()
    }

    pub fn grant_to(&self, grantee: String, right: AccessRight, target: String) -> anyhow::Result<Capability> {
        let capability = Capability::new(grantee, right, target);
        self.add(capability.clone())?;
        Ok(capability)
    }
}

/// Global Capability Manager
pub struct CapabilityManager {
    principal_capabilities: Arc<DashMap<String, CapabilitySet>>,
}

impl CapabilityManager {
    pub fn new() -> Self {
        CapabilityManager {
            principal_capabilities: Arc::new(DashMap::new()),
        }
    }

    pub fn create_principal(&self, principal_id: String) -> CapabilitySet {
        let cap_set = CapabilitySet::new(principal_id.clone());
        self.principal_capabilities.insert(principal_id, cap_set.clone());
        cap_set
    }

    pub fn get_capabilities(&self, principal_id: &str) -> Option<CapabilitySet> {
        self.principal_capabilities.get(principal_id).map(|cs| cs.clone())
    }

    pub fn grant_capability(
        &self,
        principal: &str,
        right: AccessRight,
        target: String,
    ) -> anyhow::Result<Capability> {
        match self.principal_capabilities.get(principal) {
            Some(cap_set) => {
                let cap = Capability::new(principal.to_string(), right, target);
                cap_set.add(cap.clone())?;
                Ok(cap)
            }
            None => Err(anyhow::anyhow!("Principal not found: {}", principal)),
        }
    }

    pub fn check_access(
        &self,
        principal: &str,
        right: AccessRight,
        target: &str,
        current_time: u64,
    ) -> bool {
        match self.principal_capabilities.get(principal) {
            Some(cap_set) => cap_set.has_right(right, target, current_time),
            None => false,
        }
    }
}

impl Default for CapabilityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_capability_creation() {
        let cap = Capability::new("process1".to_string(), AccessRight::ProcessCreate, "process2".to_string());
        assert_eq!(cap.owner, "process1");
        assert_eq!(cap.right, AccessRight::ProcessCreate);
    }

    #[test]
    fn test_capability_validity() {
        let cap = Capability::new("p1".to_string(), AccessRight::FileRead, "file.txt".to_string());
        assert!(cap.is_valid(0));
        assert!(cap.is_valid(u64::MAX));

        let expiring_cap = cap.with_expiry(1000);
        assert!(expiring_cap.is_valid(999));
        assert!(!expiring_cap.is_valid(1001));
    }

    #[test]
    fn test_capability_set() {
        let cap_set = CapabilitySet::new("process1".to_string());
        let cap = Capability::new("process1".to_string(), AccessRight::MemoryWrite, "heap".to_string());

        assert!(cap_set.add(cap).is_ok());
        assert!(cap_set.has_right(AccessRight::MemoryWrite, "heap", 0));
        assert!(!cap_set.has_right(AccessRight::MemoryExecute, "heap", 0));
    }

    #[test]
    fn test_capability_manager() {
        let manager = CapabilityManager::new();
        manager.create_principal("proc1".to_string());

        let can_access = manager.grant_capability("proc1", AccessRight::FileRead, "data.txt".to_string());
        assert!(can_access.is_ok());

        assert!(manager.check_access("proc1", AccessRight::FileRead, "data.txt", 0));
        assert!(!manager.check_access("proc1", AccessRight::FileWrite, "data.txt", 0));
    }
}
