//! Vault management for test isolation

use crate::errors::{HarnessError, HarnessResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Vault state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VaultState {
    /// Vault is created but not running
    Created,
    /// Vault is currently running
    Running,
    /// Vault is paused (snapshot taken)
    Paused,
    /// Vault has been destroyed
    Destroyed,
}

impl std::fmt::Display for VaultState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "Created"),
            Self::Running => write!(f, "Running"),
            Self::Paused => write!(f, "Paused"),
            Self::Destroyed => write!(f, "Destroyed"),
        }
    }
}

/// Vault configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Memory limit in bytes
    pub memory_limit: u64,
    /// CPU limit (cores)
    pub cpu_limit: f64,
    /// I/O throughput limit in MB/s
    pub io_limit_mbps: u32,
    /// Network bandwidth limit in Mbps
    pub network_limit_mbps: u32,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            memory_limit: 512 * 1024 * 1024, // 512 MB
            cpu_limit: 4.0,                   // 4 cores
            io_limit_mbps: 500,
            network_limit_mbps: 1000,
        }
    }
}

/// Vault: isolated execution environment (backed by Sanctum)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vault {
    /// Vault ID
    pub id: Uuid,
    /// Vault state
    pub state: VaultState,
    /// Vault configuration
    pub config: VaultConfig,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Process ID (if running)
    pub pid: Option<u32>,
    /// Memory used in bytes
    pub memory_used: u64,
    /// CPU time used in milliseconds
    pub cpu_time_ms: u64,
}

impl Vault {
    /// Create a new vault
    pub fn new(config: VaultConfig) -> Self {
        Self {
            id: Uuid::new_v4(),
            state: VaultState::Created,
            config,
            created_at: chrono::Utc::now(),
            pid: None,
            memory_used: 0,
            cpu_time_ms: 0,
        }
    }

    /// Get vault ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Check if vault is running
    pub fn is_running(&self) -> bool {
        self.state == VaultState::Running
    }

    /// Check if vault is available (created but not destroyed)
    pub fn is_available(&self) -> bool {
        self.state != VaultState::Destroyed
    }
}

/// Vault snapshot for deterministic replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSnapshot {
    /// Snapshot ID
    pub id: Uuid,
    /// Associated vault ID
    pub vault_id: Uuid,
    /// Snapshot data (serialized state)
    pub data: Vec<u8>,
    /// Snapshot timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Vault state at snapshot time
    pub vault_state: VaultState,
}

impl VaultSnapshot {
    /// Create a new vault snapshot
    pub fn new(vault_id: Uuid, data: Vec<u8>, vault_state: VaultState) -> Self {
        Self {
            id: Uuid::new_v4(),
            vault_id,
            data,
            created_at: chrono::Utc::now(),
            vault_state,
        }
    }

    /// Get snapshot size in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Vault manager
pub struct VaultManager {
    /// Active vaults
    vaults: HashMap<Uuid, Vault>,
    /// Vault snapshots
    snapshots: HashMap<Uuid, Vec<VaultSnapshot>>,
    /// Maximum concurrent vaults
    max_vaults: usize,
}

impl VaultManager {
    /// Create a new vault manager
    pub fn new(max_vaults: usize) -> Self {
        Self {
            vaults: HashMap::new(),
            snapshots: HashMap::new(),
            max_vaults,
        }
    }

    /// Spawn a new vault
    pub async fn spawn_vault(&mut self, config: VaultConfig) -> HarnessResult<Vault> {
        if self.vaults.len() >= self.max_vaults {
            return Err(HarnessError::VaultCreationFailed(
                "maximum vault limit reached".to_string(),
            ));
        }

        let vault = Vault::new(config);
        let vault_id = vault.id;

        self.vaults.insert(vault_id, vault.clone());
        tracing::debug!("Spawned vault: {}", vault_id);

        Ok(vault)
    }

    /// Get a vault by ID
    pub fn get_vault(&self, vault_id: Uuid) -> HarnessResult<&Vault> {
        self.vaults
            .get(&vault_id)
            .ok_or_else(|| HarnessError::VaultNotFound(vault_id.to_string()))
    }

    /// Get mutable reference to a vault
    pub fn get_vault_mut(&mut self, vault_id: Uuid) -> HarnessResult<&mut Vault> {
        self.vaults
            .get_mut(&vault_id)
            .ok_or_else(|| HarnessError::VaultNotFound(vault_id.to_string()))
    }

    /// List all active vaults
    pub fn list_vaults(&self) -> Vec<Vault> {
        self.vaults
            .values()
            .filter(|v| v.is_available())
            .cloned()
            .collect()
    }

    /// Create a snapshot of a vault
    pub async fn snapshot_vault(&mut self, vault_id: Uuid) -> HarnessResult<VaultSnapshot> {
        let vault = self
            .vaults
            .get(&vault_id)
            .ok_or_else(|| HarnessError::VaultNotFound(vault_id.to_string()))?;

        // Serialize vault state (in real implementation, this would be more comprehensive)
        let data = serde_json::to_vec(vault)
            .map_err(|e| HarnessError::SnapshotFailed(e.to_string()))?;

        let snapshot = VaultSnapshot::new(vault_id, data, vault.state);

        self.snapshots
            .entry(vault_id)
            .or_insert_with(Vec::new)
            .push(snapshot.clone());

        tracing::debug!("Created snapshot for vault: {}", vault_id);

        Ok(snapshot)
    }

