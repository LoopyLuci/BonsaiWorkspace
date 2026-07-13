// OS Abstraction Layer - unified interface across platforms

use crate::os_info::OperatingSystem;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// System capabilities - what this OS supports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCapabilities {
    pub cpu_affinity: bool,
    pub numa_awareness: bool,
    pub cgroups: bool,
    pub namespaces: bool,
    pub seccomp: bool,
    pub apparmor: bool,
    pub selinux: bool,
    pub hyper_v: bool,
    pub kvm: bool,
    pub gpu_support: bool,
    pub tpm: bool,
    pub trusted_execution: bool,
    pub container_support: bool,
    pub supported_archs: Vec<String>,
}

/// System resource information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    pub cpu_count: u32,
    pub cpu_model: String,
    pub total_memory: u64,
    pub available_memory: u64,
    pub total_disk: u64,
    pub available_disk: u64,
    pub uptime_seconds: u64,
}

/// Process information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub state: String,
    pub memory_usage: u64,
    pub cpu_usage: f64,
    pub threads: u32,
}

/// Network interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub ip_address: String,
    pub mac_address: String,
    pub mtu: u32,
    pub is_up: bool,
}

/// Storage device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageDevice {
    pub name: String,
    pub device_path: String,
    pub total_size: u64,
    pub free_space: u64,
    pub filesystem: String,
}

/// OS Abstraction Trait - unified interface for all platforms
#[async_trait]
pub trait OSAbstraction: Send + Sync {
    /// Get operating system
    fn os(&self) -> OperatingSystem;

    /// Get system capabilities
    async fn capabilities(&self) -> anyhow::Result<SystemCapabilities>;

    /// Get resource information
    async fn resource_info(&self) -> anyhow::Result<ResourceInfo>;

    /// List all processes
    async fn list_processes(&self) -> anyhow::Result<Vec<ProcessInfo>>;

    /// Get specific process info
    async fn get_process(&self, pid: u32) -> anyhow::Result<ProcessInfo>;

    /// Kill a process
    async fn kill_process(&self, pid: u32) -> anyhow::Result<()>;

    /// List network interfaces
    async fn list_network_interfaces(&self) -> anyhow::Result<Vec<NetworkInterface>>;

    /// List storage devices
    async fn list_storage_devices(&self) -> anyhow::Result<Vec<StorageDevice>>;

    /// Get filesystem information
    async fn get_filesystem_info(&self, path: &str) -> anyhow::Result<StorageDevice>;

    /// Mount a filesystem
    async fn mount(&self, device: &str, path: &str, fstype: &str) -> anyhow::Result<()>;

    /// Unmount a filesystem
    async fn unmount(&self, path: &str) -> anyhow::Result<()>;

    /// Get environment variables
    fn get_env(&self, key: &str) -> Option<String>;

    /// Set environment variable
    fn set_env(&mut self, key: String, value: String);

    /// Run a command
    async fn run_command(&self, cmd: &str, args: &[&str]) -> anyhow::Result<String>;

    /// Check if a capability is available
    async fn has_capability(&self, cap: &str) -> anyhow::Result<bool>;
}

/// Default implementation for testing
pub struct DefaultOSAbstraction {
    os: OperatingSystem,
    env_vars: HashMap<String, String>,
}

impl DefaultOSAbstraction {
    pub fn new(os: OperatingSystem) -> Self {
        Self {
            os,
            env_vars: HashMap::new(),
        }
    }
}

#[async_trait]
impl OSAbstraction for DefaultOSAbstraction {
    fn os(&self) -> OperatingSystem {
        self.os
    }

    async fn capabilities(&self) -> anyhow::Result<SystemCapabilities> {
        Ok(SystemCapabilities {
            cpu_affinity: true,
            numa_awareness: true,
            cgroups: matches!(self.os, OperatingSystem::Linux),
            namespaces: matches!(self.os, OperatingSystem::Linux),
            seccomp: matches!(self.os, OperatingSystem::Linux),
            apparmor: false,
            selinux: false,
            hyper_v: matches!(self.os, OperatingSystem::Windows),
            kvm: matches!(self.os, OperatingSystem::Linux),
            gpu_support: true,
            tpm: true,
            trusted_execution: true,
            container_support: true,
            supported_archs: vec!["x86_64".to_string(), "aarch64".to_string()],
        })
    }

