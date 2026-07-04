use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Tracks memory allocations and deallocations
pub struct AllocationTracker {
    current_bytes: Arc<AtomicUsize>,
    peak_bytes: Arc<AtomicUsize>,
    total_allocations: Arc<AtomicUsize>,
    total_deallocations: Arc<AtomicUsize>,
}

impl AllocationTracker {
    pub fn new() -> Self {
        Self {
            current_bytes: Arc::new(AtomicUsize::new(0)),
            peak_bytes: Arc::new(AtomicUsize::new(0)),
            total_allocations: Arc::new(AtomicUsize::new(0)),
            total_deallocations: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Record an allocation
    pub fn record_allocation(&self, size: usize) {
        let current = self.current_bytes.fetch_add(size, Ordering::Relaxed);
        let new_current = current + size;

        // Update peak
        let mut peak = self.peak_bytes.load(Ordering::Relaxed);
        while peak < new_current {
            match self.peak_bytes.compare_exchange(
                peak,
                new_current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => peak = actual,
            }
        }

        self.total_allocations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a deallocation
    pub fn record_deallocation(&self, size: usize) {
        self.current_bytes.fetch_sub(size, Ordering::Relaxed);
        self.total_deallocations.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current memory usage in bytes
    pub fn current_bytes(&self) -> usize {
        self.current_bytes.load(Ordering::Relaxed)
    }

    /// Get peak memory usage in bytes
    pub fn peak_bytes(&self) -> usize {
        self.peak_bytes.load(Ordering::Relaxed)
    }

    /// Get total allocations
    pub fn total_allocations(&self) -> usize {
        self.total_allocations.load(Ordering::Relaxed)
    }

    /// Get total deallocations
    pub fn total_deallocations(&self) -> usize {
        self.total_deallocations.load(Ordering::Relaxed)
    }

    /// Get memory efficiency (deallocations / allocations)
    pub fn efficiency(&self) -> f64 {
        let allocs = self.total_allocations() as f64;
        if allocs == 0.0 {
            1.0
        } else {
            self.total_deallocations() as f64 / allocs
        }
    }
}

impl Default for AllocationTracker {
    fn default() -> Self {
        Self::new()
    }
}
