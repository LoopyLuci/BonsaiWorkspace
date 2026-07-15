//! BUEB (Backend Unified Execution Backend): hardware detection and
//! task-aware compute device allocation for ML inference/training workloads.
//!
//! Typical usage:
//! ```no_run
//! use backend::{initialize, allocate, TaskRequirements, TaskType, Precision};
//!
//! initialize().unwrap();
//! let allocation = allocate(&TaskRequirements {
//!     task_type: TaskType::Inference,
//!     estimated_memory_bytes: 600_000_000,
//!     min_compute_units: 0,
//!     precision: Precision::Auto,
//!     allow_fallback: true,
//! });
//! println!("batch_size={}", allocation.batch_size);
//! ```

pub mod allocator;
pub mod cpu;
pub mod detect;
pub mod error;
pub mod types;

pub use cpu::{
    batched_matmul, elementwise_add, elementwise_mul, elementwise_relu, has_simd, matmul, max,
    mean, min, softmax, sum,
};
pub use detect::detect_hardware;
pub use error::{Error, Result};
pub use types::*;

use std::sync::OnceLock;

static PROFILE: OnceLock<HardwareProfile> = OnceLock::new();

/// Detect and cache this machine's hardware profile. Idempotent: calling it
/// again after a successful initialization is a no-op.
pub fn initialize() -> Result<()> {
    if PROFILE.get().is_some() {
        return Ok(());
    }
    let detected = detect_hardware()?;
    // Another thread may have raced us; either way the profile is now set.
    let _ = PROFILE.set(detected);
    Ok(())
}

/// Return the cached hardware profile, detecting it on first use if
/// [`initialize`] was not called explicitly.
pub fn profile() -> HardwareProfile {
    PROFILE
        .get_or_init(|| detect_hardware().expect("failed to detect hardware"))
        .clone()
}

/// Allocate compute devices for `task` using the cached hardware profile.
pub fn allocate(task: &TaskRequirements) -> DeviceAllocation {
    allocator::allocate(&profile(), task)
}

/// Whether at least one GPU was detected on this machine.
pub fn has_gpu() -> bool {
    !profile().gpus.is_empty()
}

/// Number of GPUs detected on this machine.
pub fn gpu_count() -> usize {
    profile().gpus.len()
}

/// Number of logical CPU cores.
pub fn cpu_cores() -> u32 {
    profile().cpu.logical_cores
}

/// Total system memory in bytes.
pub fn total_memory() -> u64 {
    profile().memory.total_bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_then_profile_is_consistent() {
        initialize().unwrap();
        let p1 = profile();
        let p2 = profile();
        assert_eq!(p1.cpu.logical_cores, p2.cpu.logical_cores);
        assert!(p1.cpu.logical_cores >= 1);
    }

    #[test]
    fn cpu_cores_matches_profile() {
        initialize().unwrap();
        assert_eq!(cpu_cores(), profile().cpu.logical_cores);
    }

    #[test]
    fn total_memory_is_nonzero_on_a_real_machine() {
        initialize().unwrap();
        assert!(total_memory() > 0);
    }

    #[test]
    fn gpu_count_matches_has_gpu() {
        initialize().unwrap();
        assert_eq!(has_gpu(), gpu_count() > 0);
    }

    #[test]
    fn allocate_returns_at_least_one_device() {
        initialize().unwrap();
        let allocation = allocate(&TaskRequirements {
            task_type: TaskType::Inference,
            estimated_memory_bytes: 100_000_000,
            min_compute_units: 0,
            precision: Precision::Auto,
            allow_fallback: true,
        });
        assert!(!allocation.devices.is_empty());
    }
}
