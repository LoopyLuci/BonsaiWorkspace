//! Deterministic clock for reproducible execution

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Deterministic clock for reproducible emulation execution
#[derive(Debug, Clone)]
pub struct DeterministicClock {
    cycle_count: Arc<AtomicU64>,
}

impl DeterministicClock {
    /// Create a new deterministic clock starting at cycle 0
    pub fn new() -> Self {
        Self {
            cycle_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Advance the clock by one cycle
    pub fn advance(&self) {
        self.cycle_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Advance the clock by multiple cycles
    pub fn advance_by(&self, cycles: u64) {
        self.cycle_count.fetch_add(cycles, Ordering::SeqCst);
    }

    /// Get the current cycle count
    pub fn cycle_count(&self) -> u64 {
        self.cycle_count.load(Ordering::SeqCst)
    }

    /// Reset the clock to 0
    pub fn reset(&self) {
        self.cycle_count.store(0, Ordering::SeqCst);
    }

    /// Set the clock to a specific cycle count
    pub fn set(&self, cycle_count: u64) {
        self.cycle_count.store(cycle_count, Ordering::SeqCst);
    }
}

impl Default for DeterministicClock {
    fn default() -> Self {
        Self::new()
    }
}

/// Clock trait for extensibility
pub trait Clock: Send + Sync {
    /// Get the current cycle count
    fn cycle_count(&self) -> u64;

    /// Advance by one cycle
    fn advance(&self);

    /// Advance by multiple cycles
    fn advance_by(&self, cycles: u64);

    /// Reset to 0
    fn reset(&self);
}

impl Clock for DeterministicClock {
    fn cycle_count(&self) -> u64 {
        self.cycle_count()
    }

    fn advance(&self) {
        self.advance();
    }

    fn advance_by(&self, cycles: u64) {
        self.advance_by(cycles);
    }

    fn reset(&self) {
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_creation() {
        let clock = DeterministicClock::new();
        assert_eq!(clock.cycle_count(), 0);
    }

    #[test]
    fn test_clock_advance() {
        let clock = DeterministicClock::new();
        clock.advance();
        assert_eq!(clock.cycle_count(), 1);

        clock.advance();
        assert_eq!(clock.cycle_count(), 2);
    }

    #[test]
    fn test_clock_advance_by() {
        let clock = DeterministicClock::new();
        clock.advance_by(100);
        assert_eq!(clock.cycle_count(), 100);

        clock.advance_by(50);
        assert_eq!(clock.cycle_count(), 150);
    }

    #[test]
    fn test_clock_reset() {
        let clock = DeterministicClock::new();
        clock.advance_by(1000);
        assert_eq!(clock.cycle_count(), 1000);

        clock.reset();
        assert_eq!(clock.cycle_count(), 0);
    }

    #[test]
    fn test_clock_set() {
        let clock = DeterministicClock::new();
        clock.set(12345);
        assert_eq!(clock.cycle_count(), 12345);
    }

    #[test]
    fn test_clock_concurrent_access() {
        let clock = DeterministicClock::new();
        let mut handles = vec![];

        for _ in 0..10 {
            let clk = clock.clone();
            let handle = std::thread::spawn(move || {
                for _ in 0..100 {
                    clk.advance();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(clock.cycle_count(), 1000);
    }
}
