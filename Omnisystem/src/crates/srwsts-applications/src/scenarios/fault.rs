//! Fault injection scenarios

use super::{Scenario, ScenarioResult};
use crate::errors::ApplicationStressResult;
use std::time::Instant;

/// Fault scenarios to test recovery
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultType {
    ApplicationCrash,
    NetworkLoss,
    StorageCorruption,
    GpuReset,
    MemoryExhaustion,
    DiskFull,
    PermissionDenied,
    ConcurrencyBug,
}

impl std::fmt::Display for FaultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FaultType::ApplicationCrash => write!(f, "Application Crash"),
            FaultType::NetworkLoss => write!(f, "Network Loss"),
            FaultType::StorageCorruption => write!(f, "Storage Corruption"),
            FaultType::GpuReset => write!(f, "GPU Reset"),
            FaultType::MemoryExhaustion => write!(f, "Memory Exhaustion"),
            FaultType::DiskFull => write!(f, "Disk Full"),
            FaultType::PermissionDenied => write!(f, "Permission Denied"),
            FaultType::ConcurrencyBug => write!(f, "Concurrency Bug"),
        }
    }
}

/// Fault scenario definition
pub struct FaultScenario {
    pub id: String,
    pub name: String,
    pub fault_type: FaultType,
    pub affected_app: String,
    pub duration_secs: u64,
    pub recovery_expected_secs: u64,
}

impl FaultScenario {
    /// Create a new fault scenario
    pub fn new(
        id: impl Into<String>,
        fault_type: FaultType,
        affected_app: impl Into<String>,
        duration_secs: u64,
        recovery_expected_secs: u64,
    ) -> Self {
        Self {
            id: id.into(),
            name: fault_type.to_string(),
            fault_type,
            affected_app: affected_app.into(),
            duration_secs,
            recovery_expected_secs,
        }
    }

    /// Create application crash scenario
    pub fn app_crash(app: impl Into<String>) -> Self {
        Self::new("fault-app-crash", FaultType::ApplicationCrash, app, 1, 2)
    }

    /// Create network loss scenario
    pub fn network_loss(app: impl Into<String>) -> Self {
        Self::new("fault-network-loss", FaultType::NetworkLoss, app, 5, 5)
    }

    /// Create storage corruption scenario
    pub fn storage_corruption(app: impl Into<String>) -> Self {
        Self::new("fault-storage-corruption", FaultType::StorageCorruption, app, 2, 10)
    }

    /// Create GPU reset scenario
    pub fn gpu_reset() -> Self {
        Self::new(
            "fault-gpu-reset",
            FaultType::GpuReset,
            "omnibot",
            1,
            3,
        )
    }

    /// Create memory exhaustion scenario
    pub fn memory_exhaustion(app: impl Into<String>) -> Self {
        Self::new("fault-memory-exhaustion", FaultType::MemoryExhaustion, app, 10, 5)
    }
}

impl Scenario for FaultScenario {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Fault scenario executor
pub struct FaultScenarioExecutor;

impl FaultScenarioExecutor {
    /// Execute a fault scenario
    pub async fn execute(scenario: &FaultScenario) -> ApplicationStressResult<ScenarioResult> {
        let start = Instant::now();

        tracing::info!(
            "Injecting fault {} on {}",
            scenario.fault_type,
            scenario.affected_app
        );

        // Simulate fault injection
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Simulate application under fault
        let fault_duration = std::time::Duration::from_secs(scenario.duration_secs);
        tokio::time::sleep(fault_duration).await;

        // Simulate recovery process
        let recovery_start = Instant::now();

        match scenario.fault_type {
            FaultType::ApplicationCrash => {
                // Simulate restart and state restoration
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            FaultType::NetworkLoss => {
                // Simulate reconnection and sync
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            }
            FaultType::StorageCorruption => {
                // Simulate recovery from backup or repair
                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
            }
            FaultType::GpuReset => {
                // Simulate GPU reinitialization
                tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
            }
            FaultType::MemoryExhaustion => {
                // Simulate garbage collection and cache clearing
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            FaultType::DiskFull => {
                // Simulate cleanup and space recovery
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            }
            FaultType::PermissionDenied => {
                // Simulate permission re-negotiation
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            }
            FaultType::ConcurrencyBug => {
                // Simulate deadlock detection and resolution
                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
            }
        }

        let recovery_time_ms = recovery_start.elapsed().as_millis() as u64;
        let total_duration_ms = start.elapsed().as_millis() as u64;

        // Verify recovery success (simplified check)
        let success = recovery_time_ms <= (scenario.recovery_expected_secs * 1000);

        tracing::info!(
            "Fault scenario {} completed: {} (recovery: {}ms)",
            scenario.id,
            if success { "SUCCESS" } else { "DEGRADED" },
            recovery_time_ms
        );

        Ok(ScenarioResult {
            scenario_id: scenario.id.clone(),
            name: scenario.fault_type.to_string(),
            success,
            duration_ms: total_duration_ms,
            error: None,
            recovery_time_ms: Some(recovery_time_ms),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fault_type_display() {
        assert_eq!(FaultType::ApplicationCrash.to_string(), "Application Crash");
        assert_eq!(FaultType::NetworkLoss.to_string(), "Network Loss");
    }

    #[test]
    fn test_fault_scenario_creation() {
        let scenario = FaultScenario::new(
            "fault-01",
            FaultType::ApplicationCrash,
            "workspace",
            1,
            2,
        );

        assert_eq!(scenario.id, "fault-01");
        assert_eq!(scenario.fault_type, FaultType::ApplicationCrash);
    }

    #[test]
    fn test_fault_scenario_builders() {
        let crash = FaultScenario::app_crash("workspace");
        assert_eq!(crash.fault_type, FaultType::ApplicationCrash);

        let network = FaultScenario::network_loss("buddy");
        assert_eq!(network.fault_type, FaultType::NetworkLoss);

        let storage = FaultScenario::storage_corruption("workspace");
        assert_eq!(storage.fault_type, FaultType::StorageCorruption);
    }

    #[tokio::test]
    async fn test_fault_executor() {
        let scenario = FaultScenario::app_crash("workspace");
        let result = FaultScenarioExecutor::execute(&scenario).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.scenario_id, "fault-app-crash");
    }
}
