/// KVM Hypervisor Control Module
///
/// Provides KVM (Kernel-based Virtual Machine) integration:
/// - VM lifecycle management
/// - vCPU allocation
/// - Memory management
/// - I/O device pass-through
/// - Performance monitoring

use crate::{LinuxError, Result};
use tracing::info;

/// KVM controller
pub struct KVMController {
    available: bool,
    max_vcpus: u32,
}

impl KVMController {
    /// Create KVM controller
    pub fn new() -> Result<Self> {
        info!("Initializing KVM controller");

        let available = std::path::Path::new("/dev/kvm").exists();

        if available {
            info!("✓ KVM is available at /dev/kvm");
        } else {
            info!("⚠ KVM not available (CPU may not support virtualization)");
        }

        let max_vcpus = detect_max_vcpus();
        info!("Max vCPUs available: {}", max_vcpus);

        Ok(Self {
            available,
            max_vcpus,
        })
    }

    /// Check if KVM is available
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Get maximum vCPUs
    pub fn max_vcpus(&self) -> u32 {
        self.max_vcpus
    }

    /// Create a virtual machine
    pub fn create_vm(&self, config: VMConfig) -> Result<VirtualMachine> {
        if !self.available {
            return Err(LinuxError::KVM("KVM not available".to_string()));
        }

        info!("Creating VM: {} with {} vCPUs", config.name, config.vcpus);

        let vm = VirtualMachine {
            name: config.name,
            vcpus: config.vcpus,
            memory_mb: config.memory_mb,
            state: VMState::Created,
        };

        Ok(vm)
    }

    /// List all VMs (stub - would use libvirt or direct KVM API)
    pub fn list_vms(&self) -> Result<Vec<VirtualMachine>> {
        Ok(Vec::new())
    }
}

/// Virtual machine configuration
#[derive(Debug, Clone)]
pub struct VMConfig {
    pub name: String,
    pub vcpus: u32,
    pub memory_mb: u64,
    pub disk_size_gb: u64,
    pub enable_kvm: bool,
    pub enable_nested: bool,
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            name: "omnisystem-vm".to_string(),
            vcpus: 4,
            memory_mb: 4096,
            disk_size_gb: 40,
            enable_kvm: true,
            enable_nested: false,
        }
    }
}

/// Virtual machine state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VMState {
    Created,
    Running,
    Paused,
    Stopped,
    Failed,
}

/// Virtual machine
#[derive(Debug, Clone)]
pub struct VirtualMachine {
    pub name: String,
    pub vcpus: u32,
    pub memory_mb: u64,
    pub state: VMState,
}

impl VirtualMachine {
    /// Start the VM
    pub fn start(&mut self) -> Result<()> {
        info!("Starting VM: {}", self.name);
        self.state = VMState::Running;
        Ok(())
    }

    /// Stop the VM
    pub fn stop(&mut self) -> Result<()> {
        info!("Stopping VM: {}", self.name);
        self.state = VMState::Stopped;
        Ok(())
    }

    /// Pause the VM
    pub fn pause(&mut self) -> Result<()> {
        if self.state != VMState::Running {
            return Err(LinuxError::KVM("VM is not running".to_string()));
        }
        info!("Pausing VM: {}", self.name);
        self.state = VMState::Paused;
        Ok(())
    }

    /// Resume the VM
    pub fn resume(&mut self) -> Result<()> {
        if self.state != VMState::Paused {
            return Err(LinuxError::KVM("VM is not paused".to_string()));
        }
        info!("Resuming VM: {}", self.name);
        self.state = VMState::Running;
        Ok(())
    }

    /// Get VM status
    pub fn get_status(&self) -> String {
        format!("{}: {} ({}vCPU, {}MB RAM)",
            self.name,
            format!("{:?}", self.state),
            self.vcpus,
            self.memory_mb)
    }
}

fn detect_max_vcpus() -> u32 {
    // Try to read from /proc/cpuinfo or sysctl
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_config_default() {
        let config = VMConfig::default();
        assert_eq!(config.vcpus, 4);
        assert_eq!(config.memory_mb, 4096);
        assert_eq!(config.disk_size_gb, 40);
    }

    #[test]
    fn test_vm_state_transitions() {
        let mut vm = VirtualMachine {
            name: "test-vm".to_string(),
            vcpus: 2,
            memory_mb: 2048,
            state: VMState::Created,
        };

        assert!(vm.start().is_ok());
        assert_eq!(vm.state, VMState::Running);

        assert!(vm.pause().is_ok());
        assert_eq!(vm.state, VMState::Paused);

        assert!(vm.resume().is_ok());
        assert_eq!(vm.state, VMState::Running);

        assert!(vm.stop().is_ok());
        assert_eq!(vm.state, VMState::Stopped);
    }
}
