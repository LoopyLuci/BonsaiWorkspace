/// Kernel bindings - Safe Rust wrapper around OmniOS kernel

use std::sync::Arc;

/// Handle to the Omnisystem kernel
pub struct OmniKernelHandle {
    kernel: Arc<omnisystem_kernel::OmniKernel>,
}

impl OmniKernelHandle {
    pub fn new(kernel: Arc<omnisystem_kernel::OmniKernel>) -> Self {
        OmniKernelHandle { kernel }
    }

    /// Get memory manager
    pub fn memory(&self) -> Arc<omnisystem_kernel::memory::MemoryManager> {
        self.kernel.memory()
    }

    /// Get process manager
    pub fn process(&self) -> Arc<omnisystem_kernel::process::ProcessManager> {
        self.kernel.process()
    }

    /// Get scheduler
    pub fn scheduler(&self) -> Arc<omnisystem_kernel::scheduling::Scheduler> {
        self.kernel.scheduler()
    }

    /// Get device manager
    pub fn device(&self) -> Arc<omnisystem_kernel::device::DeviceManager> {
        self.kernel.device()
    }

    /// Get IPC manager
    pub fn ipc(&self) -> Arc<omnisystem_kernel::ipc::IPCManager> {
        self.kernel.ipc()
    }

    /// Get capability manager
    pub fn capability(&self) -> Arc<omnisystem_kernel::capability::CapabilityManager> {
        self.kernel.capability()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_handle() {
        // Test would require running kernel which is async
        // Placeholder for future tests
    }
}
