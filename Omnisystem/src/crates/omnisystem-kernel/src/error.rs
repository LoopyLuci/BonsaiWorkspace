//! Error types for omnisystem-kernel.

#[derive(Debug, Clone)]
pub enum KernelError {
    /// Process/thread management failure.
    ProcessError(String),
    /// Memory management failure (allocation, mapping, etc).
    MemoryError(String),
    /// Inter-process communication failure.
    IPCError(String),
    /// Interrupt handling failure.
    InterruptError(String),
    /// Capability check/grant failure.
    CapabilityError(String),
    /// Catch-all for other kernel errors.
    Unknown(String),
}

impl std::fmt::Display for KernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KernelError::ProcessError(msg) => write!(f, "process error: {}", msg),
            KernelError::MemoryError(msg) => write!(f, "memory error: {}", msg),
            KernelError::IPCError(msg) => write!(f, "IPC error: {}", msg),
            KernelError::InterruptError(msg) => write!(f, "interrupt error: {}", msg),
            KernelError::CapabilityError(msg) => write!(f, "capability error: {}", msg),
            KernelError::Unknown(msg) => write!(f, "kernel error: {}", msg),
        }
    }
}

impl std::error::Error for KernelError {}

/// Result type used throughout omnisystem-kernel.
pub type Result<T> = std::result::Result<T, KernelError>;
