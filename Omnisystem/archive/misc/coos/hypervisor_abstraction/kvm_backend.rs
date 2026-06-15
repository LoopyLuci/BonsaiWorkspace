// KVM Hypervisor Backend
// Linux KVM implementation for Omnisystem hypervisor abstraction

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// KVM hypervisor instance manager
pub struct KVMHypervisor {
    vms: Arc<Mutex<HashMap<String, KVMVirtualMachine>>>,
    libvirt_uri: String,
    capabilities: KVMCapabilities,
}

/// KVM-specific virtual machine
pub struct KVMVirtualMachine {
    vm_id: String,
    name: String,
    domain_name: String,  // libvirt domain name
    state: VMState,
    config: VMConfiguration,
    snapshots: Vec<VMSnapshot>,
    stats: Option<VMStats>,
}

/// KVM capabilities
pub struct KVMCapabilities {
    kvm_version: String,
    qemu_version: String,
    libvirt_version: String,
    supports_nested_vm: bool,
    supports_ept: bool,
    supports_npt: bool,
    supports_unrestricted_guest: bool,
    supports_virtual_apic: bool,
    max_vms: usize,
    max_vcpus: usize,
}

/// VM lifecycle state
#[derive(Debug, Clone, PartialEq)]
pub enum VMState {
    Created,
    Starting,
    Running,
    Paused,
    Suspending,
    Suspended,
    Resuming,
    Stopping,
    Stopped,
    Error(String),
}

/// VM configuration (shared with abstract interface)
#[derive(Clone, Debug)]
pub struct VMConfiguration {
    pub vm_id: String,
    pub vm_name: String,
    pub cpu_cores: i32,
    pub memory_mb: i64,
    pub disk_size_gb: i64,
    pub disk_path: String,
    pub boot_image: String,
    pub kernel_path: String,
    pub initrd_path: String,
    pub boot_args: String,
    pub enable_nested_vm: bool,
    pub enable_uefi: bool,
}

/// VM snapshot information
#[derive(Clone, Debug)]
pub struct VMSnapshot {
    pub snapshot_id: String,
    pub vm_id: String,
    pub timestamp: i64,
    pub state: VMState,
    pub memory_snapshot: String,
    pub disk_snapshot: String,
    pub metadata: HashMap<String, String>,
}

/// VM runtime statistics
#[derive(Clone, Debug)]
pub struct VMStats {
    pub vm_id: String,
    pub timestamp: i64,
    pub cpu_usage_percent: f32,
    pub memory_used_mb: i64,
    pub memory_available_mb: i64,
    pub disk_io_read_mbps: f32,
    pub disk_io_write_mbps: f32,
    pub network_rx_mbps: f32,
    pub network_tx_mbps: f32,
}

impl KVMHypervisor {
    /// Initialize KVM hypervisor
    pub fn new() -> Result<Self, String> {
        // Verify KVM kernel module is loaded
        if !Self::verify_kvm_loaded() {
            return Err("KVM kernel module not loaded".to_string());
        }

        // Get KVM capabilities
        let capabilities = Self::detect_capabilities()?;

        Ok(KVMHypervisor {
            vms: Arc::new(Mutex::new(HashMap::new())),
            libvirt_uri: "qemu:///system".to_string(),
            capabilities,
        })
    }

    /// Verify KVM kernel module is loaded
    fn verify_kvm_loaded() -> bool {
        // Check /sys/module/kvm exists
        Path::new("/sys/module/kvm").exists()
    }

    /// Detect KVM capabilities from host
    fn detect_capabilities() -> Result<KVMCapabilities, String> {
        let qemu_version = Self::get_qemu_version()
            .unwrap_or_else(|_| "unknown".to_string());

        let kvm_version = Self::get_kvm_version()
            .unwrap_or_else(|_| "unknown".to_string());

        let libvirt_version = Self::get_libvirt_version()
            .unwrap_or_else(|_| "unknown".to_string());

        // Detect CPU flags for virtualization features
        let cpuinfo = fs::read_to_string("/proc/cpuinfo")
            .unwrap_or_default();

        Ok(KVMCapabilities {
            kvm_version,
            qemu_version,
            libvirt_version,
            supports_nested_vm: cpuinfo.contains("npt") || cpuinfo.contains("ept"),
            supports_ept: cpuinfo.contains("ept"),  // Intel Extended Page Tables
            supports_npt: cpuinfo.contains("npt"),  // AMD Nested Page Tables
            supports_unrestricted_guest: cpuinfo.contains("unrestricted_guest"),
            supports_virtual_apic: cpuinfo.contains("apic"),
            max_vms: 64,  // Reasonable default
            max_vcpus: Self::count_host_cpus(),
        })
    }

