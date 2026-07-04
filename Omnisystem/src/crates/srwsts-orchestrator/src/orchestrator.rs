//! Main orchestrator implementation with async actor pattern.

use crate::baseline::{Baseline, BaselineManager, MetricEntry};
use crate::error::{OrchestratorError, Result};
use crate::job::{Job, JobId, JobStatus};
use crate::result::{ComparisonReport, ExecutionStatus, ResultCollector, TestResult};
use crate::scheduler::JobScheduler;
use crate::telemetry::TelemetryEvent;
use crate::worker::{WorkerId, WorkerPool, WorkerStatus};
use crate::Telemetry;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error, instrument};
use uuid::Uuid;

/// Orchestrator configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Worker heartbeat timeout in seconds.
    pub heartbeat_timeout_secs: i64,
    /// Maximum concurrent jobs.
    pub max_concurrent_jobs: usize,
    /// Health check interval in seconds.
    pub health_check_interval_secs: u64,
    /// Enable automatic baseline updates.
    pub auto_update_baselines: bool,
    /// Regression severity threshold (0.0-1.0).
    pub regression_threshold: f64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            heartbeat_timeout_secs: 30,
            max_concurrent_jobs: 100,
            health_check_interval_secs: 10,
            auto_update_baselines: true,
            regression_threshold: 0.1,
        }
    }
}

/// Main orchestrator actor.
pub struct Orchestrator {
    config: Arc<RwLock<OrchestratorConfig>>,
    scheduler: Arc<JobScheduler>,
    worker_pool: Arc<WorkerPool>,
    baseline_manager: Arc<BaselineManager>,
    result_collector: Arc<ResultCollector>,
    running: Arc<RwLock<bool>>,
}

