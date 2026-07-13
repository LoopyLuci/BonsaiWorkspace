use crate::TaskMetrics;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct RuntimeMetrics {
    total: Arc<Mutex<u64>>,
    completed: Arc<Mutex<u64>>,
    failed: Arc<Mutex<u64>>,
}

impl RuntimeMetrics {
    pub fn new() -> Self {
        Self {
            total: Arc::new(Mutex::new(0)),
            completed: Arc::new(Mutex::new(0)),
            failed: Arc::new(Mutex::new(0)),
        }
    }

    pub fn record_task_completed(&self) {
        *self.total.lock() += 1;
        *self.completed.lock() += 1;
    }

    pub fn record_task_failed(&self) {
        *self.total.lock() += 1;
        *self.failed.lock() += 1;
    }

    pub fn snapshot(&self) -> TaskMetrics {
        let total = *self.total.lock();
        let completed = *self.completed.lock();
        let failed = *self.failed.lock();

        TaskMetrics {
            total_tasks: total,
            completed_tasks: completed,
            failed_tasks: failed,
            avg_duration_ms: 0.0,
        }
    }
}

impl Default for RuntimeMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = RuntimeMetrics::new();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_tasks, 0);
    }

    #[test]
    fn test_record_completed() {
        let metrics = RuntimeMetrics::new();
        metrics.record_task_completed();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.completed_tasks, 1);
    }
}
