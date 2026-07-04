//! Batch Job Queue with AriaDB-like durability
//!
//! Manages job lifecycle: queued → running → completed
//! All state is persisted to allow resumability after crashes.

use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchJob {
    pub id: String,
    pub src_lang: String,
    pub tgt_lang: String,
    pub seed: u64,
    pub status: JobStatus,
    pub fidelity: Option<f32>,
    pub exec_time_ms: Option<u64>,
    pub error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct BatchQueue {
    jobs: Arc<DashMap<String, BatchJob>>,
    job_counter: Arc<AtomicU64>,
    data_dir: PathBuf,
}

impl BatchQueue {
    pub async fn new(data_dir: PathBuf) -> Result<Self> {
        tokio::fs::create_dir_all(&data_dir).await.ok();

        let queue = Self {
            jobs: Arc::new(DashMap::new()),
            job_counter: Arc::new(AtomicU64::new(0)),
            data_dir,
        };

        // Load existing jobs from disk
        queue.load_from_disk().await?;
        Ok(queue)
    }

    /// Create all jobs for an N×N matrix
    pub async fn create_matrix(
        &self,
        languages: &[String],
        seed: u64,
    ) -> Result<usize> {
        let mut count = 0;
        for src in languages {
            for tgt in languages {
                let job_id = format!(
                    "job-{}-{}-to-{}",
                    Uuid::new_v4().simple(),
                    src.to_lowercase(),
                    tgt.to_lowercase()
                );

                let job = BatchJob {
                    id: job_id.clone(),
                    src_lang: src.clone(),
                    tgt_lang: tgt.clone(),
                    seed,
                    status: JobStatus::Queued,
                    fidelity: None,
                    exec_time_ms: None,
                    error: None,
                    created_at: chrono::Utc::now(),
                    started_at: None,
                    completed_at: None,
                };

                self.jobs.insert(job_id, job);
                count += 1;
            }
        }

        self.save_to_disk().await?;
        Ok(count)
    }

    /// Fetch a batch of queued jobs
    pub async fn fetch_batch(&self, batch_size: usize) -> Vec<BatchJob> {
        let mut batch = Vec::new();
        for entry in self.jobs.iter() {
            if entry.value().status == JobStatus::Queued && batch.len() < batch_size {
                batch.push(entry.value().clone());
            }
        }
        batch
    }

    /// Mark a job as running
    pub async fn mark_running(&self, job_id: &str) -> Result<()> {
        if let Some(mut entry) = self.jobs.get_mut(job_id) {
            entry.status = JobStatus::Running;
            entry.started_at = Some(chrono::Utc::now());
        }
        self.save_to_disk().await?;
        Ok(())
    }

    /// Mark a job as completed with fidelity
    pub async fn mark_completed(
        &self,
        job_id: &str,
        fidelity: f32,
        exec_time_ms: u64,
    ) -> Result<()> {
        if let Some(mut entry) = self.jobs.get_mut(job_id) {
            entry.status = JobStatus::Completed;
            entry.fidelity = Some(fidelity);
            entry.exec_time_ms = Some(exec_time_ms);
            entry.completed_at = Some(chrono::Utc::now());
        }
        self.save_to_disk().await?;
        Ok(())
    }

    /// Mark a job as failed
    pub async fn mark_failed(&self, job_id: &str, error: String) -> Result<()> {
        if let Some(mut entry) = self.jobs.get_mut(job_id) {
            entry.status = JobStatus::Failed;
            entry.error = Some(error);
            entry.completed_at = Some(chrono::Utc::now());
        }
        self.save_to_disk().await?;
        Ok(())
    }

    /// Get queue statistics
    pub fn stats(&self) -> QueueStats {
        let mut queued = 0;
        let mut running = 0;
        let mut completed = 0;
        let mut failed = 0;
        let mut total_fidelity = 0.0f32;
        let mut fidelity_count = 0;

        for entry in self.jobs.iter() {
            match entry.value().status {
                JobStatus::Queued => queued += 1,
                JobStatus::Running => running += 1,
                JobStatus::Completed => {
                    completed += 1;
                    if let Some(f) = entry.value().fidelity {
                        total_fidelity += f;
                        fidelity_count += 1;
                    }
                }
                JobStatus::Failed => failed += 1,
            }
        }

        QueueStats {
            total: self.jobs.len(),
            queued,
            running,
            completed,
            failed,
            avg_fidelity: if fidelity_count > 0 {
                total_fidelity / fidelity_count as f32
            } else {
                0.0
            },
        }
    }

    /// Save all jobs to disk
    async fn save_to_disk(&self) -> Result<()> {
        let jobs: Vec<_> = self.jobs.iter().map(|e| e.value().clone()).collect();
        let data = serde_json::to_string(&jobs)?;
        let path = self.data_dir.join("jobs.json");
        tokio::fs::write(path, data).await?;
        Ok(())
    }

    /// Load all jobs from disk
    async fn load_from_disk(&self) -> Result<()> {
        let path = self.data_dir.join("jobs.json");
        if !path.exists() {
            return Ok(());
        }
        let data = tokio::fs::read_to_string(path).await?;
        let jobs: Vec<BatchJob> = serde_json::from_str(&data)?;
        for job in jobs {
            self.jobs.insert(job.id.clone(), job);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct QueueStats {
    pub total: usize,
    pub queued: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub avg_fidelity: f32,
}