    /// Restore a vault from snapshot
    pub async fn restore_vault(&mut self, snapshot: &VaultSnapshot) -> HarnessResult<Vault> {
        let restored_vault: Vault = serde_json::from_slice(&snapshot.data)
            .map_err(|e| HarnessError::RestoreFailed(e.to_string()))?;

        self.vaults.insert(snapshot.vault_id, restored_vault.clone());

        tracing::debug!("Restored vault from snapshot: {}", snapshot.vault_id);

        Ok(restored_vault)
    }

    /// Get snapshots for a vault
    pub fn get_snapshots(&self, vault_id: Uuid) -> Vec<VaultSnapshot> {
        self.snapshots
            .get(&vault_id)
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    /// Destroy a vault
    pub async fn destroy_vault(&mut self, vault_id: Uuid) -> HarnessResult<()> {
        if let Some(vault) = self.vaults.get_mut(&vault_id) {
            vault.state = VaultState::Destroyed;
            tracing::debug!("Destroyed vault: {}", vault_id);
            Ok(())
        } else {
            Err(HarnessError::VaultNotFound(vault_id.to_string()))
        }
    }

    /// Shutdown all vaults
    pub async fn shutdown(&mut self) -> HarnessResult<()> {
        for vault in self.vaults.values_mut() {
            vault.state = VaultState::Destroyed;
        }
        tracing::info!(
            "Vault manager shutdown. Destroyed {} vaults",
            self.vaults.len()
        );
        Ok(())
    }

    /// Get resource usage statistics
    pub fn resource_stats(&self) -> VaultResourceStats {
        let vaults = self.list_vaults();
        let total_memory: u64 = vaults.iter().map(|v| v.memory_used).sum();
        let total_cpu_ms: u64 = vaults.iter().map(|v| v.cpu_time_ms).sum();

        VaultResourceStats {
            total_vaults: vaults.len(),
            total_memory_used: total_memory,
            total_cpu_time_ms: total_cpu_ms,
            avg_memory_per_vault: if vaults.len() > 0 {
                total_memory / vaults.len() as u64
            } else {
                0
            },
        }
    }
}

/// Vault resource statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultResourceStats {
    /// Number of active vaults
    pub total_vaults: usize,
    /// Total memory used by all vaults
    pub total_memory_used: u64,
    /// Total CPU time used by all vaults
    pub total_cpu_time_ms: u64,
    /// Average memory per vault
    pub avg_memory_per_vault: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_creation() {
        let config = VaultConfig::default();
        let vault = Vault::new(config);
        assert_eq!(vault.state, VaultState::Created);
        assert!(!vault.is_running());
        assert!(vault.is_available());
    }

    #[test]
    fn test_vault_state_display() {
        assert_eq!(VaultState::Created.to_string(), "Created");
        assert_eq!(VaultState::Running.to_string(), "Running");
        assert_eq!(VaultState::Destroyed.to_string(), "Destroyed");
    }

    #[tokio::test]
    async fn test_vault_manager_spawn() {
        let mut manager = VaultManager::new(10);
        let config = VaultConfig::default();

        let vault = manager.spawn_vault(config).await.unwrap();
        assert_eq!(manager.list_vaults().len(), 1);

        let retrieved = manager.get_vault(vault.id).unwrap();
        assert_eq!(retrieved.id, vault.id);
    }

    #[tokio::test]
    async fn test_vault_snapshot_restore() {
        let mut manager = VaultManager::new(10);
        let config = VaultConfig::default();

        let vault = manager.spawn_vault(config).await.unwrap();
        let snapshot = manager.snapshot_vault(vault.id).await.unwrap();

        assert_eq!(snapshot.vault_id, vault.id);
        assert!(snapshot.size() > 0);

        let retrieved_snapshots = manager.get_snapshots(vault.id);
        assert_eq!(retrieved_snapshots.len(), 1);
    }

    #[tokio::test]
    async fn test_vault_manager_destroy() {
        let mut manager = VaultManager::new(10);
        let config = VaultConfig::default();

        let vault = manager.spawn_vault(config).await.unwrap();
        manager.destroy_vault(vault.id).await.unwrap();

        let destroyed = manager.get_vault(vault.id).unwrap();
        assert_eq!(destroyed.state, VaultState::Destroyed);
    }

    #[tokio::test]
    async fn test_vault_manager_max_limit() {
        let mut manager = VaultManager::new(2);
        let config = VaultConfig::default();

        manager.spawn_vault(config.clone()).await.unwrap();
        manager.spawn_vault(config.clone()).await.unwrap();

        let result = manager.spawn_vault(config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_vault_resource_stats() {
        let mut manager = VaultManager::new(10);
        let config = VaultConfig::default();

        manager.spawn_vault(config).await.unwrap();
        let stats = manager.resource_stats();

        assert_eq!(stats.total_vaults, 1);
    }
}
