use dashmap::DashMap;
use module_interfaces::ModuleError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleSignature {
    pub module_id: String,
    pub version: String,
    pub signature: String,
    pub signer: String,
    pub signed_at: u64,
    pub valid_until: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModulePermission {
    pub module_id: String,
    pub permission: String,
    pub granted: bool,
    pub granted_by: String,
    pub granted_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModuleSecurityLevel {
    Public,
    Internal,
    Restricted,
    Confidential,
}

pub struct ModuleSecurityManager {
    signatures: Arc<DashMap<String, ModuleSignature>>,
    permissions: Arc<DashMap<String, Vec<ModulePermission>>>,
    trusted_signers: Arc<DashMap<String, bool>>,
    audit_log: Arc<DashMap<String, AuditEntry>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: u64,
    pub action: String,
    pub module_id: String,
    pub principal: String,
    pub result: String,
}

impl ModuleSecurityManager {
    pub fn new() -> Self {
        info!("Creating ModuleSecurityManager");
        Self {
            signatures: Arc::new(DashMap::new()),
            permissions: Arc::new(DashMap::new()),
            trusted_signers: Arc::new(DashMap::new()),
            audit_log: Arc::new(DashMap::new()),
        }
    }

    pub fn add_trusted_signer(&self, signer: String) -> Result<(), ModuleError> {
        debug!("Adding trusted signer: {}", signer);
        self.trusted_signers.insert(signer, true);
        Ok(())
    }

    pub fn sign_module(&self, module_id: &str, version: &str, signer: &str) -> Result<ModuleSignature, ModuleError> {
        debug!("Signing module: {} v{}", module_id, version);

        let signature = format!("sig:{}:{}:{}:{}", module_id, version, signer, chrono::Utc::now().timestamp());
        let sig = ModuleSignature {
            module_id: module_id.to_string(),
            version: version.to_string(),
            signature: signature.clone(),
            signer: signer.to_string(),
            signed_at: chrono::Utc::now().timestamp() as u64,
            valid_until: (chrono::Utc::now().timestamp() + 31536000) as u64, // 1 year
        };

        self.signatures.insert(format!("{}:{}", module_id, version), sig.clone());
        info!("Module signed: {}", module_id);
        Ok(sig)
    }

    pub fn verify_signature(&self, module_id: &str, version: &str) -> Result<bool, ModuleError> {
        debug!("Verifying signature for: {} v{}", module_id, version);

        let key = format!("{}:{}", module_id, version);
        match self.signatures.get(&key) {
            Some(sig) => {
                let now = chrono::Utc::now().timestamp() as u64;
                if now > sig.valid_until {
                    warn!("Signature expired for: {}", module_id);
                    return Ok(false);
                }

                if !self.trusted_signers.contains_key(&sig.signer) {
                    warn!("Untrusted signer for: {}", module_id);
                    return Ok(false);
                }

                Ok(true)
            }
            None => {
                warn!("No signature found for: {} v{}", module_id, version);
                Ok(false)
            }
        }
    }

    pub fn grant_permission(&self, module_id: String, permission: String, principal: String) -> Result<(), ModuleError> {
        debug!("Granting permission to module: {}", module_id);

        let perm = ModulePermission {
            module_id: module_id.clone(),
            permission: permission.clone(),
            granted: true,
            granted_by: principal,
            granted_at: chrono::Utc::now().timestamp() as u64,
        };

        self.permissions
            .entry(module_id.clone())
            .or_insert_with(Vec::new)
            .push(perm);

        Ok(())
    }

    pub fn check_permission(&self, module_id: &str, permission: &str) -> Result<bool, ModuleError> {
        debug!("Checking permission for module: {} -> {}", module_id, permission);

        match self.permissions.get(module_id) {
            Some(perms) => Ok(perms.iter().any(|p| p.permission == permission && p.granted)),
            None => Ok(false),
        }
    }

    pub fn audit_action(&self, action: &str, module_id: &str, principal: &str, result: &str) -> Result<(), ModuleError> {
        let entry = AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            action: action.to_string(),
            module_id: module_id.to_string(),
            principal: principal.to_string(),
            result: result.to_string(),
        };

        self.audit_log.insert(entry.id.clone(), entry);
        Ok(())
    }

    pub fn get_audit_log(&self, limit: usize) -> Vec<AuditEntry> {
        self.audit_log
            .iter()
            .take(limit)
            .map(|entry| entry.value().clone())
            .collect()
    }
}

impl Default for ModuleSecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ModuleSecurityManager {
    fn clone(&self) -> Self {
        Self {
            signatures: Arc::clone(&self.signatures),
            permissions: Arc::clone(&self.permissions),
            trusted_signers: Arc::clone(&self.trusted_signers),
            audit_log: Arc::clone(&self.audit_log),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_manager_creation() {
        let manager = ModuleSecurityManager::new();
        assert_eq!(manager.audit_log.len(), 0);
    }

    #[test]
    fn test_add_trusted_signer() {
        let manager = ModuleSecurityManager::new();
        assert!(manager.add_trusted_signer("signer1".to_string()).is_ok());
    }

    #[test]
    fn test_sign_module() {
        let manager = ModuleSecurityManager::new();
        manager.add_trusted_signer("signer1".to_string()).unwrap();
        let sig = manager.sign_module("test-module", "1.0.0", "signer1").unwrap();
        assert_eq!(sig.module_id, "test-module");
    }

    #[test]
    fn test_verify_signature() {
        let manager = ModuleSecurityManager::new();
        manager.add_trusted_signer("signer1".to_string()).unwrap();
        manager.sign_module("test-module", "1.0.0", "signer1").unwrap();
        assert!(manager.verify_signature("test-module", "1.0.0").unwrap());
    }

    #[test]
    fn test_grant_permission() {
        let manager = ModuleSecurityManager::new();
        assert!(manager
            .grant_permission("test-module".to_string(), "read".to_string(), "admin".to_string())
            .is_ok());
    }

    #[test]
    fn test_check_permission() {
        let manager = ModuleSecurityManager::new();
        manager
            .grant_permission("test-module".to_string(), "read".to_string(), "admin".to_string())
            .unwrap();
        assert!(manager.check_permission("test-module", "read").unwrap());
    }

    #[test]
    fn test_audit_action() {
        let manager = ModuleSecurityManager::new();
        assert!(manager
            .audit_action("load", "test-module", "agent1", "success")
            .is_ok());
        assert_eq!(manager.get_audit_log(10).len(), 1);
    }
}
