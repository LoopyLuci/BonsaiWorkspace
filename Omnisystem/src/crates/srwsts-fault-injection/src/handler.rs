//! Fault handlers for type-specific fault processing.

use crate::error::{FaultError, Result};
use crate::fault::{FaultType, FaultTypeKind};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, warn, error};

/// Fault handler trait for type-specific processing.
#[async_trait]
pub trait FaultHandler: Send + Sync {
    /// Get the fault type this handler processes.
    fn fault_type(&self) -> FaultTypeKind;

    /// Inject the fault.
    async fn inject(&self, fault: &FaultType) -> Result<()>;

    /// Recover from the fault.
    async fn recover(&self, fault: &FaultType) -> Result<()>;

    /// Check the health status after fault injection.
    async fn health_check(&self) -> Result<bool>;

    /// Get handler statistics.
    async fn statistics(&self) -> HandlerStatistics;
}

/// Memory fault handler.
pub struct MemoryFaultHandler {
    injected_faults: Arc<DashMap<String, FaultType>>,
}

impl MemoryFaultHandler {
    pub fn new() -> Self {
        Self {
            injected_faults: Arc::new(DashMap::new()),
        }
    }
}

impl Default for MemoryFaultHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FaultHandler for MemoryFaultHandler {
    fn fault_type(&self) -> FaultTypeKind {
        FaultTypeKind::Memory
    }

    async fn inject(&self, fault: &FaultType) -> Result<()> {
        match fault {
            FaultType::MemoryPressure { pressure_percent, page_faults } => {
                debug!("injecting memory pressure: {}% ({} page faults)", pressure_percent, page_faults);
                // Simulate: In real implementation, would trigger memory pressure via Vault
                self.injected_faults.insert("memory_pressure".to_string(), fault.clone());
                Ok(())
            }
            FaultType::OutOfMemory { target_pid } => {
                debug!("injecting OOM condition for PID: {}", target_pid);
                self.injected_faults.insert("oom".to_string(), fault.clone());
                Ok(())
            }
            FaultType::AllocationFailure { failure_rate, min_size } => {
                debug!("injecting allocation failures at {}% (min size: {})", failure_rate, min_size);
                self.injected_faults.insert("allocation_failure".to_string(), fault.clone());
                Ok(())
            }
            _ => Err(FaultError::InvalidConfiguration("not a memory fault".to_string())),
        }
    }

    async fn recover(&self, fault: &FaultType) -> Result<()> {
        match fault {
            FaultType::MemoryPressure { .. } => {
                debug!("recovering from memory pressure");
                self.injected_faults.remove("memory_pressure");
                Ok(())
            }
            FaultType::OutOfMemory { .. } => {
                debug!("recovering from OOM");
                self.injected_faults.remove("oom");
                Ok(())
            }
            FaultType::AllocationFailure { .. } => {
                debug!("recovering from allocation failures");
                self.injected_faults.remove("allocation_failure");
                Ok(())
            }
            _ => Err(FaultError::InvalidConfiguration("not a memory fault".to_string())),
        }
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.injected_faults.is_empty())
    }

    async fn statistics(&self) -> HandlerStatistics {
        HandlerStatistics {
            handler_type: "MemoryFaultHandler".to_string(),
            active_faults: self.injected_faults.len(),
            total_injections: 0,
            total_recoveries: 0,
        }
    }
}

/// Network fault handler.
pub struct NetworkFaultHandler {
    injected_faults: Arc<DashMap<String, FaultType>>,
}

impl NetworkFaultHandler {
    pub fn new() -> Self {
        Self {
            injected_faults: Arc::new(DashMap::new()),
        }
    }
}

impl Default for NetworkFaultHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FaultHandler for NetworkFaultHandler {
    fn fault_type(&self) -> FaultTypeKind {
        FaultTypeKind::Network
    }

    async fn inject(&self, fault: &FaultType) -> Result<()> {
        match fault {
            FaultType::NetworkPartition { isolated_subnets, affect_bidirectional } => {
                debug!("injecting network partition for subnets: {:?} (bidirectional: {})",
                    isolated_subnets, affect_bidirectional);
                self.injected_faults.insert("network_partition".to_string(), fault.clone());
                Ok(())
            }
            FaultType::NetworkLatency { latency_ms, jitter_ms, ports } => {
                debug!("injecting network latency: {}ms ± {}ms on ports: {:?}",
                    latency_ms, jitter_ms, ports);
                self.injected_faults.insert("network_latency".to_string(), fault.clone());
                Ok(())
            }
            FaultType::PacketLoss { loss_percent, correlation_percent } => {
                debug!("injecting packet loss: {}% (correlation: {}%)", loss_percent, correlation_percent);
                self.injected_faults.insert("packet_loss".to_string(), fault.clone());
                Ok(())
            }
            _ => Err(FaultError::InvalidConfiguration("not a network fault".to_string())),
        }
    }

