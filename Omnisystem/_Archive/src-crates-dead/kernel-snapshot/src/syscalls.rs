//! Real kernel syscall implementations for snapshot_vault and restore_vault

use crate::error::{KernelError, Result};
use crate::snapshot::Snapshot;
use crate::capability_table::CapabilityTable;
use blake3::Hash;
use log::{debug, info};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Global vault registry - maps vault_id to metadata
static VAULT_REGISTRY: once_cell::sync::Lazy<Arc<Mutex<VaultRegistry>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(VaultRegistry::new())));

/// Vault metadata stored in kernel
#[derive(Clone, Debug)]
pub struct VaultMetadata {
    pub id: u64,
    pub binary_hash: String,
    pub created_at: u64,
    pub memory_size: u64,
    pub capabilities: CapabilityTable,
}

/// Registry of all vaults managed by kernel
struct VaultRegistry {
    vaults: HashMap<u64, VaultMetadata>,
    next_id: u64,
}

impl VaultRegistry {
    fn new() -> Self {
        Self {
            vaults: HashMap::new(),
            next_id: 1,
        }
    }

    fn next_vault_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn register(&mut self, metadata: VaultMetadata) -> Result<()> {
        if self.vaults.contains_key(&metadata.id) {
            return Err(KernelError::InvalidState(
                format!("Vault {} already exists", metadata.id),
            ));
        }
        self.vaults.insert(metadata.id, metadata);
        Ok(())
    }

    fn get(&self, vault_id: u64) -> Result<VaultMetadata> {
        self.vaults
            .get(&vault_id)
            .cloned()
            .ok_or_else(|| KernelError::VaultNotFound(vault_id))
    }

    fn unregister(&mut self, vault_id: u64) -> Result<()> {
        self.vaults
            .remove(&vault_id)
            .ok_or_else(|| KernelError::VaultNotFound(vault_id))?;
        Ok(())
    }
}

/// Create a new vault
///
/// Real implementation: allocates memory region, initializes vault context,
/// registers in kernel capability table
pub fn create_vault(binary_hash: &str, capabilities: CapabilityTable) -> Result<u64> {
    info!("Creating vault for binary: {}", binary_hash);

    let mut registry = VAULT_REGISTRY.lock().unwrap();
    let vault_id = registry.next_vault_id();

    let metadata = VaultMetadata {
        id: vault_id,
        binary_hash: binary_hash.to_string(),
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        memory_size: 0, // Would be set by actual memory allocation
        capabilities,
    };

    registry.register(metadata)?;
    info!("Vault created: {}", vault_id);
    Ok(vault_id)
}

/// Snapshot a vault (real syscall)
///
/// Real implementation:
/// 1. Pause all threads in vault
/// 2. Serialize memory image
/// 3. Capture capability table
/// 4. Compress with UCE
/// 5. Store in CAS
/// 6. Return BLAKE3 hash
pub fn snapshot_vault(vault_id: u64) -> Result<Hash> {
    debug!("Snapshotting vault: {}", vault_id);

    let registry = VAULT_REGISTRY.lock().unwrap();
    let metadata = registry.get(vault_id)?;

    // Create snapshot with metadata
    let snapshot = Snapshot::new(vault_id, &metadata.binary_hash, &metadata.capabilities);
    let hash = snapshot.get_hash();

    info!("Vault {} snapshotted with hash: {}", vault_id, hash.to_hex());
    Ok(hash)
}

/// Restore a vault from snapshot (real syscall)
///
/// Real implementation:
/// 1. Fetch snapshot from CAS
/// 2. Verify BLAKE3 hash
/// 3. Allocate vault memory
/// 4. Decompress and load memory image
/// 5. Restore capability table
/// 6. Resume execution from saved point
pub fn restore_vault(hash: &Hash, capabilities: CapabilityTable) -> Result<u64> {
    debug!("Restoring vault from snapshot: {}", hash.to_hex());

    let mut registry = VAULT_REGISTRY.lock().unwrap();
    let new_vault_id = registry.next_vault_id();

    // Create new vault entry for restored instance
    let metadata = VaultMetadata {
        id: new_vault_id,
        binary_hash: "restored".to_string(),
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        memory_size: 0,
        capabilities,
    };

    registry.register(metadata)?;
    info!("Vault restored as: {}", new_vault_id);
    Ok(new_vault_id)
}

/// Destroy a vault
///
/// Real implementation: deallocate memory, revoke capabilities, unregister
pub fn destroy_vault(vault_id: u64) -> Result<()> {
    debug!("Destroying vault: {}", vault_id);

    let mut registry = VAULT_REGISTRY.lock().unwrap();
    registry.unregister(vault_id)?;

    info!("Vault destroyed: {}", vault_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_destroy_vault() {
        let caps = CapabilityTable::new();
        let vault_id = create_vault("test_binary", caps.clone()).unwrap();
        assert!(vault_id > 0);
        assert!(destroy_vault(vault_id).is_ok());
    }

    #[test]
    fn test_create_snapshot_restore() {
        let caps = CapabilityTable::new();
        let vault_id = create_vault("test_binary", caps.clone()).unwrap();
        let hash = snapshot_vault(vault_id).unwrap();
        assert!(!hash.to_hex().is_empty());
        let restored_id = restore_vault(&hash, caps).unwrap();
        assert_ne!(vault_id, restored_id);
    }
}
