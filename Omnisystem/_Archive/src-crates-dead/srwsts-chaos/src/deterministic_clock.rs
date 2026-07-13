//! Deterministic virtual clock for fault injection and chaos engineering.
//!
//! All faults are tied to a deterministic virtual clock, enabling perfect
//! reproducibility across test runs with the same seed.

use crate::error::{ChaosError, Result};
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{debug, trace};

/// Deterministic virtual clock for chaos testing.
///
/// All times are in epoch seconds. Same clock time → same faults at same moments.
#[derive(Clone)]
pub struct DeterministicClock {
    state: Arc<RwLock<ClockState>>,
}

/// Internal clock state.
struct ClockState {
    /// Current virtual time (epoch seconds).
    current_time: u64,
    /// Time scaling factor (1.0 = real-time, 0.1 = 10x faster).
    time_scale: f64,
    /// Whether clock is paused.
    is_paused: bool,
    /// Total virtual time elapsed.
    total_elapsed: u64,
    /// Clock synchronization epoch for determinism.
    sync_epoch: u64,
}

impl DeterministicClock {
    /// Create a new deterministic clock starting at given epoch time.
    pub fn new(start_time: u64) -> Self {
        Self {
            state: Arc::new(RwLock::new(ClockState {
                current_time: start_time,
                time_scale: 1.0,
                is_paused: false,
                total_elapsed: 0,
                sync_epoch: start_time,
            })),
        }
    }

    /// Get current virtual time.
    pub fn now(&self) -> u64 {
        let state = self.state.read();
        state.current_time
    }

    /// Advance time by given seconds.
    pub fn advance(&self, seconds: u64) -> Result<u64> {
        let mut state = self.state.write();

        if state.is_paused {
            return Err(ChaosError::ClockError("Clock is paused".to_string()));
        }

        let scaled_advance = (seconds as f64 * state.time_scale) as u64;
        state.current_time = state.current_time.saturating_add(scaled_advance);
        state.total_elapsed = state.total_elapsed.saturating_add(scaled_advance);

        debug!(
            "Clock advanced by {}s (scaled: {}s), now at {}",
            seconds, scaled_advance, state.current_time
        );

        Ok(state.current_time)
    }

    /// Jump time to specific point.
    pub fn jump_to(&self, time: u64) -> Result<()> {
        let mut state = self.state.write();

        if state.is_paused {
            return Err(ChaosError::ClockError("Clock is paused".to_string()));
        }

        let jump_distance = time.abs_diff(state.current_time);
        state.current_time = time;
        state.total_elapsed = state.total_elapsed.saturating_add(jump_distance);

        trace!("Clock jumped to {} (distance: {}s)", time, jump_distance);

        Ok(())
    }

    /// Pause the clock.
    pub fn pause(&self) {
        let mut state = self.state.write();
        state.is_paused = true;
        debug!("Clock paused at {}", state.current_time);
    }

    /// Resume the clock.
    pub fn resume(&self) {
        let mut state = self.state.write();
        state.is_paused = false;
        debug!("Clock resumed from {}", state.current_time);
    }

    /// Check if clock is paused.
    pub fn is_paused(&self) -> bool {
        let state = self.state.read();
        state.is_paused
    }

    /// Set time scale factor (1.0 = real-time, 0.5 = 2x slower, 2.0 = 2x faster).
    pub fn set_time_scale(&self, scale: f64) -> Result<()> {
        if scale <= 0.0 {
            return Err(ChaosError::InvalidParameter(
                "Time scale must be > 0".to_string(),
            ));
        }

        let mut state = self.state.write();
        state.time_scale = scale;

        debug!("Clock time scale set to {}x", scale);

        Ok(())
    }

    /// Get current time scale.
    pub fn time_scale(&self) -> f64 {
        let state = self.state.read();
        state.time_scale
    }

    /// Get total elapsed virtual time.
    pub fn total_elapsed(&self) -> u64 {
        let state = self.state.read();
        state.total_elapsed
    }

    /// Reset clock to initial state (for test replay).
    pub fn reset(&self, start_time: u64) {
        let mut state = self.state.write();
        state.current_time = start_time;
        state.is_paused = false;
        state.total_elapsed = 0;
        state.sync_epoch = start_time;

        debug!("Clock reset to {}", start_time);
    }

    /// Get synchronization epoch (for deterministic replay).
    pub fn sync_epoch(&self) -> u64 {
        let state = self.state.read();
        state.sync_epoch
    }

    /// Check if time is within bounds.
    pub fn is_within_bounds(&self, time: u64, max_time: u64) -> bool {
        time >= self.now() && time <= max_time
    }
}

/// Clock watcher for monitoring time progression.
pub struct ClockWatcher {
    clock: DeterministicClock,
    last_check: u64,
}

impl ClockWatcher {
    /// Create a new clock watcher.
    pub fn new(clock: DeterministicClock) -> Self {
        let last_check = clock.now();
        Self { clock, last_check }
    }

    /// Check time delta since last check.
    pub fn delta(&mut self) -> u64 {
        let current = self.clock.now();
        let delta = current.saturating_sub(self.last_check);
        self.last_check = current;
        delta
    }

    /// Reset watcher.
    pub fn reset(&mut self) {
        self.last_check = self.clock.now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_creation() {
        let clock = DeterministicClock::new(1000);
        assert_eq!(clock.now(), 1000);
    }

    #[test]
    fn test_clock_advance() {
        let clock = DeterministicClock::new(1000);
        clock.advance(50).unwrap();
        assert_eq!(clock.now(), 1050);
    }

    #[test]
    fn test_clock_jump() {
        let clock = DeterministicClock::new(1000);
        clock.jump_to(2000).unwrap();
        assert_eq!(clock.now(), 2000);
    }

    #[test]
    fn test_clock_pause_resume() {
        let clock = DeterministicClock::new(1000);
        clock.pause();
        assert!(clock.is_paused());
        assert!(clock.advance(10).is_err());

        clock.resume();
        assert!(!clock.is_paused());
        clock.advance(10).unwrap();
        assert_eq!(clock.now(), 1010);
    }

    #[test]
    fn test_clock_time_scale() {
        let clock = DeterministicClock::new(1000);
        clock.set_time_scale(2.0).unwrap();
        clock.advance(10).unwrap();
        assert_eq!(clock.now(), 1020); // 10 * 2.0
    }

    #[test]
    fn test_clock_reset() {
        let clock = DeterministicClock::new(1000);
        clock.advance(100).unwrap();
        assert_eq!(clock.now(), 1100);

        clock.reset(1000);
        assert_eq!(clock.now(), 1000);
        assert_eq!(clock.total_elapsed(), 0);
    }

    #[test]
    fn test_clock_watcher() {
        let clock = DeterministicClock::new(1000);
        let mut watcher = ClockWatcher::new(clock.clone());

        clock.advance(50).unwrap();
        assert_eq!(watcher.delta(), 50);

        clock.advance(30).unwrap();
        assert_eq!(watcher.delta(), 30);
    }

    #[test]
    fn test_clock_invalid_time_scale() {
        let clock = DeterministicClock::new(1000);
        assert!(clock.set_time_scale(0.0).is_err());
        assert!(clock.set_time_scale(-1.0).is_err());
    }
}
