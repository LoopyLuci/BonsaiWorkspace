use std::sync::atomic::{AtomicU64, Ordering};

pub struct HotPathProfiler {
    samples: AtomicU64,
}

impl HotPathProfiler {
    pub fn new() -> Self {
        Self {
            samples: AtomicU64::new(0),
        }
    }

    pub fn record_sample(&self) {
        self.samples.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_sample_count(&self) -> u64 {
        self.samples.load(Ordering::Relaxed)
    }
}
