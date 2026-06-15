//! Job definition and status management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a job.
pub type JobId = Uuid;

/// Priority levels for job scheduling (0-255, higher = more important).
pub type Priority = u8;

/// Job execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is queued awaiting worker assignment.
    Queued,
    /// Job is assigned to a worker.
    Assigned,
    /// Job is running on the worker.
    Running,
    /// Job completed successfully.
    Completed,
    /// Job failed.
    Failed,
    /// Job was cancelled.
    Cancelled,
}

impl JobStatus {
    /// Check if this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
        )
    }

    /// Check if this state allows worker assignment.
    pub fn can_assign(&self) -> bool {
        matches!(self, JobStatus::Queued)
    }

    /// Check if this state allows running.
    pub fn can_run(&self) -> bool {
        matches!(self, JobStatus::Assigned)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Queued => "Queued",
            JobStatus::Assigned => "Assigned",
            JobStatus::Running => "Running",
            JobStatus::Completed => "Completed",
            JobStatus::Failed => "Failed",
            JobStatus::Cancelled => "Cancelled",
        }
    }
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Complete job specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job identifier.
    pub id: JobId,
    /// Human-readable test name.
    pub name: String,
    /// Priority level (0-255).
    pub priority: Priority,
    /// Current execution status.
    pub status: JobStatus,
    /// Worker assigned to this job (if any).
    pub assigned_worker: Option<String>,
    /// When this job was created.
    pub created_at: DateTime<Utc>,
    /// When this job was last updated.
    pub updated_at: DateTime<Utc>,
    /// When execution started (if applicable).
    pub started_at: Option<DateTime<Utc>>,
    /// When execution finished (if applicable).
    pub finished_at: Option<DateTime<Utc>>,
    /// Baseline name for regression comparison.
    pub baseline_name: Option<String>,
    /// Custom metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Job {
    /// Create a new job.
    pub fn new(name: String, priority: Priority) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            priority,
            status: JobStatus::Queued,
            assigned_worker: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            finished_at: None,
            baseline_name: None,
            metadata: HashMap::new(),
        }
    }

    /// Attempt to transition to a new status.
    /// Returns true if the transition is valid, false otherwise.
    pub fn transition_to(&mut self, new_status: JobStatus) -> bool {
        let valid = match (self.status, new_status) {
            (JobStatus::Queued, JobStatus::Assigned) => true,
            (JobStatus::Assigned, JobStatus::Running) => true,
            (JobStatus::Running, JobStatus::Completed) => true,
            (JobStatus::Running, JobStatus::Failed) => true,
            (JobStatus::Queued, JobStatus::Cancelled) => true,
            (JobStatus::Assigned, JobStatus::Cancelled) => true,
            _ => false,
        };

        if valid {
            self.status = new_status;
            self.updated_at = Utc::now();

            if new_status == JobStatus::Running && self.started_at.is_none() {
                self.started_at = Some(Utc::now());
            }

            if new_status.is_terminal() {
                self.finished_at = Some(Utc::now());
            }
        }

        valid
    }

    /// Assign this job to a worker.
    pub fn assign_to(&mut self, worker_id: String) -> bool {
        if self.status.can_assign() {
            self.assigned_worker = Some(worker_id);
            self.transition_to(JobStatus::Assigned)
        } else {
            false
        }
    }

    /// Get the execution duration if the job has finished.
    pub fn duration(&self) -> Option<chrono::Duration> {
        match (self.started_at, self.finished_at) {
            (Some(start), Some(finish)) => Some(finish - start),
            _ => None,
        }
    }

    /// Add metadata to the job.
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set the baseline name for this job.
    pub fn with_baseline(mut self, baseline: String) -> Self {
        self.baseline_name = Some(baseline);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_creation() {
        let job = Job::new("test_job".to_string(), 50);
        assert_eq!(job.name, "test_job");
        assert_eq!(job.priority, 50);
        assert_eq!(job.status, JobStatus::Queued);
        assert!(job.assigned_worker.is_none());
    }

    #[test]
    fn test_valid_transitions() {
        let mut job = Job::new("test".to_string(), 50);
        assert!(job.transition_to(JobStatus::Assigned));
        assert!(job.transition_to(JobStatus::Running));
        assert!(job.transition_to(JobStatus::Completed));
    }

    #[test]
    fn test_invalid_transitions() {
        let mut job = Job::new("test".to_string(), 50);
        assert!(!job.transition_to(JobStatus::Running)); // Can't run from Queued
        assert!(!job.transition_to(JobStatus::Completed)); // Can't complete from Queued
    }

    #[test]
    fn test_assign_worker() {
        let mut job = Job::new("test".to_string(), 50);
        assert!(job.assign_to("worker_1".to_string()));
        assert_eq!(job.assigned_worker, Some("worker_1".to_string()));
        assert_eq!(job.status, JobStatus::Assigned);
    }

    #[test]
    fn test_duration() {
        let mut job = Job::new("test".to_string(), 50);
        job.transition_to(JobStatus::Assigned);
        job.transition_to(JobStatus::Running);
        job.transition_to(JobStatus::Completed);
        assert!(job.duration().is_some());
    }
}
