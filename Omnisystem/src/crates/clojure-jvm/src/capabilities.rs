//! Capability-based access control for JVM sandbox

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// A capability token that grants specific permissions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityToken {
    /// Unique token ID
    pub id: String,
    /// Associated capabilities
    pub capabilities: Vec<Capability>,
    /// Creation timestamp (Unix seconds)
    pub created_at: u64,
    /// Expiration timestamp (Unix seconds, 0 = never)
    pub expires_at: u64,
}

/// Types of capabilities that can be granted
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Filesystem access to specific paths
    Filesystem(Vec<String>),

    /// Network access
    Network,

    /// Network access to specific hosts:ports
    NetworkRestricted(Vec<String>),

    /// Threading capability
    Threading,

    /// CPU resource limit (cores)
    CpuLimit(u32),

    /// Memory resource limit (MB)
    MemoryLimit(u64),

    /// Time limit (seconds)
    TimeLimit(u32),

    /// I/O operations (IOPS)
    IopLimit(u32),

    /// System call access (list of allowed syscalls)
    SyscallWhitelist(Vec<String>),

    /// Inter-process communication
    IPC,

    /// Environment variable access
    EnvironmentVariables(Vec<String>),
}

/// Access control enforcement
pub struct AccessControl {
    capabilities: Vec<Capability>,
    allowed_paths: HashSet<PathBuf>,
    network_allowed: bool,
    threading_allowed: bool,
}

impl AccessControl {
    /// Create new access control from capabilities
    pub fn new(capabilities: Vec<Capability>) -> Self {
        let mut allowed_paths = HashSet::new();
        let mut network_allowed = false;
        let mut threading_allowed = false;

        for cap in &capabilities {
            match cap {
                Capability::Filesystem(paths) => {
                    for path_str in paths {
                        allowed_paths.insert(PathBuf::from(path_str));
                    }
                }
                Capability::Network => network_allowed = true,
                Capability::NetworkRestricted(_) => network_allowed = true,
                Capability::Threading => threading_allowed = true,
                _ => {}
            }
        }

        Self {
            capabilities,
            allowed_paths,
            network_allowed,
            threading_allowed,
        }
    }

    /// Check if path access is allowed
    pub fn can_access_path(&self, path: &str) -> bool {
        let path = Path::new(path);

        // Check if any allowed path is a parent of requested path
        for allowed in &self.allowed_paths {
            if path.starts_with(allowed) {
                return true;
            }
        }

        false
    }

    /// Check if network access is allowed
    pub fn can_access_network(&self) -> bool {
        self.network_allowed
    }

    /// Check if threading is allowed
    pub fn can_use_threading(&self) -> bool {
        self.threading_allowed
    }

    /// Get CPU limit if set
    pub fn cpu_limit(&self) -> Option<u32> {
        self.capabilities.iter().find_map(|cap| {
            if let Capability::CpuLimit(cores) = cap {
                Some(*cores)
            } else {
                None
            }
        })
    }

    /// Get memory limit if set
    pub fn memory_limit(&self) -> Option<u64> {
        self.capabilities.iter().find_map(|cap| {
            if let Capability::MemoryLimit(mb) = cap {
                Some(*mb)
            } else {
                None
            }
        })
    }

    /// Get time limit if set
    pub fn time_limit(&self) -> Option<u32> {
        self.capabilities.iter().find_map(|cap| {
            if let Capability::TimeLimit(secs) = cap {
                Some(*secs)
            } else {
                None
            }
        })
    }

    /// Check if capability is allowed
    pub fn has_capability(&self, cap: &Capability) -> bool {
        self.capabilities.iter().any(|c| {
            std::mem::discriminant(c) == std::mem::discriminant(cap)
        })
    }
}

impl Default for AccessControl {
    fn default() -> Self {
        Self::new(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_access() {
        let access = AccessControl::new(vec![
            Capability::Filesystem(vec!["/safe".to_string()]),
        ]);

        assert!(access.can_access_path("/safe/file.txt"));
        assert!(!access.can_access_path("/unsafe/file.txt"));
    }

    #[test]
    fn test_network_access() {
        let access = AccessControl::new(vec![Capability::Network]);
        assert!(access.can_access_network());

        let access_restricted = AccessControl::new(vec![]);
        assert!(!access_restricted.can_access_network());
    }

    #[test]
    fn test_capability_limits() {
        let access = AccessControl::new(vec![
            Capability::CpuLimit(4),
            Capability::MemoryLimit(1024),
            Capability::TimeLimit(300),
        ]);

        assert_eq!(access.cpu_limit(), Some(4));
        assert_eq!(access.memory_limit(), Some(1024));
        assert_eq!(access.time_limit(), Some(300));
    }
}
