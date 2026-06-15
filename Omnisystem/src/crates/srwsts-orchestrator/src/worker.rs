//! Worker pool and management.

use crate::error::{OrchestratorError, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Unique identifier for a worker.
pub type WorkerId = String;

/// Worker health status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerStatus {
    /// Worker is ready for job assignment.
    Idle,
    /// Worker is executing a job.
    Busy,
    /// Worker is degraded but functional.
    Degraded,
    /// Worker is unavailable.
    Unavailable,
    /// Worker is shutting down.
    Draining,
}

impl WorkerStatus {
    pub fn is_available(&self) -> bool {
        matches!(self, WorkerStatus::Idle | WorkerStatus::Degraded)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            WorkerStatus::Idle => "Idle",
            WorkerStatus::Busy => "Busy",
            WorkerStatus::Degraded => "Degraded",
            WorkerStatus::Unavailable => "Unavailable",
            WorkerStatus::Draining => "Draining",
        }
    }
}

impl std::fmt::Display for WorkerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Worker information and health metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    /// Worker ID.
    pub id: WorkerId,
    /// Current status.
    pub status: WorkerStatus,
    /// Heartbeat timestamp.
    pub last_heartbeat: DateTime<Utc>,
    /// Number of jobs completed.
    pub jobs_completed: u64,
    /// Number of jobs failed.
    pub jobs_failed: u64,
    /// Average job duration in milliseconds.
    pub avg_job_duration_ms: u64,
    /// When this worker registered.
    pub registered_at: DateTime<Utc>,
}

impl WorkerInfo {
    /// Create a new worker info.
    pub fn new(id: WorkerId) -> Self {
        let now = Utc::now();
        Self {
            id,
            status: WorkerStatus::Idle,
            last_heartbeat: now,
            jobs_completed: 0,
            jobs_failed: 0,
            avg_job_duration_ms: 0,
            registered_at: now,
        }
    }

    /// Update heartbeat timestamp.
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Utc::now();
    }

    /// Record a successful job execution.
    pub fn record_success(&mut self, duration_ms: u64) {
        self.jobs_completed += 1;
        // Exponential moving average (alpha = 0.2)
        self.avg_job_duration_ms = (self.avg_job_duration_ms as f64 * 0.8
            + duration_ms as f64 * 0.2) as u64;
    }

    /// Record a failed job execution.
    pub fn record_failure(&mut self) {
        self.jobs_failed += 1;
    }

    /// Get success rate as a percentage.
    pub fn success_rate(&self) -> f64 {
        let total = self.jobs_completed + self.jobs_failed;
        if total == 0 {
            0.0
        } else {
            (self.jobs_completed as f64 / total as f64) * 100.0
        }
    }

    /// Check if worker is healthy based on heartbeat.
    pub fn is_healthy(&self, timeout_secs: i64) -> bool {
        let age = Utc::now() - self.last_heartbeat;
        age.num_seconds() < timeout_secs
    }
}

/// Worker pool manager.
pub struct WorkerPool {
    workers: Arc<DashMap<WorkerId, WorkerInfo>>,
    total_jobs: Arc<AtomicU64>,
    heartbeat_timeout_secs: i64,
}

impl WorkerPool {
    /// Create a new worker pool with default heartbeat timeout of 30 seconds.
    pub fn new() -> Self {
        Self {
            workers: Arc::new(DashMap::new()),
            total_jobs: Arc::new(AtomicU64::new(0)),
            heartbeat_timeout_secs: 30,
        }
    }

    /// Create with custom heartbeat timeout.
    pub fn with_timeout(heartbeat_timeout_secs: i64) -> Self {
        Self {
            workers: Arc::new(DashMap::new()),
            total_jobs: Arc::new(AtomicU64::new(0)),
            heartbeat_timeout_secs,
        }
    }

    /// Register a new worker.
    pub fn register(&self, id: WorkerId) -> Result<()> {
        if self.workers.contains_key(&id) {
            warn!("worker {} already registered", id);
            return Err(OrchestratorError::WorkerPoolError(
                "worker already registered".to_string(),
            ));
        }

        let info = WorkerInfo::new(id.clone());
        self.workers.insert(id.clone(), info);
        info!("registered worker: {}", id);
        Ok(())
    }

    /// Unregister a worker.
    pub fn unregister(&self, id: &WorkerId) -> Result<()> {
        self.workers
            .remove(id)
            .ok_or_else(|| OrchestratorError::WorkerNotFound(id.clone()))?;
        info!("unregistered worker: {}", id);
        Ok(())
    }

    /// Get worker info.
    pub fn get(&self, id: &WorkerId) -> Result<WorkerInfo> {
        self.workers
            .get(id)
            .map(|r| r.clone())
            .ok_or_else(|| OrchestratorError::WorkerNotFound(id.clone()))
    }

    /// Update worker status.
    pub fn set_status(&self, id: &WorkerId, status: WorkerStatus) -> Result<()> {
        self.workers
            .alter(id, |_, mut info| {
                info.status = status;
                info.update_heartbeat();
                info
            })
            .ok_or_else(|| OrchestratorError::WorkerNotFound(id.clone()))?;

        debug!("worker {} status: {}", id, status);
        Ok(())
    }

