/// Resource guard — concurrency limiter + memory pressure check.
///
/// Every heavy Tauri command should acquire a guard before running so
/// that the app degrades gracefully under load instead of OOM-crashing.
use std::sync::Arc;
use tokio::sync::{Semaphore, SemaphorePermit};

/// Configurable limits. Keep defaults conservative so low-end machines work.
#[derive(Clone, Debug)]
pub struct GuardConfig {
    /// Max simultaneous heavy operations (inference, training, …).
    pub max_concurrent: usize,
    /// Refuse new work when free RSS exceeds this fraction of total RAM (0–1).
    /// E.g. 0.90 = deny when 90 % of RAM is used.
    pub memory_pressure_threshold: f32,
}

impl Default for GuardConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 4,
            memory_pressure_threshold: 0.90,
        }
    }
}

pub struct ResourceGuard {
    semaphore: Arc<Semaphore>,
    config: GuardConfig,
}

#[derive(Debug, thiserror::Error)]
pub enum GuardError {
    #[error("too many concurrent operations (limit {0})")]
    TooManyConcurrent(usize),
    #[error("memory pressure too high ({used_pct:.0}% used, threshold {threshold:.0}%)")]
    MemoryPressure { used_pct: f32, threshold: f32 },
}

impl ResourceGuard {
    pub fn new(config: GuardConfig) -> Arc<Self> {
        Arc::new(Self {
            semaphore: Arc::new(Semaphore::new(config.max_concurrent)),
            config,
        })
    }

    /// Try to acquire a concurrency slot; also check memory pressure.
    /// Returns a permit that releases the slot when dropped.
    pub async fn acquire(&self) -> Result<SemaphorePermit<'_>, GuardError> {
        // Memory check first — fast, synchronous.
        self.check_memory()?;

        // Try to take a slot without blocking indefinitely.
        self.semaphore
            .try_acquire()
            .map_err(|_| GuardError::TooManyConcurrent(self.config.max_concurrent))
    }

    fn check_memory(&self) -> Result<(), GuardError> {
        use sysinfo::System;
        let mut sys = System::new();
        sys.refresh_memory();
        let total = sys.total_memory();
        if total == 0 {
            return Ok(()); // can't measure — don't block
        }
        let used = total - sys.available_memory();
        let used_pct = used as f32 / total as f32;
        let threshold = self.config.memory_pressure_threshold;
        if used_pct > threshold {
            return Err(GuardError::MemoryPressure {
                used_pct: used_pct * 100.0,
                threshold: threshold * 100.0,
            });
        }
        Ok(())
    }

    /// Current memory pressure as a 0–1 fraction. Returns 0 if unavailable.
    pub fn memory_pressure() -> f32 {
        use sysinfo::System;
        let mut sys = System::new();
        sys.refresh_memory();
        let total = sys.total_memory();
        if total == 0 {
            return 0.0;
        }
        let used = total - sys.available_memory();
        used as f32 / total as f32
    }
}
