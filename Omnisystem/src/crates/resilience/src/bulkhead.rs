use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Bulkhead isolation (limit concurrent operations)
pub struct Bulkhead {
    name: String,
    max_concurrent: usize,
    current_count: Arc<RwLock<usize>>,
    queue_depth: Arc<RwLock<usize>>,
    max_queue_depth: usize,
    rejected_count: Arc<RwLock<u64>>,
}

impl Bulkhead {
    pub fn new(name: &str, max_concurrent: usize, max_queue_depth: usize) -> Self {
        Self {
            name: name.to_string(),
            max_concurrent,
            current_count: Arc::new(RwLock::new(0)),
            queue_depth: Arc::new(RwLock::new(0)),
            max_queue_depth,
            rejected_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Try to acquire a slot
    pub fn try_acquire(&self) -> Result<BulkheadGuard, BulkheadError> {
        let mut count = self.current_count.write();

        if *count >= self.max_concurrent {
            // Check queue
            let mut queue = self.queue_depth.write();
            if *queue >= self.max_queue_depth {
                *self.rejected_count.write() += 1;
                return Err(BulkheadError::RejectedFull);
            }

            *queue += 1;
            drop(queue);

            tracing::warn!(
                "Bulkhead '{}': Queued (queue depth: {})",
                self.name,
                self.queue_depth.read()
            );
            Ok(BulkheadGuard::new(
                self.current_count.clone(),
                self.queue_depth.clone(),
            ))
        } else {
            *count += 1;
            tracing::debug!(
                "Bulkhead '{}': Acquired ({}/{})",
                self.name,
                count,
                self.max_concurrent
            );

            Ok(BulkheadGuard::new(
                self.current_count.clone(),
                self.queue_depth.clone(),
            ))
        }
    }

    /// Get current state
    pub fn state(&self) -> BulkheadState {
        BulkheadState {
            name: self.name.clone(),
            current_concurrent: *self.current_count.read(),
            max_concurrent: self.max_concurrent,
            queue_depth: *self.queue_depth.read(),
            max_queue_depth: self.max_queue_depth,
            rejected_count: *self.rejected_count.read(),
        }
    }

    /// Get utilization percentage
    pub fn utilization_percent(&self) -> f64 {
        let current = *self.current_count.read();
        if self.max_concurrent == 0 {
            0.0
        } else {
            (current as f64 / self.max_concurrent as f64) * 100.0
        }
    }
}

/// Guard that releases bulkhead slot on drop
pub struct BulkheadGuard {
    current_count: Arc<RwLock<usize>>,
    queue_depth: Arc<RwLock<usize>>,
}

impl BulkheadGuard {
    fn new(current_count: Arc<RwLock<usize>>, queue_depth: Arc<RwLock<usize>>) -> Self {
        Self {
            current_count,
            queue_depth,
        }
    }
}

impl Drop for BulkheadGuard {
    fn drop(&mut self) {
        let mut queue = self.queue_depth.write();
        if *queue > 0 {
            *queue -= 1;
        } else {
            let mut count = self.current_count.write();
            if *count > 0 {
                *count -= 1;
            }
        }
    }
}

/// Bulkhead state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkheadState {
    pub name: String,
    pub current_concurrent: usize,
    pub max_concurrent: usize,
    pub queue_depth: usize,
    pub max_queue_depth: usize,
    pub rejected_count: u64,
}

/// Bulkhead errors
#[derive(Debug, thiserror::Error)]
pub enum BulkheadError {
    #[error("Bulkhead full (queue depth exceeded)")]
    RejectedFull,
    #[error("Bulkhead timeout")]
    Timeout,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bulkhead_acquire() {
        let bulkhead = Bulkhead::new("test", 2, 5);

        let guard1 = bulkhead.try_acquire();
        assert!(guard1.is_ok());

        let guard2 = bulkhead.try_acquire();
        assert!(guard2.is_ok());

        let guard3 = bulkhead.try_acquire();
        assert!(guard3.is_ok()); // Goes to queue

        let state = bulkhead.state();
        assert_eq!(state.current_concurrent, 2);
        assert_eq!(state.queue_depth, 1);
    }

    #[test]
    fn test_bulkhead_overflow() {
        let bulkhead = Bulkhead::new("test", 1, 1);

        let _guard1 = bulkhead.try_acquire();
        let _guard2 = bulkhead.try_acquire(); // Queued
        let guard3 = bulkhead.try_acquire();

        assert!(guard3.is_err());
        assert_eq!(bulkhead.state().rejected_count, 1);
    }

    #[test]
    fn test_bulkhead_utilization() {
        let bulkhead = Bulkhead::new("test", 10, 5);
        assert_eq!(bulkhead.utilization_percent(), 0.0);

        let _guard = bulkhead.try_acquire();
        assert_eq!(bulkhead.utilization_percent(), 10.0);
    }
}
