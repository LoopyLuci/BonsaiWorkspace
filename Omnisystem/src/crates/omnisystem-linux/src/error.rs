//! Error types

/// Errors produced by the Linux OS-integration modules (cgroups, eBPF,
/// KVM, netlink, perf, systemd)
#[derive(Debug)]
pub enum LinuxError {
    /// cgroup manager/controller failure
    Cgroup(String),
    /// KVM virtualization failure
    KVM(String),
    /// systemd integration failure
    Systemd(String),
    /// Underlying filesystem/syscall I/O error
    IO(std::io::Error),
    /// Other error
    Other(String),
}

impl std::fmt::Display for LinuxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinuxError::Cgroup(msg) => write!(f, "cgroup error: {}", msg),
            LinuxError::KVM(msg) => write!(f, "KVM error: {}", msg),
            LinuxError::Systemd(msg) => write!(f, "systemd error: {}", msg),
            LinuxError::IO(err) => write!(f, "I/O error: {}", err),
            LinuxError::Other(msg) => write!(f, "error: {}", msg),
        }
    }
}

impl std::error::Error for LinuxError {}

impl From<std::io::Error> for LinuxError {
    fn from(err: std::io::Error) -> Self {
        LinuxError::IO(err)
    }
}

/// Result type
pub type Result<T> = std::result::Result<T, LinuxError>;
