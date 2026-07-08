//! Fault injector actor for managing fault injection lifecycle.

use crate::error::{FaultError, Result};
use crate::fault::{FaultDefinition, FaultId, FaultType};
use crate::handler::FaultHandlerRegistry;
use crate::schedule::FaultSchedule;
use crate::vault::{VaultChannel, VaultMessage};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{info, debug, warn, error, instrument};
use uuid::Uuid;

/// Fault injector actor.
pub struct FaultInjector {
    id: String,
    schedule: Arc<FaultSchedule>,
    handler_registry: Arc<FaultHandlerRegistry>,
    vault_channel: Arc<RwLock<VaultChannel>>,
    active_faults: Arc<DashMap<FaultId, FaultDefinition>>,
    completed_faults: Arc<DashMap<FaultId, FaultDefinition>>,
    is_running: Arc<RwLock<bool>>,
}

impl FaultInjector {
    /// Create a new fault injector.
    pub fn new(id: String, schedule: FaultSchedule) -> Self {
        let vault_channel = VaultChannel::new(id.clone());

        Self {
            id: id.clone(),
            schedule: Arc::new(schedule),
            handler_registry: Arc::new(FaultHandlerRegistry::with_defaults()),
            vault_channel: Arc::new(RwLock::new(vault_channel)),
            active_faults: Arc::new(DashMap::new()),
            completed_faults: Arc::new(DashMap::new()),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the fault injector (begins monitoring for fault injection).
    #[instrument(skip(self))]
    pub async fn start(&self) -> Result<()> {
        *self.is_running.write().await = true;
        self.schedule.start().await?;
        info!("fault injector {} started", self.id);
        Ok(())
    }

    /// Stop the fault injector.
    #[instrument(skip(self))]
    pub async fn stop(&self) -> Result<()> {
        *self.is_running.write().await = false;
        self.schedule.stop().await?;

        // Recover all active faults
        let faults: Vec<_> = self
            .active_faults
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        for fault in faults {
            let _ = self.recover_fault(fault.id).await;
        }

        info!("fault injector {} stopped", self.id);
        Ok(())
    }

    /// Check if injector is running.
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Inject a fault immediately.
    #[instrument(skip(self))]
    pub async fn inject_fault(&self, fault_def: FaultDefinition) -> Result<()> {
        if !*self.is_running.read().await {
            return Err(FaultError::InjectionFailed("injector not running".to_string()));
        }

        // Validate fault
        fault_def.validate()?;

        let fault_id = fault_def.id;
        let fault_type = &fault_def.fault_type;

        debug!("injecting fault: {} ({})", fault_id, fault_type.name());

        // Get handler
        let handler = self
            .handler_registry
            .get_for_fault(fault_type)
            .map_err(|e| FaultError::InjectionFailed(e.to_string()))?;

        // Inject via handler
        handler.inject(fault_type).await?;

        // Communicate with Vault
        let mut vault = self.vault_channel.write().await;
        let vault_msg = vault
            .protocol
            .create_inject_fault(fault_id, fault_type.clone(), fault_def.duration_secs);
        vault.send(vault_msg).await?;
        drop(vault);

        // Record active fault
        self.active_faults.insert(fault_id, fault_def);

        info!("fault {} injected successfully", fault_id);
        Ok(())
    }

    /// Recover from a fault.
    #[instrument(skip(self))]
    pub async fn recover_fault(&self, fault_id: FaultId) -> Result<()> {
        let fault_def = self
            .active_faults
            .remove(&fault_id)
            .ok_or_else(|| FaultError::FaultNotFound(fault_id.to_string()))?
            .1;

        let fault_type = &fault_def.fault_type;

        debug!("recovering from fault: {} ({})", fault_id, fault_type.name());

        // Get handler
        let handler = self
            .handler_registry
            .get_for_fault(fault_type)
            .map_err(|e| FaultError::RecoveryFailed(e.to_string()))?;

        // Recover via handler
        handler.recover(fault_type).await?;

        // Communicate with Vault
        let mut vault = self.vault_channel.write().await;
        let vault_msg = vault.protocol.create_recover_fault(fault_id);
        vault.send(vault_msg).await?;
        drop(vault);

        // Record completed fault
        self.completed_faults.insert(fault_id, fault_def);

        info!("fault {} recovered successfully", fault_id);
        Ok(())
    }

    /// Get status of a fault.
    pub async fn get_fault_status(&self, fault_id: FaultId) -> Result<FaultInjectionStatus> {
        if let Some(entry) = self.active_faults.get(&fault_id) {
            Ok(FaultInjectionStatus::Active(entry.value().clone()))
        } else if let Some(entry) = self.completed_faults.get(&fault_id) {
            Ok(FaultInjectionStatus::Completed(entry.value().clone()))
        } else if let Ok(fault) = self.schedule.get_fault(fault_id) {
            Ok(FaultInjectionStatus::Scheduled(fault))
        } else {
            Err(FaultError::FaultNotFound(fault_id.to_string()))
        }
    }

    /// Get all active faults.
    pub fn get_active_faults(&self) -> Vec<FaultDefinition> {
        self.active_faults.iter().map(|r| r.value().clone()).collect()
    }

    /// Get all completed faults.
    pub fn get_completed_faults(&self) -> Vec<FaultDefinition> {
        self.completed_faults
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }

    /// Run the fault injection loop (monitors schedule and injects faults).
    pub async fn run_loop(&self, check_interval_secs: u64) -> Result<()> {
        self.start().await?;

        let mut ticker = interval(Duration::from_secs(check_interval_secs));

        loop {
            if !*self.is_running.read().await {
                break;
            }

            ticker.tick().await;
            let now = current_time_secs();

            // Get faults that should be active now
            let faults_to_inject: Vec<_> = self
                .schedule
                .list_faults()
                .iter()
                .filter(|f| {
                    f.inject_at == now && !self.active_faults.contains_key(&f.id)
                })
                .cloned()
                .collect();

            for fault in faults_to_inject {
                if let Err(e) = self.inject_fault(fault).await {
                    error!("failed to inject fault: {}", e);
                }
            }

            // Get faults that should recover now
            let faults_to_recover: Vec<_> = self
                .active_faults
                .iter()
                .filter(|entry| entry.value().recovery_time() == now)
                .map(|entry| entry.key().clone())
                .collect();

            for fault_id in faults_to_recover {
                if let Err(e) = self.recover_fault(fault_id).await {
                    error!("failed to recover fault: {}", e);
                }
            }
        }

        self.stop().await?;
        Ok(())
    }

    /// Get injector statistics.
    pub fn statistics(&self) -> InjectorStatistics {
        InjectorStatistics {
            injector_id: self.id.clone(),
            active_faults: self.active_faults.len(),
            completed_faults: self.completed_faults.len(),
            scheduled_faults: self.schedule.list_faults().len(),
        }
    }

    /// Health check: verify handlers are functional.
    pub async fn health_check(&self) -> Result<bool> {
        for handler_type in self.handler_registry.list() {
            if let Ok(handler) = self.handler_registry.get(
                crate::fault::FaultTypeKind::Memory, // Just check memory as example
            ) {
                if !handler.health_check().await? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}

/// Fault injection status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FaultInjectionStatus {
    Scheduled(FaultDefinition),
    Active(FaultDefinition),
    Completed(FaultDefinition),
}

/// Injector statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectorStatistics {
    pub injector_id: String,
    pub active_faults: usize,
    pub completed_faults: usize,
    pub scheduled_faults: usize,
}

/// Get current Unix timestamp in seconds.
fn current_time_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::FaultSchedule;

    #[tokio::test]
    async fn test_injector_creation() {
        let schedule = FaultSchedule::new(42);
        let injector = FaultInjector::new("inj1".to_string(), schedule);
        assert!(!injector.is_running().await);
    }

    #[tokio::test]
    async fn test_injector_start_stop() {
        let schedule = FaultSchedule::new(42);
        let injector = FaultInjector::new("inj1".to_string(), schedule);

        injector.start().await.unwrap();
        assert!(injector.is_running().await);

        injector.stop().await.unwrap();
        assert!(!injector.is_running().await);
    }

    #[tokio::test]
    async fn test_inject_fault() {
        let schedule = FaultSchedule::new(42);
        let injector = FaultInjector::new("inj1".to_string(), schedule);
        injector.start().await.unwrap();

        let fault = FaultDefinition::new(
            FaultType::MemoryPressure {
                pressure_percent: 50,
                page_faults: 1000,
            },
            0,
            5,
        );

        assert!(injector.inject_fault(fault).await.is_ok());
        assert_eq!(injector.get_active_faults().len(), 1);

        injector.stop().await.unwrap();
    }
}