    /// Update heartbeat for a worker.
    pub fn heartbeat(&self, id: &WorkerId) -> Result<()> {
        self.workers
            .alter(id, |_, mut info| {
                info.update_heartbeat();
                info
            })
            .ok_or_else(|| OrchestratorError::WorkerNotFound(id.clone()))?;

        Ok(())
    }

    /// Get the first available idle worker.
    pub fn get_available(&self) -> Option<WorkerId> {
        self.workers
            .iter()
            .find(|entry| entry.value().status == WorkerStatus::Idle)
            .map(|entry| entry.key().clone())
    }

    /// Get all available workers.
    pub fn get_all_available(&self) -> Vec<WorkerId> {
        self.workers
            .iter()
            .filter(|entry| entry.value().status.is_available())
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Record a successful job completion on a worker.
    pub fn record_success(&self, id: &WorkerId, duration_ms: u64) -> Result<()> {
        self.workers
            .alter(id, |_, mut info| {
                info.record_success(duration_ms);
                info
            })
            .ok_or_else(|| OrchestratorError::WorkerNotFound(id.clone()))?;

        self.total_jobs.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Record a failed job on a worker.
    pub fn record_failure(&self, id: &WorkerId) -> Result<()> {
        self.workers
            .alter(id, |_, mut info| {
                info.record_failure();
                info
            })
            .ok_or_else(|| OrchestratorError::WorkerNotFound(id.clone()))?;

        self.total_jobs.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Get all workers.
    pub fn list_all(&self) -> Vec<WorkerInfo> {
        self.workers
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get idle workers.
    pub fn list_idle(&self) -> Vec<WorkerInfo> {
        self.workers
            .iter()
            .filter(|entry| entry.value().status == WorkerStatus::Idle)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get busy workers.
    pub fn list_busy(&self) -> Vec<WorkerInfo> {
        self.workers
            .iter()
            .filter(|entry| entry.value().status == WorkerStatus::Busy)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Health check: mark unhealthy workers as unavailable.
    pub fn perform_health_check(&self) -> usize {
        let mut unhealthy_count = 0;
        self.workers
            .iter_mut()
            .filter(|mut entry| !entry.value().is_healthy(self.heartbeat_timeout_secs))
            .for_each(|mut entry| {
                entry.value_mut().status = WorkerStatus::Unavailable;
                unhealthy_count += 1;
            });

        if unhealthy_count > 0 {
            warn!("marked {} workers as unhealthy", unhealthy_count);
        }

        unhealthy_count
    }

    /// Get overall worker pool statistics.
    pub fn statistics(&self) -> PoolStatistics {
        let workers: Vec<_> = self.workers.iter().map(|entry| entry.value().clone()).collect();

        let total = workers.len();
        let idle = workers.iter().filter(|w| w.status == WorkerStatus::Idle).count();
        let busy = workers.iter().filter(|w| w.status == WorkerStatus::Busy).count();
        let unavailable = workers
            .iter()
            .filter(|w| w.status == WorkerStatus::Unavailable)
            .count();

        let total_completed: u64 = workers.iter().map(|w| w.jobs_completed).sum();
        let total_failed: u64 = workers.iter().map(|w| w.jobs_failed).sum();
        let avg_success_rate = if workers.is_empty() {
            0.0
        } else {
            workers.iter().map(|w| w.success_rate()).sum::<f64>() / workers.len() as f64
        };

        PoolStatistics {
            total_workers: total,
            idle_workers: idle,
            busy_workers: busy,
            unavailable_workers: unavailable,
            total_jobs_completed: total_completed,
            total_jobs_failed: total_failed,
            average_success_rate: avg_success_rate,
        }
    }
}

impl Default for WorkerPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Worker pool statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatistics {
    pub total_workers: usize,
    pub idle_workers: usize,
    pub busy_workers: usize,
    pub unavailable_workers: usize,
    pub total_jobs_completed: u64,
    pub total_jobs_failed: u64,
    pub average_success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_info_creation() {
        let info = WorkerInfo::new("worker_1".to_string());
        assert_eq!(info.id, "worker_1");
        assert_eq!(info.status, WorkerStatus::Idle);
        assert_eq!(info.jobs_completed, 0);
        assert_eq!(info.jobs_failed, 0);
    }

    #[test]
    fn test_worker_pool_register() {
        let pool = WorkerPool::new();
        assert!(pool.register("w1".to_string()).is_ok());
        assert!(pool.get(&"w1".to_string()).is_ok());
        assert!(pool.register("w1".to_string()).is_err());
    }

    #[test]
    fn test_worker_pool_status() {
        let pool = WorkerPool::new();
        pool.register("w1".to_string()).unwrap();
        pool.set_status(&"w1".to_string(), WorkerStatus::Busy)
            .unwrap();
        let info = pool.get(&"w1".to_string()).unwrap();
        assert_eq!(info.status, WorkerStatus::Busy);
    }

    #[test]
    fn test_worker_pool_available() {
        let pool = WorkerPool::new();
        pool.register("w1".to_string()).unwrap();
        pool.register("w2".to_string()).unwrap();
        pool.set_status(&"w1".to_string(), WorkerStatus::Busy).unwrap();

        let available = pool.get_available();
        assert_eq!(available, Some("w2".to_string()));
    }

    #[test]
    fn test_success_rate() {
        let mut info = WorkerInfo::new("w1".to_string());
        info.jobs_completed = 80;
        info.jobs_failed = 20;
        assert_eq!(info.success_rate(), 80.0);
    }
}