    async fn resource_info(&self) -> anyhow::Result<ResourceInfo> {
        Ok(ResourceInfo {
            cpu_count: num_cpus::get() as u32,
            cpu_model: "Unknown".to_string(),
            total_memory: 16 * 1024 * 1024 * 1024, // 16GB
            available_memory: 8 * 1024 * 1024 * 1024, // 8GB
            total_disk: 256 * 1024 * 1024 * 1024, // 256GB
            available_disk: 128 * 1024 * 1024 * 1024, // 128GB
            uptime_seconds: 86400,
        })
    }

    async fn list_processes(&self) -> anyhow::Result<Vec<ProcessInfo>> {
        Ok(vec![])
    }

    async fn get_process(&self, pid: u32) -> anyhow::Result<ProcessInfo> {
        Ok(ProcessInfo {
            pid,
            name: "unknown".to_string(),
            state: "running".to_string(),
            memory_usage: 0,
            cpu_usage: 0.0,
            threads: 1,
        })
    }

    async fn kill_process(&self, _pid: u32) -> anyhow::Result<()> {
        Ok(())
    }

    async fn list_network_interfaces(&self) -> anyhow::Result<Vec<NetworkInterface>> {
        Ok(vec![NetworkInterface {
            name: "eth0".to_string(),
            ip_address: "192.168.1.1".to_string(),
            mac_address: "00:00:00:00:00:00".to_string(),
            mtu: 1500,
            is_up: true,
        }])
    }

    async fn list_storage_devices(&self) -> anyhow::Result<Vec<StorageDevice>> {
        Ok(vec![])
    }

    async fn get_filesystem_info(&self, path: &str) -> anyhow::Result<StorageDevice> {
        Ok(StorageDevice {
            name: path.to_string(),
            device_path: "/dev/sda1".to_string(),
            total_size: 256 * 1024 * 1024 * 1024,
            free_space: 128 * 1024 * 1024 * 1024,
            filesystem: "ext4".to_string(),
        })
    }

    async fn mount(&self, device: &str, path: &str, _fstype: &str) -> anyhow::Result<()> {
        tracing::info!("Would mount {} at {}", device, path);
        Ok(())
    }

    async fn unmount(&self, path: &str) -> anyhow::Result<()> {
        tracing::info!("Would unmount {}", path);
        Ok(())
    }

    fn get_env(&self, key: &str) -> Option<String> {
        self.env_vars.get(key).cloned()
    }

    fn set_env(&mut self, key: String, value: String) {
        self.env_vars.insert(key, value);
    }

    async fn run_command(&self, cmd: &str, _args: &[&str]) -> anyhow::Result<String> {
        Ok(format!("Would run: {}", cmd))
    }

    async fn has_capability(&self, cap: &str) -> anyhow::Result<bool> {
        let capabilities = self.capabilities().await?;
        Ok(match cap {
            "cpu_affinity" => capabilities.cpu_affinity,
            "numa" => capabilities.numa_awareness,
            "cgroups" => capabilities.cgroups,
            "gpu" => capabilities.gpu_support,
            _ => false,
        })
    }
}

// Import num_cpus for CPU count
extern crate num_cpus;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_capabilities() {
        let os = DefaultOSAbstraction::new(OperatingSystem::Linux);
        let caps = os.capabilities().await.unwrap();
        assert!(caps.container_support);
    }

    #[tokio::test]
    async fn test_env_vars() {
        let mut os = DefaultOSAbstraction::new(OperatingSystem::Linux);
        os.set_env("TEST".to_string(), "value".to_string());
        assert_eq!(os.get_env("TEST"), Some("value".to_string()));
    }
}
