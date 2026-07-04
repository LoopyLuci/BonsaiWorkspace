use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PipelineStatus {
    Queued,
    Running,
    Passed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRun {
    pub run_id: Uuid,
    pub branch: String,
    pub commit: String,
    pub status: PipelineStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub stages: Vec<StageRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageRun {
    pub stage_name: String,
    pub status: PipelineStatus,
    pub duration_ms: u64,
}

pub struct CIPipeline {
    runs: Arc<DashMap<Uuid, PipelineRun>>,
}

impl CIPipeline {
    pub fn new() -> Self {
        Self {
            runs: Arc::new(DashMap::new()),
        }
    }

    pub async fn start_run(&self, branch: String, commit: String) -> Uuid {
        let run_id = Uuid::new_v4();
        let run = PipelineRun {
            run_id,
            branch,
            commit,
            status: PipelineStatus::Running,
            started_at: Utc::now(),
            ended_at: None,
            stages: vec![],
        };
        self.runs.insert(run_id, run);
        run_id
    }

    pub async fn update_run_status(&self, run_id: Uuid, status: PipelineStatus) {
        if let Some(mut run) = self.runs.get_mut(&run_id) {
            run.status = status;
            if status == PipelineStatus::Passed || status == PipelineStatus::Failed {
                run.ended_at = Some(Utc::now());
            }
        }
    }

    pub async fn get_run(&self, run_id: Uuid) -> Option<PipelineRun> {
        self.runs.get(&run_id).map(|r| r.clone())
    }
}

impl Default for CIPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_run() {
        let pipeline = CIPipeline::new();
        let run_id = pipeline.start_run("main".to_string(), "abc123".to_string()).await;
        
        let run = pipeline.get_run(run_id).await.unwrap();
        assert_eq!(run.status, PipelineStatus::Running);
        
        pipeline.update_run_status(run_id, PipelineStatus::Passed).await;
        let updated = pipeline.get_run(run_id).await.unwrap();
        assert_eq!(updated.status, PipelineStatus::Passed);
    }
}