    async fn recover(&self, fault: &FaultType) -> Result<()> {
        match fault {
            FaultType::NetworkPartition { .. } => {
                debug!("recovering from network partition");
                self.injected_faults.remove("network_partition");
                Ok(())
            }
            FaultType::NetworkLatency { .. } => {
                debug!("recovering from network latency");
                self.injected_faults.remove("network_latency");
                Ok(())
            }
            FaultType::PacketLoss { .. } => {
                debug!("recovering from packet loss");
                self.injected_faults.remove("packet_loss");
                Ok(())
            }
            _ => Err(FaultError::InvalidConfiguration("not a network fault".to_string())),
        }
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.injected_faults.is_empty())
    }

    async fn statistics(&self) -> HandlerStatistics {
        HandlerStatistics {
            handler_type: "NetworkFaultHandler".to_string(),
            active_faults: self.injected_faults.len(),
            total_injections: 0,
            total_recoveries: 0,
        }
    }
}

/// Storage fault handler.
pub struct StorageFaultHandler {
    injected_faults: Arc<DashMap<String, FaultType>>,
}

impl StorageFaultHandler {
    pub fn new() -> Self {
        Self {
            injected_faults: Arc::new(DashMap::new()),
        }
    }
}

impl Default for StorageFaultHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FaultHandler for StorageFaultHandler {
    fn fault_type(&self) -> FaultTypeKind {
        FaultTypeKind::Storage
    }

    async fn inject(&self, fault: &FaultType) -> Result<()> {
        match fault {
            FaultType::DiskFull { mount_point, fill_percent } => {
                debug!("injecting disk full on {}: {}%", mount_point, fill_percent);
                self.injected_faults.insert("disk_full".to_string(), fault.clone());
                Ok(())
            }
            FaultType::IoError { failure_rate, filesystem, error_codes } => {
                debug!("injecting I/O errors at {}%: {:?} on {:?}", failure_rate, error_codes, filesystem);
                self.injected_faults.insert("io_error".to_string(), fault.clone());
                Ok(())
            }
            FaultType::DataCorruption { corruption_rate, paths } => {
                debug!("injecting data corruption at {}% in {:?}", corruption_rate, paths);
                self.injected_faults.insert("data_corruption".to_string(), fault.clone());
                Ok(())
            }
            _ => Err(FaultError::InvalidConfiguration("not a storage fault".to_string())),
        }
    }

    async fn recover(&self, fault: &FaultType) -> Result<()> {
        match fault {
            FaultType::DiskFull { .. } => {
                debug!("recovering from disk full");
                self.injected_faults.remove("disk_full");
                Ok(())
            }
            FaultType::IoError { .. } => {
                debug!("recovering from I/O errors");
                self.injected_faults.remove("io_error");
                Ok(())
            }
            FaultType::DataCorruption { .. } => {
                debug!("recovering from data corruption");
                self.injected_faults.remove("data_corruption");
                Ok(())
            }
            _ => Err(FaultError::InvalidConfiguration("not a storage fault".to_string())),
        }
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.injected_faults.is_empty())
    }

    async fn statistics(&self) -> HandlerStatistics {
        HandlerStatistics {
            handler_type: "StorageFaultHandler".to_string(),
            active_faults: self.injected_faults.len(),
            total_injections: 0,
            total_recoveries: 0,
        }
    }
}

/// CPU fault handler.
pub struct CpuFaultHandler {
    injected_faults: Arc<DashMap<String, FaultType>>,
}

impl CpuFaultHandler {
    pub fn new() -> Self {
        Self {
            injected_faults: Arc::new(DashMap::new()),
        }
    }
}

impl Default for CpuFaultHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FaultHandler for CpuFaultHandler {
    fn fault_type(&self) -> FaultTypeKind {
        FaultTypeKind::Cpu
    }

    async fn inject(&self, fault: &FaultType) -> Result<()> {
        match fault {
            FaultType::CpuOverload { cpu_percent, core_count } => {
                debug!("injecting CPU overload: {}% on {} cores", cpu_percent, core_count);
                self.injected_faults.insert("cpu_overload".to_string(), fault.clone());
                Ok(())
            }
            FaultType::CacheContention { working_set_size, intensity } => {
                debug!("injecting cache contention: {}B working set, {}% intensity",
                    working_set_size, intensity);
                self.injected_faults.insert("cache_contention".to_string(), fault.clone());
                Ok(())
            }
            FaultType::ThermalThrottling { temperature, throttle_percent } => {
                debug!("injecting thermal throttling at {}°C: {}%", temperature, throttle_percent);
                self.injected_faults.insert("thermal_throttling".to_string(), fault.clone());
                Ok(())
            }
            _ => Err(FaultError::InvalidConfiguration("not a CPU fault".to_string())),
        }
    }

