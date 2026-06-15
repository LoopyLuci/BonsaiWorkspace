use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use crate::gpu_layer::BackendType;

#[derive(Debug, Default)]
struct BackendStats {
    successes: u64,
    failures: u64,
    last_error: Option<String>,
    last_success_at: Option<Instant>,
    last_failure_at: Option<Instant>,
}

#[derive(Debug, Default)]
pub struct GpuTelemetry {
    stats: Mutex<HashMap<BackendType, BackendStats>>,
}

impl GpuTelemetry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_success(&self, backend: &BackendType) {
        let mut map = self.stats.lock().unwrap();
        let s = map.entry(backend.clone()).or_default();
        s.successes += 1;
        s.last_success_at = Some(Instant::now());
    }

    pub fn record_failure(&self, backend: &BackendType, error: &str) {
        let mut map = self.stats.lock().unwrap();
        let s = map.entry(backend.clone()).or_default();
        s.failures += 1;
        s.last_error = Some(error.to_owned());
        s.last_failure_at = Some(Instant::now());
    }

    pub fn success_count(&self, backend: &BackendType) -> u64 {
        self.stats
            .lock()
            .unwrap()
            .get(backend)
            .map_or(0, |s| s.successes)
    }

    pub fn failure_count(&self, backend: &BackendType) -> u64 {
        self.stats
            .lock()
            .unwrap()
            .get(backend)
            .map_or(0, |s| s.failures)
    }
}
