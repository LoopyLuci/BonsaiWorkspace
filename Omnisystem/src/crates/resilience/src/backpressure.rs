use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Backpressure controller for flow control
pub struct BackpressureController {
    name: String,
    queue_capacity: usize,
    current_load: Arc<RwLock<usize>>,
    peak_load: Arc<RwLock<usize>>,
    rejected_count: Arc<RwLock<u64>>,
    backpressure_threshold: f64, // 0.0 to 1.0
}

impl BackpressureController {
    pub fn new(name: &str, queue_capacity: usize) -> Self {
        Self {
            name: name.to_string(),
            queue_capacity,
            current_load: Arc::new(RwLock::new(0)),
            peak_load: Arc::new(RwLock::new(0)),
            rejected_count: Arc::new(RwLock::new(0)),
            backpressure_threshold: 0.8, // Trigger at 80% capacity
        }
    }

    /// Set backpressure threshold
    pub fn set_threshold(&mut self, threshold: f64) {
        self.backpressure_threshold = threshold.clamp(0.0, 1.0);
    }

    /// Try to add item
    pub fn try_add(&self) -> Result<BackpressureGuard, BackpressureError> {
        let mut load = self.current_load.write();

        let capacity_threshold = (self.queue_capacity as f64 * self.backpressure_threshold) as usize;

        if *load >= capacity_threshold {
            *self.rejected_count.write() += 1;
            return Err(BackpressureError::BackpressureActive);
        }

        if *load >= self.queue_capacity {
            *self.rejected_count.write() += 1;
            return Err(BackpressureError::QueueFull);
        }

        *load += 1;

        // Update peak
        let mut peak = self.peak_load.write();
        if *load > *peak {
            *peak = *load;
        }

        Ok(BackpressureGuard::new(self.current_load.clone()))
    }

    /// Get current load
    pub fn current_load(&self) -> usize {
        *self.current_load.read()
    }

    /// Get load percentage
    pub fn load_percent(&self) -> f64 {
        if self.queue_capacity == 0 {
            0.0
        } else {
            (self.current_load() as f64 / self.queue_capacity as f64) * 100.0
        }
    }

    /// Should apply backpressure?
    pub fn should_backpressure(&self) -> bool {
        let threshold = (self.queue_capacity as f64 * self.backpressure_threshold) as usize;
        self.current_load() >= threshold
    }

    /// Get state
    pub fn state(&self) -> BackpressureState {
        BackpressureState {
            name: self.name.clone(),
            current_load: self.current_load(),
            queue_capacity: self.queue_capacity,
            peak_load: *self.peak_load.read(),
            load_percent: self.load_percent(),
            backpressure_active: self.should_backpressure(),
            rejected_count: *self.rejected_count.read(),
        }
    }
}

/// Guard that reduces load on drop
pub struct BackpressureGuard {
    current_load: Arc<RwLock<usize>>,
}

impl BackpressureGuard {
    fn new(current_load: Arc<RwLock<usize>>) -> Self {
        Self { current_load }
    }
}

impl Drop for BackpressureGuard {
    fn drop(&mut self) {
        let mut load = self.current_load.write();
        if *load > 0 {
            *load -= 1;
        }
    }
}

/// Backpressure state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressureState {
    pub name: String,
    pub current_load: usize,
    pub queue_capacity: usize,
    pub peak_load: usize,
    pub load_percent: f64,
    pub backpressure_active: bool,
    pub rejected_count: u64,
}

/// Backpressure errors
#[derive(Debug, thiserror::Error)]
pub enum BackpressureError {
    #[error("Backpressure active, request rejected")]
    BackpressureActive,
    #[error("Queue full")]
    QueueFull,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backpressure_add() {
        let bp = BackpressureController::new("test", 100);
        let guard = bp.try_add();
        assert!(guard.is_ok());
        assert_eq!(bp.current_load(), 1);
    }

    #[test]
    fn test_backpressure_threshold() {
        let mut bp = BackpressureController::new("test", 100);
        bp.set_threshold(0.5);

        // Fill to 50 items
        let mut guards = Vec::new();
        for _ in 0..50 {
            if let Ok(g) = bp.try_add() {
                guards.push(g);
            }
        }

        // Should trigger backpressure
        assert!(bp.should_backpressure());
    }

    #[test]
    fn test_backpressure_queue_full() {
        let bp = BackpressureController::new("test", 2);

        let _g1 = bp.try_add();
        let _g2 = bp.try_add();

        // Queue is now full
        let result = bp.try_add();
        assert!(result.is_err());
    }

    #[test]
    fn test_backpressure_load_percent() {
        let bp = BackpressureController::new("test", 100);
        assert_eq!(bp.load_percent(), 0.0);

        let _guard = bp.try_add();
        assert_eq!(bp.load_percent(), 1.0);
    }
}
