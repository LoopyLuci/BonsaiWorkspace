//! Performance metrics and monitoring

/// Performance metrics
#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub compilation_time_ms: u128,
    pub cache_time_ms: u128,
    pub linking_time_ms: u128,
    pub memory_peak_mb: u64,
    pub cache_hit_rate: f32,
}

impl Metrics {
    pub fn total_time_ms(&self) -> u128 {
        self.compilation_time_ms + self.cache_time_ms + self.linking_time_ms
    }
}