impl Orchestrator {
    /// Create a new orchestrator.
    pub async fn new(config: OrchestratorConfig) -> Result<Self> {
        info!("initializing orchestrator with config: {:?}", config);

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            scheduler: Arc::new(JobScheduler::new()),
            worker_pool: Arc::new(WorkerPool::with_timeout(config.heartbeat_timeout_secs)),
            baseline_manager: Arc::new(BaselineManager::new()),
            result_collector: Arc::new(ResultCollector::new()),
            running: Arc::new(RwLock::new(true)),
        })
    }

    /// Check if orchestrator is running.
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Register a worker.
    #[instrument(skip(self))]
    pub async fn register_worker(&self, worker_id: WorkerId) -> Result<()> {
        if !*self.running.read().await {
            return Err(OrchestratorError::Shutdown);
        }

        self.worker_pool.register(worker_id.clone())?;
        Telemetry::record_event(TelemetryEvent::WorkerRegistered {
            worker_id: worker_id.clone(),
        });
        Ok(())
    }

    /// Unregister a worker.
    #[instrument(skip(self))]
    pub async fn unregister_worker(&self, worker_id: &WorkerId) -> Result<()> {
        self.worker_pool.unregister(worker_id)?;
        Telemetry::record_event(TelemetryEvent::WorkerUnregistered {
            worker_id: worker_id.clone(),
        });
        Ok(())
    }

    /// Worker heartbeat.
    #[instrument(skip(self))]
    pub async fn worker_heartbeat(&self, worker_id: &WorkerId) -> Result<()> {
        self.worker_pool.heartbeat(worker_id)?;
        Ok(())
    }

    /// Schedule a test job.
    #[instrument(skip(self))]
    pub async fn schedule_test(
        &self,
        name: String,
        priority: u8,
        baseline_name: Option<String>,
    ) -> Result<JobId> {
        if !*self.running.read().await {
            return Err(OrchestratorError::Shutdown);
        }

        let mut job = Job::new(name, priority);
        if let Some(baseline) = baseline_name {
            job = job.with_baseline(baseline);
        }

        let job_id = job.id;
        self.scheduler.enqueue(job).await?;

        Telemetry::record_event(TelemetryEvent::JobScheduled {
            job_id: job_id.to_string(),
            priority,
        });

        Ok(job_id)
    }

    /// Assign a job to a worker.
    #[instrument(skip(self))]
    pub async fn assign_job(&self, job_id: JobId) -> Result<WorkerId> {
        if !*self.running.read().await {
            return Err(OrchestratorError::Shutdown);
        }

        // Get available worker
        let worker_id = self
            .worker_pool
            .get_available()
            .ok_or(OrchestratorError::NoAvailableWorkers)?;

        // Update job
        let mut job = self.scheduler.get(job_id).await?;
        job.assign_to(worker_id.clone());
        self.scheduler.update(job_id, job).await?;

        // Update worker status
        self.worker_pool
            .set_status(&worker_id, WorkerStatus::Busy)
            .await?;

        Telemetry::record_event(TelemetryEvent::JobAssigned {
            job_id: job_id.to_string(),
            worker_id: worker_id.clone(),
        });

        Ok(worker_id)
    }

    /// Mark a job as started.
    #[instrument(skip(self))]
    pub async fn job_started(&self, job_id: JobId) -> Result<()> {
        let mut job = self.scheduler.get(job_id).await?;
        if !job.transition_to(JobStatus::Running) {
            return Err(OrchestratorError::InvalidStatusTransition {
                from: job.status.to_string(),
                to: JobStatus::Running.to_string(),
            });
        }

        self.scheduler.update(job_id, job.clone()).await?;

        let worker_id = job.assigned_worker.unwrap_or_default();
        Telemetry::record_event(TelemetryEvent::JobStarted {
            job_id: job_id.to_string(),
            worker_id,
        });

        Ok(())
    }

    /// Collect result from a completed job.
    #[instrument(skip(self, result))]
    pub async fn collect_result(&self, result: TestResult) -> Result<()> {
        if !*self.running.read().await {
            return Err(OrchestratorError::Shutdown);
        }

        // Record result
        self.result_collector.record_result(result.clone())?;

        // Update job status
        let mut job = self.scheduler.get(result.job_id).await?;
        let new_status = if result.status.is_success() {
            JobStatus::Completed
        } else {
            JobStatus::Failed
        };

        if !job.transition_to(new_status) {
            return Err(OrchestratorError::InvalidStatusTransition {
                from: job.status.to_string(),
                to: new_status.to_string(),
            });
        }

        self.scheduler.update(result.job_id, job.clone()).await?;

        // Update worker
        if let Some(worker_id) = &job.assigned_worker {
            if result.status.is_success() {
                let duration = (result.finished_at - result.started_at).num_milliseconds() as u64;
                self.worker_pool.record_success(worker_id, duration)?;
            } else {
                self.worker_pool.record_failure(worker_id)?;
            }
            self.worker_pool
                .set_status(worker_id, WorkerStatus::Idle)
                .await?;
        }

        // Detect regression if baseline is specified
        if let Some(baseline_name) = &job.baseline_name {
            if let Ok(baseline) = self.baseline_manager.get_baseline(baseline_name) {
                let _comparison = self
                    .compare_against_baseline(&result, &baseline)
                    .await?;
            }
        }

        let duration = (result.finished_at - result.started_at).num_milliseconds() as u64;
        Telemetry::record_event(TelemetryEvent::JobCompleted {
            job_id: result.job_id.to_string(),
            duration_ms: duration,
        });

        Ok(())
    }

    /// Compare a result against a baseline.
    #[instrument(skip(self, result, baseline))]
    pub async fn compare_against_baseline(
        &self,
        result: &TestResult,
        baseline: &Baseline,
    ) -> Result<ComparisonReport> {
        let config = self.config.read().await;
        let comparison = self.result_collector.compare_against_baseline(result, baseline)?;

        for regression in &comparison.regressions {
            if regression.severity > config.regression_threshold {
                warn!(
                    "REGRESSION DETECTED: {} ({}) - {} to {}",
                    regression.metric, regression.percent_change, regression.baseline_value, regression.current_value
                );

                Telemetry::record_event(TelemetryEvent::RegressionDetected {
                    job_id: result.job_id.to_string(),
                    metric: regression.metric.clone(),
                    severity: regression.severity,
                });
            }
        }

        Ok(comparison)
    }

    /// Register or update a baseline.
    #[instrument(skip(self, baseline))]
    pub async fn register_baseline(&self, baseline: Baseline) -> Result<String> {
        let name = baseline.name.clone();
        let hash = self.baseline_manager.register_baseline(baseline).await?;

        Telemetry::record_event(TelemetryEvent::BaselineUpdated {
            baseline_name: name,
            version: 1,
        });

        Ok(hash)
    }

    /// Get a baseline.
    pub async fn get_baseline(&self, name: &str) -> Result<Baseline> {
        self.baseline_manager.get_baseline(name)
    }

    /// Update baseline metrics.
    #[instrument(skip(self, metrics))]
    pub async fn update_baseline(&self, name: &str, metrics: Vec<MetricEntry>) -> Result<()> {
        self.baseline_manager.update_baseline(name, metrics).await?;

        if let Ok(baseline) = self.baseline_manager.get_baseline(name) {
            Telemetry::record_event(TelemetryEvent::BaselineUpdated {
                baseline_name: name.to_string(),
                version: baseline.version.0,
            });
        }

        Ok(())
    }

    /// Perform health check on all workers.
    pub async fn perform_health_check(&self) -> Result<()> {
        let unhealthy = self.worker_pool.perform_health_check();

        Telemetry::record_event(TelemetryEvent::WorkerHealthCheck {
            healthy: unhealthy == 0,
            unhealthy_count: unhealthy,
        });

        Ok(())
    }

    /// Get job status.
    pub async fn get_job_status(&self, job_id: JobId) -> Result<JobStatus> {
        let job = self.scheduler.get(job_id).await?;
        Ok(job.status)
    }

    /// Get worker pool statistics.
    pub fn worker_pool_stats(&self) -> crate::worker::PoolStatistics {
        self.worker_pool.statistics()
    }

    /// Get scheduler statistics.
    pub async fn scheduler_stats(&self) -> crate::scheduler::SchedulerStatistics {
        self.scheduler.statistics().await
    }

    /// Get result collection statistics.
    pub fn result_collection_stats(&self) -> crate::result::CollectionStatistics {
        self.result_collector.statistics()
    }

    /// Shutdown the orchestrator gracefully.
    pub async fn shutdown(&self) -> Result<()> {
        info!("shutting down orchestrator");
        *self.running.write().await = false;
        self.scheduler.clear().await;
        Ok(())
    }

    /// Get list of all jobs.
    pub async fn list_jobs(&self) -> Result<Vec<Job>> {
        Ok(self.scheduler.all_jobs().await)
    }

    /// Get list of all workers.
    pub fn list_workers(&self) -> Vec<crate::worker::WorkerInfo> {
        self.worker_pool.list_all()
    }

    /// Get list of all baselines.
    pub fn list_baselines(&self) -> Vec<String> {
        self.baseline_manager.list_baselines()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let config = OrchestratorConfig::default();
        let orch = Orchestrator::new(config).await.unwrap();
        assert!(orch.is_running().await);
    }

    #[tokio::test]
    async fn test_register_worker() {
        let orch = Orchestrator::new(OrchestratorConfig::default()).await.unwrap();
        let result = orch.register_worker("w1".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_schedule_test() {
        let orch = Orchestrator::new(OrchestratorConfig::default()).await.unwrap();
        let job_id = orch
            .schedule_test("test".to_string(), 50, None)
            .await
            .unwrap();
        assert!(!job_id.to_string().is_empty());
    }

    #[tokio::test]
    async fn test_assign_job() {
        let orch = Orchestrator::new(OrchestratorConfig::default()).await.unwrap();
        orch.register_worker("w1".to_string()).await.unwrap();
        let job_id = orch
            .schedule_test("test".to_string(), 50, None)
            .await
            .unwrap();

        let worker = orch.assign_job(job_id).await.unwrap();
        assert_eq!(worker, "w1");
    }

    #[tokio::test]
    async fn test_no_available_workers() {
        let orch = Orchestrator::new(OrchestratorConfig::default()).await.unwrap();
        let job_id = orch
            .schedule_test("test".to_string(), 50, None)
            .await
            .unwrap();

        let result = orch.assign_job(job_id).await;
        assert!(matches!(
            result,
            Err(OrchestratorError::NoAvailableWorkers)
        ));
    }
}
