//! Performance metrics collection

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Performance metrics for applications
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    compilations: Arc<AtomicU64>,
    total_compilation_ms: Arc<AtomicU64>,
    tasks_completed: Arc<AtomicU64>,
    total_task_ms: Arc<AtomicU64>,
    files_processed: Arc<AtomicU64>,
}

impl PerformanceMetrics {
    /// Create new performance metrics
    pub fn new() -> Self {
        Self {
            compilations: Arc::new(AtomicU64::new(0)),
            total_compilation_ms: Arc::new(AtomicU64::new(0)),
            tasks_completed: Arc::new(AtomicU64::new(0)),
            total_task_ms: Arc::new(AtomicU64::new(0)),
            files_processed: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a compilation
    pub fn record_compilation(&self, duration: Duration) {
        self.compilations.fetch_add(1, Ordering::Relaxed);
        self.total_compilation_ms
            .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
    }

    /// Record a task completion
    pub fn record_task_completion(&self, duration: Duration) {
        self.tasks_completed.fetch_add(1, Ordering::Relaxed);
        self.total_task_ms
            .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
    }

    /// Record file processing
    pub fn record_file_processed(&self) {
        self.files_processed.fetch_add(1, Ordering::Relaxed);
    }

    /// Get compilation count
    pub fn compilation_count(&self) -> u64 {
        self.compilations.load(Ordering::Acquire)
    }

    /// Get average compilation time
    pub fn average_compilation_ms(&self) -> f64 {
        let count = self.compilations.load(Ordering::Acquire);
        if count == 0 {
            return 0.0;
        }

        let total = self.total_compilation_ms.load(Ordering::Acquire);
        total as f64 / count as f64
    }

    /// Get tasks completed
    pub fn tasks_completed(&self) -> u64 {
        self.tasks_completed.load(Ordering::Acquire)
    }

    /// Get average task time
    pub fn average_task_ms(&self) -> f64 {
        let count = self.tasks_completed.load(Ordering::Acquire);
        if count == 0 {
            return 0.0;
        }

        let total = self.total_task_ms.load(Ordering::Acquire);
        total as f64 / count as f64
    }

    /// Get files processed
    pub fn files_processed(&self) -> u64 {
        self.files_processed.load(Ordering::Acquire)
    }

    /// Get throughput (compilations per minute)
    pub fn compilation_throughput(&self) -> f64 {
        let count = self.compilation_count();
        (count as f64 * 60000.0) / 1000.0
    }

    /// Generate summary
    pub fn summary(&self) -> super::PerformanceSummary {
        super::PerformanceSummary {
            compilations: self.compilation_count(),
            avg_compilation_ms: self.average_compilation_ms(),
            tasks_completed: self.tasks_completed(),
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics_creation() {
        let metrics = PerformanceMetrics::new();
        assert_eq!(metrics.compilation_count(), 0);
        assert_eq!(metrics.tasks_completed(), 0);
    }

    #[test]
    fn test_compilation_recording() {
        let metrics = PerformanceMetrics::new();
        metrics.record_compilation(Duration::from_millis(100));
        metrics.record_compilation(Duration::from_millis(200));

        assert_eq!(metrics.compilation_count(), 2);
        assert_eq!(metrics.average_compilation_ms(), 150.0);
    }

    #[test]
    fn test_task_completion_recording() {
        let metrics = PerformanceMetrics::new();
        metrics.record_task_completion(Duration::from_millis(500));
        metrics.record_task_completion(Duration::from_millis(1500));

        assert_eq!(metrics.tasks_completed(), 2);
        assert_eq!(metrics.average_task_ms(), 1000.0);
    }

    #[test]
    fn test_file_processing() {
        let metrics = PerformanceMetrics::new();
        for _ in 0..100 {
            metrics.record_file_processed();
        }

        assert_eq!(metrics.files_processed(), 100);
    }

    #[test]
    fn test_average_with_zero_count() {
        let metrics = PerformanceMetrics::new();
        assert_eq!(metrics.average_compilation_ms(), 0.0);
        assert_eq!(metrics.average_task_ms(), 0.0);
    }
}