    async fn recover(&self, fault: &FaultType) -> Result<()> {
        match fault {
            FaultType::CpuOverload { .. } => {
                debug!("recovering from CPU overload");
                self.injected_faults.remove("cpu_overload");
                Ok(())
            }
            FaultType::CacheContention { .. } => {
                debug!("recovering from cache contention");
                self.injected_faults.remove("cache_contention");
                Ok(())
            }
            FaultType::ThermalThrottling { .. } => {
                debug!("recovering from thermal throttling");
                self.injected_faults.remove("thermal_throttling");
                Ok(())
            }
            _ => Err(FaultError::InvalidConfiguration("not a CPU fault".to_string())),
        }
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.injected_faults.is_empty())
    }

    async fn statistics(&self) -> HandlerStatistics {
        HandlerStatistics {
            handler_type: "CpuFaultHandler".to_string(),
            active_faults: self.injected_faults.len(),
            total_injections: 0,
            total_recoveries: 0,
        }
    }
}

/// Time/Clock fault handler.
pub struct TimeFaultHandler {
    injected_faults: Arc<DashMap<String, FaultType>>,
}

impl TimeFaultHandler {
    pub fn new() -> Self {
        Self {
            injected_faults: Arc::new(DashMap::new()),
        }
    }
}

impl Default for TimeFaultHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FaultHandler for TimeFaultHandler {
    fn fault_type(&self) -> FaultTypeKind {
        FaultTypeKind::Time
    }

    async fn inject(&self, fault: &FaultType) -> Result<()> {
        match fault {
            FaultType::ClockSkew { offset_secs, drift_rate_nanos_per_sec } => {
                debug!("injecting clock skew: {}s offset, {}ns/s drift", offset_secs, drift_rate_nanos_per_sec);
                self.injected_faults.insert("clock_skew".to_string(), fault.clone());
                Ok(())
            }
            FaultType::TimeJump { offset_secs } => {
                debug!("injecting time jump: {}s", offset_secs);
                self.injected_faults.insert("time_jump".to_string(), fault.clone());
                Ok(())
            }
            _ => Err(FaultError::InvalidConfiguration("not a time fault".to_string())),
        }
    }

    async fn recover(&self, fault: &FaultType) -> Result<()> {
        match fault {
            FaultType::ClockSkew { .. } => {
                debug!("recovering from clock skew");
                self.injected_faults.remove("clock_skew");
                Ok(())
            }
            FaultType::TimeJump { .. } => {
                debug!("recovering from time jump");
                self.injected_faults.remove("time_jump");
                Ok(())
            }
            _ => Err(FaultError::InvalidConfiguration("not a time fault".to_string())),
        }
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.injected_faults.is_empty())
    }

    async fn statistics(&self) -> HandlerStatistics {
        HandlerStatistics {
            handler_type: "TimeFaultHandler".to_string(),
            active_faults: self.injected_faults.len(),
            total_injections: 0,
            total_recoveries: 0,
        }
    }
}

/// Hardware fault handler.
pub struct HardwareFaultHandler {
    injected_faults: Arc<DashMap<String, FaultType>>,
}

impl HardwareFaultHandler {
    pub fn new() -> Self {
        Self {
            injected_faults: Arc::new(DashMap::new()),
        }
    }
}

impl Default for HardwareFaultHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FaultHandler for HardwareFaultHandler {
    fn fault_type(&self) -> FaultTypeKind {
        FaultTypeKind::Hardware
    }

    async fn inject(&self, fault: &FaultType) -> Result<()> {
        match fault {
            FaultType::GpuReset { device_id } => {
                debug!("injecting GPU reset on device {}", device_id);
                self.injected_faults.insert("gpu_reset".to_string(), fault.clone());
                Ok(())
            }
            FaultType::PowerFailure { recovery_secs } => {
                debug!("injecting power failure (recovery in {}s)", recovery_secs);
                self.injected_faults.insert("power_failure".to_string(), fault.clone());
                Ok(())
            }
            FaultType::ThermalShutdown { temperature } => {
                debug!("injecting thermal shutdown at {}°C", temperature);
                self.injected_faults.insert("thermal_shutdown".to_string(), fault.clone());
                Ok(())
            }
            FaultType::FanFailure { fan_id } => {
                debug!("injecting fan failure on fan {}", fan_id);
                self.injected_faults.insert("fan_failure".to_string(), fault.clone());
                Ok(())
            }
            _ => Err(FaultError::InvalidConfiguration("not a hardware fault".to_string())),
        }
    }

