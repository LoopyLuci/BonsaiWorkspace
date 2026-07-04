/// Mesh Worker – Executes jobs in distributed mesh
use ubvm_core::{TestJob, TestResult};

#[derive(Debug, Clone)]
pub struct WorkerState {
    pub id: String,
    pub capabilities: Vec<String>,
    pub healthy: bool,
    pub assigned_jobs: u32,
}

pub struct WorkerNode {
    state: WorkerState,
}

impl WorkerNode {
    pub fn new(id: &str, capabilities: Vec<String>) -> Self {
        Self {
            state: WorkerState {
                id: id.to_string(),
                capabilities,
                healthy: true,
                assigned_jobs: 0,
            },
        }
    }

    pub fn id(&self) -> &str {
        &self.state.id
    }

    pub fn is_healthy(&self) -> bool {
        self.state.healthy
    }

    pub fn can_execute(&self, job: &TestJob) -> bool {
        if !self.state.healthy {
            return false;
        }
        job.language.as_ref().map_or(true, |lang| {
            self.state.capabilities.iter().any(|cap| cap == lang)
        })
    }

    /// Execute a job on this worker
    pub async fn execute(&mut self, job: &TestJob) -> anyhow::Result<TestResult> {
        if !self.can_execute(job) {
            return Ok(TestResult::error(
                job.id,
                "Worker cannot execute this job".into(),
            ));
        }

        self.state.assigned_jobs += 1;

        // Simulate execution (in production: send to actual worker node)
        let result = TestResult {
            id: job.id,
            passed: true,
            fidelity: 1.0,
            duration_ms: 50,
            output: format!("Executed by worker {}", self.state.id),
            error: None,
        };

        Ok(result)
    }

    pub fn jobs_assigned(&self) -> u32 {
        self.state.assigned_jobs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_worker_creation() {
        let worker = WorkerNode::new("worker-1", vec!["rust".into(), "python".into()]);
        assert_eq!(worker.id(), "worker-1");
        assert!(worker.is_healthy());
    }

    #[tokio::test]
    async fn test_can_execute() {
        let worker = WorkerNode::new("worker-1", vec!["rust".into()]);
        let job = TestJob {
            id: ubvm_core::TestId::new(),
            suite: "language".into(),
            case: "test".into(),
            input: serde_json::json!({}),
            expected: serde_json::json!({}),
            language: Some("rust".into()),
            timeout: Duration::from_secs(30),
        };
        assert!(worker.can_execute(&job));
    }

    #[tokio::test]
    async fn test_execute_job() {
        let mut worker = WorkerNode::new("worker-1", vec!["rust".into()]);
        let job = TestJob {
            id: ubvm_core::TestId::new(),
            suite: "language".into(),
            case: "test".into(),
            input: serde_json::json!({}),
            expected: serde_json::json!({}),
            language: Some("rust".into()),
            timeout: Duration::from_secs(30),
        };
        let result = worker.execute(&job).await.unwrap();
        assert!(result.passed);
        assert_eq!(worker.jobs_assigned(), 1);
    }
}
