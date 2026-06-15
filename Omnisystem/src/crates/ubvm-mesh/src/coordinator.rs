/// Mesh Coordinator – Orchestrates distributed test execution
use ubvm_core::{TestJob, TestResult};
use crate::worker::WorkerNode;
use dashmap::DashMap;

pub struct MeshCoordinator {
    workers: DashMap<String, WorkerNode>,
    results: DashMap<ubvm_core::TestId, TestResult>,
}

impl MeshCoordinator {
    pub fn new() -> Self {
        Self {
            workers: DashMap::new(),
            results: DashMap::new(),
        }
    }

    pub async fn register_worker(&self, id: &str, capabilities: Vec<String>) -> anyhow::Result<()> {
        let worker = WorkerNode::new(id, capabilities);
        self.workers.insert(id.to_string(), worker);
        Ok(())
    }

    pub async fn execute_job(&self, job: &TestJob) -> anyhow::Result<TestResult> {
        // Find a suitable worker
        let mut best_worker = None;
        let mut min_jobs = u32::MAX;

        for entry in self.workers.iter() {
            let worker = entry.value();
            if worker.can_execute(job) && worker.jobs_assigned() < min_jobs {
                best_worker = Some(entry.key().clone());
                min_jobs = worker.jobs_assigned();
            }
        }

        match best_worker {
            Some(worker_id) => {
                // Execute on worker
                let mut worker = self
                    .workers
                    .get_mut(&worker_id)
                    .ok_or_else(|| anyhow::anyhow!("Worker not found"))?;
                let result = worker.execute(job).await?;
                drop(worker);

                self.results.insert(job.id, result.clone());
                Ok(result)
            }
            None => Ok(TestResult::error(
                job.id,
                "No suitable worker found".into(),
            )),
        }
    }

    pub fn get_result(&self, id: ubvm_core::TestId) -> Option<TestResult> {
        self.results.get(&id).map(|r| r.clone())
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    pub fn result_count(&self) -> usize {
        self.results.len()
    }
}

impl Default for MeshCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_coordinator_creation() {
        let coord = MeshCoordinator::new();
        assert_eq!(coord.worker_count(), 0);
    }

    #[tokio::test]
    async fn test_register_worker() {
        let coord = MeshCoordinator::new();
        coord
            .register_worker("worker-1", vec!["rust".into()])
            .await
            .unwrap();
        assert_eq!(coord.worker_count(), 1);
    }

    #[tokio::test]
    async fn test_execute_job() {
        let coord = MeshCoordinator::new();
        coord
            .register_worker("worker-1", vec!["rust".into()])
            .await
            .unwrap();

        let job = TestJob {
            id: ubvm_core::TestId::new(),
            suite: "language".into(),
            case: "test".into(),
            input: serde_json::json!({}),
            expected: serde_json::json!({}),
            language: Some("rust".into()),
            timeout: Duration::from_secs(30),
        };

        let result = coord.execute_job(&job).await.unwrap();
        assert!(result.passed);
        assert_eq!(coord.result_count(), 1);
    }
}
