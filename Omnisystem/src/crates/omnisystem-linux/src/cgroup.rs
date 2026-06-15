/// cgroup Resource Management Module
///
/// Provides cgroup (control groups) integration:
/// - Process grouping and resource limits
/// - CPU allocation
/// - Memory limits
/// - I/O bandwidth throttling
/// - Freezer (pause/resume process groups)

use crate::{LinuxError, Result};
use std::path::PathBuf;
use tracing::info;

/// cgroup manager
pub struct CgroupManager {
    cgroup_root: PathBuf,
    version: CgroupVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupVersion {
    V1,
    V2,
}

impl CgroupManager {
    /// Create cgroup manager
    pub fn new() -> Result<Self> {
        info!("Initializing cgroup manager");

        // Try to detect cgroups version
        let cgroup_root = PathBuf::from("/sys/fs/cgroup");

        let version = if cgroup_root.join("cgroup.controllers").exists() {
            info!("✓ cgroups v2 detected");
            CgroupVersion::V2
        } else if cgroup_root.join("cpuset").exists() {
            info!("✓ cgroups v1 detected");
            CgroupVersion::V1
        } else {
            return Err(LinuxError::Cgroup("No cgroups support detected".to_string()));
        };

        Ok(Self {
            cgroup_root,
            version,
        })
    }

    /// Check if cgroups v2 is available
    pub fn is_v2(&self) -> bool {
        self.version == CgroupVersion::V2
    }

    /// Create a cgroup
    pub fn create_cgroup(&self, name: &str) -> Result<Cgroup> {
        info!("Creating cgroup: {}", name);

        let path = self.cgroup_root.join(name);
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }

        Ok(Cgroup {
            name: name.to_string(),
            path,
            limits: CgroupLimits::default(),
        })
    }

    /// Delete a cgroup
    pub fn delete_cgroup(&self, name: &str) -> Result<()> {
        info!("Deleting cgroup: {}", name);

        let path = self.cgroup_root.join(name);
        if path.exists() {
            std::fs::remove_dir_all(&path)?;
        }

        Ok(())
    }
}

/// cgroup resource limits
#[derive(Debug, Clone)]
pub struct CgroupLimits {
    pub cpuset_cpus: Option<String>,
    pub memory_limit: Option<u64>,
    pub memory_soft_limit: Option<u64>,
    pub cpu_shares: Option<u32>,
    pub io_throttle_read_bps: Option<u64>,
    pub io_throttle_write_bps: Option<u64>,
}

impl Default for CgroupLimits {
    fn default() -> Self {
        Self {
            cpuset_cpus: None,
            memory_limit: None,
            memory_soft_limit: None,
            cpu_shares: None,
            io_throttle_read_bps: None,
            io_throttle_write_bps: None,
        }
    }
}

/// cgroup
pub struct Cgroup {
    pub name: String,
    pub path: PathBuf,
    pub limits: CgroupLimits,
}

impl Cgroup {
    /// Set memory limit
    pub fn set_memory_limit(&mut self, bytes: u64) -> Result<()> {
        info!("Setting memory limit for {}: {} bytes", self.name, bytes);
        self.limits.memory_limit = Some(bytes);

        let limit_path = match self.version() {
            CgroupVersion::V2 => self.path.join("memory.max"),
            CgroupVersion::V1 => self.path.join("memory.limit_in_bytes"),
        };

        std::fs::write(&limit_path, bytes.to_string())?;
        Ok(())
    }

    /// Set CPU limit
    pub fn set_cpu_limit(&mut self, cpus: &str) -> Result<()> {
        info!("Setting CPU limit for {}: {}", self.name, cpus);
        self.limits.cpuset_cpus = Some(cpus.to_string());

        let cpuset_path = self.path.join("cpuset.cpus");
        std::fs::write(&cpuset_path, cpus)?;
        Ok(())
    }

    /// Add process to cgroup
    pub fn add_process(&self, pid: u32) -> Result<()> {
        info!("Adding PID {} to cgroup {}", pid, self.name);

        let procs_file = match self.version() {
            CgroupVersion::V2 => self.path.join("cgroup.procs"),
            CgroupVersion::V1 => self.path.join("cgroup.procs"),
        };

        std::fs::write(&procs_file, pid.to_string())?;
        Ok(())
    }

    fn version(&self) -> CgroupVersion {
        if self.path.join("cgroup.controllers").exists() {
            CgroupVersion::V2
        } else {
            CgroupVersion::V1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cgroup_limits_default() {
        let limits = CgroupLimits::default();
        assert!(limits.cpuset_cpus.is_none());
        assert!(limits.memory_limit.is_none());
        assert!(limits.cpu_shares.is_none());
    }
}
