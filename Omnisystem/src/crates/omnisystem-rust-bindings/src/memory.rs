/// Memory management bindings

pub use omnisystem_kernel::memory::{MemoryManager as KernelMemoryManager, MemoryStats};

pub struct MemoryManager;

impl MemoryManager {
    pub fn get_stats(kernel_mm: &KernelMemoryManager) -> MemoryStats {
        kernel_mm.get_stats()
    }
}
