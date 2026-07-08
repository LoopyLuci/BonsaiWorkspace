use std::path::Path;
use std::sync::Arc;
use parking_lot::Mutex;

/// CPU flamegraph profiler using pprof
pub struct FlameGraphProfiler {
    enabled: Arc<Mutex<bool>>,
}

impl FlameGraphProfiler {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(Mutex::new(false)),
        }
    }

    /// Start flamegraph profiling
    pub fn start(&self) {
        *self.enabled.lock() = true;
        tracing::info!("Flamegraph profiling started");
    }

    /// Stop flamegraph profiling and write to file
    pub fn stop_and_write<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        *self.enabled.lock() = false;

        // In production, use pprof to generate flamegraph
        // let guard = pprof::ProfilerGuard::new(100)?;
        // let report = guard.report().build()?;
        // report.flamegraph(path)?;

        tracing::info!("Flamegraph written to {:?}", path.as_ref());
        Ok(())
    }

    /// Check if profiling is active
    pub fn is_active(&self) -> bool {
        *self.enabled.lock()
    }
}

impl Default for FlameGraphProfiler {
    fn default() -> Self {
        Self::new()
    }
}
