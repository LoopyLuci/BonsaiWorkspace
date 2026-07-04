/// Hypervisor Abstraction Layer
/// Unified abstraction for KVM, Hyper-V, Virtualization.framework

use serde::{Deserialize, Serialize};

/// Hypervisor Backend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HypervisorBackend {
    /// KVM - Linux kernel-based virtual machine
    KVM,
    /// Hyper-V - Windows hypervisor
    HyperV,
    /// Virtualization.framework - macOS hypervisor
    VirtualizationFramework,
    /// None - Running natively without virtualization
    None,
}

impl HypervisorBackend {
    pub fn detect() -> Self {
        #[cfg(target_os = "linux")]
        if std::path::Path::new("/dev/kvm").exists() {
            return HypervisorBackend::KVM;
        }

        #[cfg(target_os = "windows")]
        if cfg!(windows) {
            return HypervisorBackend::HyperV;
        }

        #[cfg(target_os = "macos")]
        if cfg!(target_os = "macos") {
            return HypervisorBackend::VirtualizationFramework;
        }

        HypervisorBackend::None
    }

    pub fn name(&self) -> &'static str {
        match self {
            HypervisorBackend::KVM => "KVM",
            HypervisorBackend::HyperV => "Hyper-V",
            HypervisorBackend::VirtualizationFramework => "Virtualization.framework",
            HypervisorBackend::None => "None",
        }
    }

    pub fn is_available(&self) -> bool {
        *self != HypervisorBackend::None
    }
}

/// VM Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMConfig {
    pub vm_id: String,
    pub name: String,
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub disk_gb: u64,
    pub backend: HypervisorBackend,
    pub network_enabled: bool,
    pub nested_virtualization: bool,
}

impl VMConfig {
    pub fn new(name: String, cpu_cores: u32, memory_mb: u64) -> Self {
        VMConfig {
            vm_id: uuid::Uuid::new_v4().to_string(),
            name,
            cpu_cores,
            memory_mb,
            disk_gb: 20,
            backend: HypervisorBackend::detect(),
            network_enabled: true,
            nested_virtualization: false,
        }
    }

    pub fn with_disk(mut self, gb: u64) -> Self {
        self.disk_gb = gb;
        self
    }

    pub fn with_backend(mut self, backend: HypervisorBackend) -> Self {
        self.backend = backend;
        self
    }

    pub fn with_networking(mut self, enabled: bool) -> Self {
        self.network_enabled = enabled;
        self
    }
}

/// VM State
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VMState {
    Stopped,
    Running,
    Paused,
    Suspended,
    Error,
}

/// Virtual Machine Manager
pub struct VirtualMachineManager {
    backend: HypervisorBackend,
    vms: std::sync::Arc<dashmap::DashMap<String, VMState>>,
}

impl VirtualMachineManager {
    pub fn new(backend: HypervisorBackend) -> Self {
        VirtualMachineManager {
            backend,
            vms: std::sync::Arc::new(dashmap::DashMap::new()),
        }
    }

    pub fn detect_and_create() -> Self {
        let backend = HypervisorBackend::detect();
        tracing::info!("Detected hypervisor: {}", backend.name());
        VirtualMachineManager::new(backend)
    }

    pub fn backend(&self) -> HypervisorBackend {
        self.backend
    }

    pub fn create_vm(&self, config: VMConfig) -> anyhow::Result<VMInfo> {
        if !self.backend.is_available() {
            return Err(anyhow::anyhow!("Hypervisor not available"));
        }

        self.vms.insert(config.vm_id.clone(), VMState::Stopped);

        tracing::info!(
            "Created VM: {} on {} ({}x {}MB)",
            config.name,
            self.backend.name(),
            config.cpu_cores,
            config.memory_mb
        );

        Ok(VMInfo {
            vm_id: config.vm_id,
            name: config.name,
            state: VMState::Stopped,
            backend: self.backend,
            cpu_cores: config.cpu_cores,
            memory_mb: config.memory_mb,
        })
    }

    pub fn start_vm(&self, vm_id: &str) -> anyhow::Result<()> {
        match self.vms.get_mut(vm_id) {
            Some(mut state) => {
                *state = VMState::Running;
                tracing::info!("Started VM: {}", vm_id);
                Ok(())
            }
            None => Err(anyhow::anyhow!("VM not found: {}", vm_id)),
        }
    }

    pub fn stop_vm(&self, vm_id: &str) -> anyhow::Result<()> {
        match self.vms.get_mut(vm_id) {
            Some(mut state) => {
                *state = VMState::Stopped;
                tracing::info!("Stopped VM: {}", vm_id);
                Ok(())
            }
            None => Err(anyhow::anyhow!("VM not found: {}", vm_id)),
        }
    }

    pub fn pause_vm(&self, vm_id: &str) -> anyhow::Result<()> {
        match self.vms.get_mut(vm_id) {
            Some(mut state) => {
                if *state == VMState::Running {
                    *state = VMState::Paused;
                    tracing::info!("Paused VM: {}", vm_id);
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("VM not running: {}", vm_id))
                }
            }
            None => Err(anyhow::anyhow!("VM not found: {}", vm_id)),
        }
    }

    pub fn get_vm_state(&self, vm_id: &str) -> Option<VMState> {
        self.vms.get(vm_id).map(|state| *state)
    }

    pub fn list_vms(&self) -> Vec<String> {
        self.vms.iter().map(|entry| entry.key().clone()).collect()
    }

    pub fn vm_count(&self) -> usize {
        self.vms.len()
    }
}

/// VM Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMInfo {
    pub vm_id: String,
    pub name: String,
    pub state: VMState,
    pub backend: HypervisorBackend,
    pub cpu_cores: u32,
    pub memory_mb: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hypervisor_detection() {
        let backend = HypervisorBackend::detect();
        assert!(!backend.name().is_empty());
    }

    #[test]
    fn test_vm_config() {
        let config = VMConfig::new("test_vm".to_string(), 4, 8192)
            .with_disk(50)
            .with_networking(true);

        assert_eq!(config.name, "test_vm");
        assert_eq!(config.cpu_cores, 4);
        assert_eq!(config.memory_mb, 8192);
        assert_eq!(config.disk_gb, 50);
    }

    #[test]
    fn test_vm_manager() {
        let manager = VirtualMachineManager::new(HypervisorBackend::None);
        let config = VMConfig::new("vm1".to_string(), 2, 4096);

        // Creating VM on None backend should fail
        let result = manager.create_vm(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_vm_state_transitions() {
        let manager = VirtualMachineManager::new(HypervisorBackend::KVM);
        let config = VMConfig::new("vm1".to_string(), 2, 4096).with_backend(HypervisorBackend::KVM);

        // Manually create VM state for testing
        manager.vms.insert(config.vm_id.clone(), VMState::Stopped);

        // Test state transitions
        assert_eq!(manager.get_vm_state(&config.vm_id), Some(VMState::Stopped));

        manager.start_vm(&config.vm_id).unwrap();
        assert_eq!(manager.get_vm_state(&config.vm_id), Some(VMState::Running));

        manager.pause_vm(&config.vm_id).unwrap();
        assert_eq!(manager.get_vm_state(&config.vm_id), Some(VMState::Paused));

        manager.stop_vm(&config.vm_id).unwrap();
        assert_eq!(manager.get_vm_state(&config.vm_id), Some(VMState::Stopped));
    }
}
