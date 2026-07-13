//! Job scheduling with priority queues and worker assignment.

use crate::error::{OrchestratorError, Result};
use crate::job::{Job, JobId, JobStatus};
use dashmap::DashMap;
use priority_queue::PriorityQueue;
use std::cmp::Reverse;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

/// Job scheduling queue with priority support.
pub struct JobScheduler {
    /// Priority queue: (JobId, -Priority) => higher priority = lower Reverse value
    queue: Arc<Mutex<PriorityQueue<JobId, Reverse<u8>>>>,
    /// All jobs by ID
    jobs: Arc<DashMap<JobId, Job>>,
}

impl JobScheduler {
    /// Create a new job scheduler.
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(PriorityQueue::new())),
            jobs: Arc::new(DashMap::new()),
        }
    }

    /// Enqueue a job.
    pub async fn enqueue(&self, job: Job) -> Result<()> {
        let id = job.id;
        let priority = job.priority;

        if self.jobs.contains_key(&id) {
            return Err(OrchestratorError::JobAlreadyScheduled(id));
        }

        self.jobs.insert(id, job);
        let mut queue = self.queue.lock().await;
        queue.push(id, Reverse(priority));

        info!("enqueued job {} with priority {}", id, priority);
        Ok(())
    }

    /// Dequeue the next highest-priority job.
    pub async fn dequeue(&self) -> Option<Job> {
        let mut queue = self.queue.lock().await;
        if let Some((job_id, _)) = queue.pop() {
            return self.jobs.remove(&job_id).map(|(_, job)| job);
        }
        None
    }

    /// Get a job by ID.
    pub async fn get(&self, id: JobId) -> Result<Job> {
        self.jobs
            .get(&id)
            .map(|r| r.clone())
            .ok_or(OrchestratorError::JobNotFound(id))
    }

    /// Update a job.
    pub async fn update(&self, id: JobId, job: Job) -> Result<()> {
        self.jobs
            .alter(&id, |_, _| job)
            .ok_or(OrchestratorError::JobNotFound(id))?;
        Ok(())
    }

    /// Get all queued jobs.
    pub async fn queued_jobs(&self) -> Vec<Job> {
        let queue = self.queue.lock().await;
        queue
            .iter()
            .filter_map(|(job_id, _)| {
                self.jobs.get(job_id).map(|r| r.clone())
            })
            .collect()
    }

    /// Get queue depth.
    pub async fn queue_depth(&self) -> usize {
        let queue = self.queue.lock().await;
        queue.len()
    }

    /// Get all jobs (regardless of status).
    pub async fn all_jobs(&self) -> Vec<Job> {
        self.jobs.iter().map(|r| r.value().clone()).collect()
    }

    /// Get jobs by status.
    pub async fn jobs_by_status(&self, status: JobStatus) -> Vec<Job> {
        self.jobs
            .iter()
            .filter(|r| r.value().status == status)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Count jobs by status.
    pub async fn count_by_status(&self, status: JobStatus) -> usize {
        self.jobs
            .iter()
            .filter(|r| r.value().status == status)
            .count()
    }

    /// Get scheduler statistics.
    pub async fn statistics(&self) -> SchedulerStatistics {
        let queued = self.count_by_status(JobStatus::Queued).await;
        let assigned = self.count_by_status(JobStatus::Assigned).await;
        let running = self.count_by_status(JobStatus::Running).await;
        let completed = self.count_by_status(JobStatus::Completed).await;
        let failed = self.count_by_status(JobStatus::Failed).await;
        let cancelled = self.count_by_status(JobStatus::Cancelled).await;

        SchedulerStatistics {
            queued,
            assigned,
            running,
            completed,
            failed,
            cancelled,
            total: queued + assigned + running + completed + failed + cancelled,
        }
    }

    /// Remove a job from the scheduler.
    pub async fn remove(&self, id: JobId) -> Result<Job> {
        // Also remove from queue
        let mut queue = self.queue.lock().await;
        queue.change_priority(&id, Reverse(0)); // Doesn't matter, just to mark it
        // Actually remove from queue by filtering
        let items: Vec<_> = queue.iter().map(|(id, p)| (*id, *p)).collect();
        drop(queue); // Release lock before reinserting

        let job = self
            .jobs
            .remove(&id)
            .ok_or(OrchestratorError::JobNotFound(id))?
            .1;

        debug!("removed job {} from scheduler", id);
        Ok(job)
    }

    /// Check if a job exists.
    pub async fn exists(&self, id: JobId) -> bool {
        self.jobs.contains_key(&id)
    }

    /// Clear all jobs (for testing/shutdown).
    pub async fn clear(&self) {
        self.jobs.clear();
        let mut queue = self.queue.lock().await;
        queue.clear();
    }
}

impl Default for JobScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Scheduler statistics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SchedulerStatistics {
    pub queued: usize,
    pub assigned: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub total: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enqueue_dequeue() {
        let scheduler = JobScheduler::new();
        let job = Job::new("test".to_string(), 50);
        let id = job.id;

        scheduler.enqueue(job).await.unwrap();
        let depth = scheduler.queue_depth().await;
        assert_eq!(depth, 1);

        let dequeued = scheduler.dequeue().await;
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().id, id);
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let scheduler = JobScheduler::new();

        let job_low = Job::new("low".to_string(), 10);
        let job_high = Job::new("high".to_string(), 100);

        scheduler.enqueue(job_low).await.unwrap();
        scheduler.enqueue(job_high).await.unwrap();

        // Should dequeue high-priority first
        let first = scheduler.dequeue().await.unwrap();
        assert_eq!(first.priority, 100);

        let second = scheduler.dequeue().await.unwrap();
        assert_eq!(second.priority, 10);
    }

    #[tokio::test]
    async fn test_get_job() {
        let scheduler = JobScheduler::new();
        let job = Job::new("test".to_string(), 50);
        let id = job.id;

        scheduler.enqueue(job).await.unwrap();
        let retrieved = scheduler.get(id).await.unwrap();
        assert_eq!(retrieved.id, id);
    }

    #[tokio::test]
    async fn test_statistics() {
        let scheduler = JobScheduler::new();
        let job1 = Job::new("test1".to_string(), 50);
        let job2 = Job::new("test2".to_string(), 50);

        scheduler.enqueue(job1).await.unwrap();
        scheduler.enqueue(job2).await.unwrap();

        let stats = scheduler.statistics().await;
        assert_eq!(stats.queued, 2);
        assert_eq!(stats.total, 2);
    }
}
