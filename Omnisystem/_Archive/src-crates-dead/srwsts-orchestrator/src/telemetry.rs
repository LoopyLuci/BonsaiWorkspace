//! Telemetry and observability infrastructure.

use chrono::Utc;
use metrics::{counter, histogram, gauge};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{info, warn, error};

/// Telemetry event types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TelemetryEvent {
    JobScheduled { job_id: String, priority: u8 },
    JobAssigned { job_id: String, worker_id: String },
    JobStarted { job_id: String, worker_id: String },
    JobCompleted { job_id: String, duration_ms: u64 },
    JobFailed { job_id: String, error: String },
    RegressionDetected { job_id: String, metric: String, severity: f64 },
    WorkerRegistered { worker_id: String },
    WorkerUnregistered { worker_id: String },
    WorkerHealthCheck { healthy: bool, unhealthy_count: usize },
    BaselineUpdated { baseline_name: String, version: u64 },
}

/// Telemetry system for observability.
pub struct Telemetry;

impl Telemetry {
    /// Record a telemetry event.
    pub fn record_event(event: TelemetryEvent) {
        match event {
            TelemetryEvent::JobScheduled { job_id, priority } => {
                counter!("orchestrator.jobs.scheduled").increment(1);
                info!("job scheduled: {} (priority: {})", job_id, priority);
            }
            TelemetryEvent::JobAssigned { job_id, worker_id } => {
                counter!("orchestrator.jobs.assigned").increment(1);
                info!("job assigned: {} -> {}", job_id, worker_id);
            }
            TelemetryEvent::JobStarted { job_id, worker_id } => {
                counter!("orchestrator.jobs.started").increment(1);
                info!("job started: {} on {}", job_id, worker_id);
            }
            TelemetryEvent::JobCompleted { job_id, duration_ms } => {
                counter!("orchestrator.jobs.completed").increment(1);
                histogram!("orchestrator.job.duration_ms").record(duration_ms as f64);
                info!("job completed: {} ({}ms)", job_id, duration_ms);
            }
            TelemetryEvent::JobFailed { job_id, error } => {
                counter!("orchestrator.jobs.failed").increment(1);
                error!("job failed: {} ({})", job_id, error);
            }
            TelemetryEvent::RegressionDetected {
                job_id,
                metric,
                severity,
            } => {
                counter!("orchestrator.regressions").increment(1);
                histogram!("orchestrator.regression.severity").record(severity);
                warn!("regression detected: {} in {} (severity: {})", job_id, metric, severity);
            }
            TelemetryEvent::WorkerRegistered { worker_id } => {
                counter!("orchestrator.workers.registered").increment(1);
                info!("worker registered: {}", worker_id);
            }
            TelemetryEvent::WorkerUnregistered { worker_id } => {
                counter!("orchestrator.workers.unregistered").increment(1);
                info!("worker unregistered: {}", worker_id);
            }
            TelemetryEvent::WorkerHealthCheck { healthy, unhealthy_count } => {
                if unhealthy_count > 0 {
                    warn!("health check: {} unhealthy workers", unhealthy_count);
                    counter!("orchestrator.workers.unhealthy").add(unhealthy_count as u64);
                }
            }
            TelemetryEvent::BaselineUpdated { baseline_name, version } => {
                counter!("orchestrator.baselines.updated").increment(1);
                info!("baseline updated: {} (v{})", baseline_name, version);
            }
        }
    }

    /// Start a timer for operation tracking.
    pub fn start_timer() -> OperationTimer {
        OperationTimer {
            start: Instant::now(),
        }
    }

    /// Record a gauge value.
    pub fn record_gauge(metric: &str, value: f64) {
        gauge!(metric).set(value);
    }

    /// Record a counter increment.
    pub fn increment_counter(metric: &str, count: u64) {
        counter!(metric).add(count);
    }

    /// Record a histogram value.
    pub fn record_histogram(metric: &str, value: f64) {
        histogram!(metric).record(value);
    }
}

/// Operation timer for measuring durations.
pub struct OperationTimer {
    start: Instant,
}

impl OperationTimer {
    /// Finish timing and return duration in milliseconds.
    pub fn finish(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    /// Record the timing to a histogram metric.
    pub fn record_to(&self, metric: &str) {
        histogram!(metric).record(self.finish() as f64);
    }
}

/// Performance metrics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub timestamp: chrono::DateTime<Utc>,
    pub jobs_scheduled: u64,
    pub jobs_completed: u64,
    pub jobs_failed: u64,
    pub avg_job_duration_ms: u64,
    pub regressions_detected: u64,
    pub workers_registered: u64,
    pub workers_unregistered: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_timer() {
        let timer = Telemetry::start_timer();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let duration = timer.finish();
        assert!(duration >= 10);
    }

    #[test]
    fn test_telemetry_event_creation() {
        let event = TelemetryEvent::JobScheduled {
            job_id: "job1".to_string(),
            priority: 50,
        };

        match event {
            TelemetryEvent::JobScheduled { job_id, priority } => {
                assert_eq!(job_id, "job1");
                assert_eq!(priority, 50);
            }
            _ => panic!("wrong event type"),
        }
    }
}
