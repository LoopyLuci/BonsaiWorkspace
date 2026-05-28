//! Global Sequence Number (GSN) allocator.
//!
//! All chunks across all lanes share a unified 64-bit sequence space.
//! This enables ordered reassembly regardless of which physical path
//! delivered each chunk.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Thread-safe monotonic GSN allocator.
#[derive(Debug, Clone)]
pub struct GsnAllocator {
    next: Arc<AtomicU64>,
}

impl GsnAllocator {
    /// Create a new allocator starting at 0.
    pub fn new() -> Self {
        Self { next: Arc::new(AtomicU64::new(0)) }
    }

    /// Create starting at a specific value (useful for resuming a transfer).
    pub fn starting_at(n: u64) -> Self {
        Self { next: Arc::new(AtomicU64::new(n)) }
    }

    /// Allocate the next GSN (monotonically increasing, wraps at u64::MAX).
    pub fn next(&self) -> u64 {
        self.next.fetch_add(1, Ordering::Relaxed)
    }

    /// Allocate a contiguous range of `count` GSNs.
    /// Returns the first GSN; the rest are [first, first+count).
    pub fn next_range(&self, count: u64) -> u64 {
        self.next.fetch_add(count, Ordering::Relaxed)
    }

    /// Current value (for checkpointing, not for allocation).
    pub fn current(&self) -> u64 {
        self.next.load(Ordering::Relaxed)
    }
}

impl Default for GsnAllocator {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential() {
        let alloc = GsnAllocator::new();
        assert_eq!(alloc.next(), 0);
        assert_eq!(alloc.next(), 1);
        assert_eq!(alloc.next(), 2);
    }

    #[test]
    fn range_allocation() {
        let alloc = GsnAllocator::new();
        let first = alloc.next_range(10);
        assert_eq!(first, 0);
        let next = alloc.next();
        assert_eq!(next, 10);
    }

    #[test]
    fn concurrent() {
        let alloc = GsnAllocator::new();
        let handles: Vec<_> = (0..4).map(|_| {
            let a = alloc.clone();
            std::thread::spawn(move || {
                (0..1000).map(|_| a.next()).collect::<Vec<_>>()
            })
        }).collect();
        let mut all: Vec<u64> = handles.into_iter().flat_map(|h| h.join().unwrap()).collect();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 4000, "each GSN must be unique");
    }
}