    async fn recover(&self, fault: &FaultType) -> Result<()> {
        match fault {
            FaultType::GpuReset { .. } => {
                debug!("recovering from GPU reset");
                self.injected_faults.remove("gpu_reset");
                Ok(())
            }
            FaultType::PowerFailure { .. } => {
                debug!("recovering from power failure");
                self.injected_faults.remove("power_failure");
                Ok(())
            }
            FaultType::ThermalShutdown { .. } => {
                debug!("recovering from thermal shutdown");
                self.injected_faults.remove("thermal_shutdown");
                Ok(())
            }
            FaultType::FanFailure { .. } => {
                debug!("recovering from fan failure");
                self.injected_faults.remove("fan_failure");
                Ok(())
            }
            _ => Err(FaultError::InvalidConfiguration("not a hardware fault".to_string())),
        }
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.injected_faults.is_empty())
    }

    async fn statistics(&self) -> HandlerStatistics {
        HandlerStatistics {
            handler_type: "HardwareFaultHandler".to_string(),
            active_faults: self.injected_faults.len(),
            total_injections: 0,
            total_recoveries: 0,
        }
    }
}

/// Fault handler registry for managing multiple handlers.
pub struct FaultHandlerRegistry {
    handlers: Arc<DashMap<FaultTypeKind, Arc<dyn FaultHandler>>>,
}

impl FaultHandlerRegistry {
    /// Create a new registry.
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(DashMap::new()),
        }
    }

    /// Create registry with default handlers.
    pub fn with_defaults() -> Self {
        let registry = Self::new();
        registry.register(FaultTypeKind::Memory, Arc::new(MemoryFaultHandler::new()));
        registry.register(FaultTypeKind::Network, Arc::new(NetworkFaultHandler::new()));
        registry.register(FaultTypeKind::Storage, Arc::new(StorageFaultHandler::new()));
        registry.register(FaultTypeKind::Cpu, Arc::new(CpuFaultHandler::new()));
        registry.register(FaultTypeKind::Time, Arc::new(TimeFaultHandler::new()));
        registry.register(FaultTypeKind::Hardware, Arc::new(HardwareFaultHandler::new()));
        registry
    }

    /// Register a handler.
    pub fn register(&self, fault_type: FaultTypeKind, handler: Arc<dyn FaultHandler>) {
        self.handlers.insert(fault_type, handler);
        debug!("registered handler for {}", fault_type);
    }

    /// Get a handler by fault type.
    pub fn get(&self, fault_type: FaultTypeKind) -> Result<Arc<dyn FaultHandler>> {
        self.handlers
            .get(&fault_type)
            .map(|r| r.clone())
            .ok_or_else(|| FaultError::HandlerNotFound(fault_type.to_string()))
    }

    /// Get handler for a specific fault.
    pub fn get_for_fault(&self, fault: &FaultType) -> Result<Arc<dyn FaultHandler>> {
        self.get(fault.category())
    }

    /// List all registered handlers.
    pub fn list(&self) -> Vec<String> {
        self.handlers
            .iter()
            .map(|entry| entry.key().to_string())
            .collect()
    }
}

impl Default for FaultHandlerRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Handler statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerStatistics {
    pub handler_type: String,
    pub active_faults: usize,
    pub total_injections: u64,
    pub total_recoveries: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_handler() {
        let handler = MemoryFaultHandler::new();
        let fault = FaultType::MemoryPressure {
            pressure_percent: 50,
            page_faults: 1000,
        };

        assert!(handler.inject(&fault).await.is_ok());
        assert!(!handler.health_check().await.unwrap());
        assert!(handler.recover(&fault).await.is_ok());
        assert!(handler.health_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_handler_registry() {
        let registry = FaultHandlerRegistry::with_defaults();
        let handler = registry.get(FaultTypeKind::Memory);
        assert!(handler.is_ok());
    }

    #[tokio::test]
    async fn test_get_handler_for_fault() {
        let registry = FaultHandlerRegistry::with_defaults();
        let fault = FaultType::NetworkLatency {
            latency_ms: 100,
            jitter_ms: 10,
            ports: vec![],
        };

        let handler = registry.get_for_fault(&fault);
        assert!(handler.is_ok());
        assert_eq!(handler.unwrap().fault_type(), FaultTypeKind::Network);
    }
}
