use parking_lot::RwLock;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use crate::KernelError;

pub type CapabilityId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Capability {
    // Memory capabilities
    MemoryRead,
    MemoryWrite,
    MemoryExecute,

    // Process capabilities
    ProcessCreate,
    ProcessTerminate,
    ProcessSignal,

    // Device capabilities
    DeviceRead,
    DeviceWrite,
    DeviceControl,

    // Network capabilities
    NetworkBind,
    NetworkConnect,
    NetworkListen,

    // IPC capabilities
    IPCCreate,
    IPCRead,
    IPCWrite,

    // File system capabilities
    FileRead,
    FileWrite,
    FileDelete,
    FileExecute,

    // Administrative capabilities
    AdminCapabilities,
    AdminScheduling,
    AdminInterrupts,

    // Security capabilities
    SecurityCreateToken,
    SecurityModifyPolicy,

    // Platform specific
    Custom(u64),
}

#[derive(Clone, Debug)]
pub struct CapabilitySet {
    capabilities: BTreeSet<Capability>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        CapabilitySet {
            capabilities: BTreeSet::new(),
        }
    }

    pub fn grant(&mut self, capability: Capability) {
        self.capabilities.insert(capability);
    }

    pub fn revoke(&mut self, capability: Capability) {
        self.capabilities.remove(&capability);
    }

    pub fn has(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }

    pub fn list(&self) -> Vec<Capability> {
        self.capabilities.iter().cloned().collect()
    }

    pub fn count(&self) -> usize {
        self.capabilities.len()
    }
}

pub struct CapabilityManager {
    process_capabilities: RwLock<BTreeMap<u64, CapabilitySet>>,
}

impl CapabilityManager {
    pub fn new() -> Self {
        CapabilityManager {
            process_capabilities: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn create_process_capabilities(&self, pid: u64) -> Result<(), KernelError> {
        let mut caps = self.process_capabilities.write();

        if caps.contains_key(&pid) {
            return Err(KernelError::CapabilityError(
                "Process already has capabilities".to_string(),
            ));
        }

        caps.insert(pid, CapabilitySet::new());
        Ok(())
    }

    pub fn grant_capability(&self, pid: u64, capability: Capability) -> Result<(), KernelError> {
        let mut caps = self.process_capabilities.write();

        match caps.get_mut(&pid) {
            Some(cap_set) => {
                cap_set.grant(capability);
                Ok(())
            }
            None => Err(KernelError::CapabilityError(
                "Process not found".to_string(),
            )),
        }
    }

    pub fn revoke_capability(&self, pid: u64, capability: Capability) -> Result<(), KernelError> {
        let mut caps = self.process_capabilities.write();

        match caps.get_mut(&pid) {
            Some(cap_set) => {
                cap_set.revoke(capability);
                Ok(())
            }
            None => Err(KernelError::CapabilityError(
                "Process not found".to_string(),
            )),
        }
    }

    pub fn check_capability(&self, pid: u64, capability: Capability) -> Result<bool, KernelError> {
        let caps = self.process_capabilities.read();

        match caps.get(&pid) {
            Some(cap_set) => Ok(cap_set.has(capability)),
            None => Err(KernelError::CapabilityError(
                "Process not found".to_string(),
            )),
        }
    }

    pub fn get_capabilities(&self, pid: u64) -> Result<Vec<Capability>, KernelError> {
        let caps = self.process_capabilities.read();

        match caps.get(&pid) {
            Some(cap_set) => Ok(cap_set.list()),
            None => Err(KernelError::CapabilityError(
                "Process not found".to_string(),
            )),
        }
    }

    pub fn delete_process_capabilities(&self, pid: u64) -> Result<(), KernelError> {
        self.process_capabilities
            .write()
            .remove(&pid)
            .ok_or(KernelError::CapabilityError("Process not found".to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_set() {
        let mut cap_set = CapabilitySet::new();
        cap_set.grant(Capability::MemoryRead);
        assert!(cap_set.has(Capability::MemoryRead));
        assert!(!cap_set.has(Capability::MemoryWrite));
    }

    #[test]
    fn test_capability_manager() {
        let cm = CapabilityManager::new();
        let result = cm.create_process_capabilities(1);
        assert!(result.is_ok());

        let grant_result = cm.grant_capability(1, Capability::MemoryRead);
        assert!(grant_result.is_ok());

        let check_result = cm.check_capability(1, Capability::MemoryRead);
        assert!(check_result.is_ok());
        assert!(check_result.unwrap());
    }
}