    /// Get QEMU version
    fn get_qemu_version() -> Result<String, String> {
        let output = Command::new("qemu-system-x86_64")
            .arg("--version")
            .output()
            .map_err(|e| format!("Failed to get QEMU version: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().next().unwrap_or("unknown").to_string())
    }

    /// Get KVM version
    fn get_kvm_version() -> Result<String, String> {
        let output = fs::read_to_string("/sys/module/kvm/version")
            .map_err(|e| format!("Failed to read KVM version: {}", e))?;
        Ok(output.trim().to_string())
    }

    /// Get libvirt version
    fn get_libvirt_version() -> Result<String, String> {
        let output = Command::new("virsh")
            .arg("--version")
            .output()
            .map_err(|e| format!("Failed to get libvirt version: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().to_string())
    }

    /// Count host CPU cores
    fn count_host_cpus() -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    }

    /// Create a new virtual machine
    pub fn create_vm(&self, config: VMConfiguration) -> Result<String, String> {
        // Generate libvirt XML domain definition
        let libvirt_xml = self.generate_libvirt_xml(&config)?;

        // Create domain via libvirt
        self.execute_virsh(&["define"], &libvirt_xml)?;

        // Store VM in memory
        let vm = KVMVirtualMachine {
            vm_id: config.vm_id.clone(),
            name: config.vm_name.clone(),
            domain_name: format!("omnisystem-{}", config.vm_id),
            state: VMState::Created,
            config,
            snapshots: Vec::new(),
            stats: None,
        };

        let mut vms = self.vms.lock().unwrap();
        vms.insert(vm.vm_id.clone(), vm);

        Ok(vm.vm_id)
    }

    /// Generate libvirt XML domain definition
    fn generate_libvirt_xml(&self, config: &VMConfiguration) -> Result<String, String> {
        let domain_name = format!("omnisystem-{}", config.vm_id);

        let xml = format!(
            r#"<domain type='kvm'>
  <name>{}</name>
  <memory unit='MiB'>{}</memory>
  <currentMemory unit='MiB'>{}</currentMemory>
  <vcpu placement='static'>{}</vcpu>
  <os>
    <type arch='x86_64'>hvm</type>
    <kernel>{}</kernel>
    <initrd>{}</initrd>
    <cmdline>{}</cmdline>
  </os>
  <devices>
    <emulator>/usr/bin/qemu-system-x86_64</emulator>
    <disk type='file' device='disk'>
      <driver name='qemu' type='qcow2' cache='writeback' io='io_uring'/>
      <source file='{}'/>
      <target dev='vda' bus='virtio'/>
    </disk>
    <interface type='bridge'>
      <mac address='52:54:00:{}:{}:01'/>
      <source bridge='virbr0'/>
      <model type='virtio'/>
    </interface>
    <console type='pty'>
      <target type='virtio' port='0'/>
    </console>
    <serial type='pty'>
      <target port='0'/>
    </serial>
    <memballoon model='virtio'/>
    <rng model='virtio'>
      <backend model='random'>/dev/urandom</backend>
    </rng>
  </devices>
  <features>
    <acpi/>
    <apic/>
    <nested_paging>on</nested_paging>
    <unrestricted_guest>on</unrestricted_guest>
  </features>
</domain>"#,
            domain_name,
            config.memory_mb,
            config.memory_mb,
            config.cpu_cores,
            config.kernel_path,
            config.initrd_path,
            config.boot_args,
            config.disk_path,
            config.vm_id,
            config.vm_id
        );

        Ok(xml)
    }

    /// Execute virsh command
    fn execute_virsh(&self, args: &[&str], stdin_data: &str) -> Result<String, String> {
        let mut cmd = Command::new("virsh");
        for arg in args {
            cmd.arg(arg);
        }
        cmd.arg("-");  // Read from stdin

        let output = cmd
            .stdin(std::process::Stdio::piped())
            .output()
            .map_err(|e| format!("Failed to execute virsh: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("virsh failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }

    /// Start a virtual machine
    pub fn start_vm(&self, vm_id: &str) -> Result<(), String> {
        let mut vms = self.vms.lock().unwrap();
        let vm = vms.get_mut(vm_id)
            .ok_or_else(|| format!("VM {} not found", vm_id))?;

        let domain_name = vm.domain_name.clone();
        vm.state = VMState::Starting;

        // Release lock before executing long-running command
        drop(vms);

        // Start domain via libvirt
        let output = Command::new("virsh")
            .args(&["start", &domain_name])
            .output()
            .map_err(|e| format!("Failed to start VM: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut vms = self.vms.lock().unwrap();
            if let Some(vm) = vms.get_mut(vm_id) {
                vm.state = VMState::Error(stderr.to_string());
            }
            return Err(format!("Failed to start VM: {}", stderr));
        }

        let mut vms = self.vms.lock().unwrap();
        if let Some(vm) = vms.get_mut(vm_id) {
            vm.state = VMState::Running;
        }

        Ok(())
    }

    /// Stop a virtual machine
    pub fn stop_vm(&self, vm_id: &str) -> Result<(), String> {
        let mut vms = self.vms.lock().unwrap();
        let vm = vms.get_mut(vm_id)
            .ok_or_else(|| format!("VM {} not found", vm_id))?;

        let domain_name = vm.domain_name.clone();
        vm.state = VMState::Stopping;

        drop(vms);

        // Gracefully shutdown
        Command::new("virsh")
            .args(&["shutdown", &domain_name])
            .output()
            .map_err(|e| format!("Failed to shutdown VM: {}", e))?;

        // Wait up to 30 seconds for graceful shutdown
        let mut attempts = 0;
        while attempts < 30 {
            std::thread::sleep(std::time::Duration::from_secs(1));

            let output = Command::new("virsh")
                .args(&["domstate", &domain_name])
                .output()
                .unwrap_or_default();

            let state = String::from_utf8_lossy(&output.stdout);
            if state.contains("shut off") {
                let mut vms = self.vms.lock().unwrap();
                if let Some(vm) = vms.get_mut(vm_id) {
                    vm.state = VMState::Stopped;
                }
                return Ok(());
            }

            attempts += 1;
        }

        // Force kill if graceful shutdown didn't work
        let _ = Command::new("virsh")
            .args(&["destroy", &domain_name])
            .output();

        let mut vms = self.vms.lock().unwrap();
        if let Some(vm) = vms.get_mut(vm_id) {
            vm.state = VMState::Stopped;
        }

        Ok(())
    }

    /// Pause a virtual machine
    pub fn pause_vm(&self, vm_id: &str) -> Result<(), String> {
        let mut vms = self.vms.lock().unwrap();
        let vm = vms.get_mut(vm_id)
            .ok_or_else(|| format!("VM {} not found", vm_id))?;

        let domain_name = vm.domain_name.clone();
        vm.state = VMState::Paused;

        drop(vms);

        Command::new("virsh")
            .args(&["suspend", &domain_name])
            .output()
            .map_err(|e| format!("Failed to pause VM: {}", e))?;

        Ok(())
    }

    /// Resume a paused virtual machine
    pub fn resume_vm(&self, vm_id: &str) -> Result<(), String> {
        let mut vms = self.vms.lock().unwrap();
        let vm = vms.get_mut(vm_id)
            .ok_or_else(|| format!("VM {} not found", vm_id))?;

        let domain_name = vm.domain_name.clone();
        vm.state = VMState::Resuming;

        drop(vms);

        Command::new("virsh")
            .args(&["resume", &domain_name])
            .output()
            .map_err(|e| format!("Failed to resume VM: {}", e))?;

        let mut vms = self.vms.lock().unwrap();
        if let Some(vm) = vms.get_mut(vm_id) {
            vm.state = VMState::Running;
        }

        Ok(())
    }

    /// Get VM current state
    pub fn get_vm_state(&self, vm_id: &str) -> Result<VMState, String> {
        let vms = self.vms.lock().unwrap();
        let vm = vms.get(vm_id)
            .ok_or_else(|| format!("VM {} not found", vm_id))?;

        Ok(vm.state.clone())
    }

    /// Get VM statistics
    pub fn get_vm_stats(&self, vm_id: &str) -> Result<VMStats, String> {
        let mut vms = self.vms.lock().unwrap();
        let vm = vms.get_mut(vm_id)
            .ok_or_else(|| format!("VM {} not found", vm_id))?;

        let domain_name = vm.domain_name.clone();
        drop(vms);

        // Get stats from virsh
        let output = Command::new("virsh")
            .args(&["domstats", &domain_name])
            .output()
            .map_err(|e| format!("Failed to get VM stats: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse stats (simplified)
        let stats = VMStats {
            vm_id: vm_id.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            cpu_usage_percent: 0.0,  // TODO: parse from output
            memory_used_mb: 0,        // TODO: parse from output
            memory_available_mb: 0,   // TODO: parse from output
            disk_io_read_mbps: 0.0,   // TODO: parse from output
            disk_io_write_mbps: 0.0,  // TODO: parse from output
            network_rx_mbps: 0.0,     // TODO: parse from output
            network_tx_mbps: 0.0,     // TODO: parse from output
        };

        let mut vms = self.vms.lock().unwrap();
        if let Some(vm) = vms.get_mut(vm_id) {
            vm.stats = Some(stats.clone());
        }

        Ok(stats)
    }

    /// Create a snapshot of the VM
    pub fn create_snapshot(&self, vm_id: &str, snapshot_id: &str) -> Result<VMSnapshot, String> {
        let mut vms = self.vms.lock().unwrap();
        let vm = vms.get_mut(vm_id)
            .ok_or_else(|| format!("VM {} not found", vm_id))?;

        let domain_name = vm.domain_name.clone();
        let state = vm.state.clone();

        let snapshot = VMSnapshot {
            snapshot_id: snapshot_id.to_string(),
            vm_id: vm_id.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            state,
            memory_snapshot: format!("/var/lib/libvirt/qemu/snapshot/{}.mem", snapshot_id),
            disk_snapshot: format!("/var/lib/libvirt/qemu/snapshot/{}.img", snapshot_id),
            metadata: HashMap::new(),
        };

        vm.snapshots.push(snapshot.clone());

        drop(vms);

        // Create snapshot via virsh
        let xml = format!(
            r#"<domainsnapshot>
  <name>{}</name>
  <description>Omnisystem snapshot</description>
  <memory snapshot='external' file='{}'/>
  <disks>
    <disk name='vda' snapshot='external' type='file' file='{}'/>
  </disks>
</domainsnapshot>"#,
            snapshot_id,
            snapshot.memory_snapshot,
            snapshot.disk_snapshot
        );

        let mut cmd = Command::new("virsh");
        cmd.args(&["snapshot-create", &domain_name, "-"]);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to create snapshot: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to create snapshot: {}", stderr));
        }

        Ok(snapshot)
    }

    /// Restore a snapshot
    pub fn restore_snapshot(&self, vm_id: &str, snapshot_id: &str) -> Result<(), String> {
        let domain_name = {
            let vms = self.vms.lock().unwrap();
            let vm = vms.get(vm_id)
                .ok_or_else(|| format!("VM {} not found", vm_id))?;
            vm.domain_name.clone()
        };

        // Revert to snapshot via virsh
        Command::new("virsh")
            .args(&["snapshot-revert", &domain_name, snapshot_id])
            .output()
            .map_err(|e| format!("Failed to restore snapshot: {}", e))?;

        Ok(())
    }

    /// List all VMs
    pub fn list_vms(&self) -> Result<Vec<String>, String> {
        let vms = self.vms.lock().unwrap();
        Ok(vms.keys().cloned().collect())
    }

    /// Delete a VM
    pub fn delete_vm(&self, vm_id: &str) -> Result<(), String> {
        let mut vms = self.vms.lock().unwrap();
        let vm = vms.remove(vm_id)
            .ok_or_else(|| format!("VM {} not found", vm_id))?;

        drop(vms);

        // Undefine domain via libvirt
        Command::new("virsh")
            .args(&["undefine", &vm.domain_name, "--remove-all-storage"])
            .output()
            .map_err(|e| format!("Failed to delete VM: {}", e))?;

        Ok(())
    }

    /// Get hypervisor capabilities
    pub fn get_capabilities(&self) -> &KVMCapabilities {
        &self.capabilities
    }
}

/// Unit tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kvm_initialization() {
        if KVMHypervisor::verify_kvm_loaded() {
            let result = KVMHypervisor::new();
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_capability_detection() {
        let result = KVMHypervisor::detect_capabilities();
        assert!(result.is_ok());
    }
}
