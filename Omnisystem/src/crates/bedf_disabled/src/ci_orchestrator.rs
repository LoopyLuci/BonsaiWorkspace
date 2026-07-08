use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use tokio::sync::DashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CIPipeline {
    pub id: String,
    pub name: String,
    pub teams: Vec<TeamJob>,
    pub status: PipelineStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamJob {
    pub team_id: String,
    pub team_name: String,
    pub crate_name: String,
    pub status: JobStatus,
    pub output: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PipelineStatus {
    Pending,
    Running,
    Success,
    Failure,
    Partial,
}

pub struct BonsaiCIOrchestrator {
    pipelines: DashMap<String, CIPipeline>,
    config: CIOrchestratorConfig,
}

#[derive(Debug, Clone)]
pub struct CIOrchestratorConfig {
    pub parallel_jobs: usize,
    pub timeout_secs: u64,
    pub retry_on_failure: bool,
    pub max_retries: u32,
}

impl Default for CIOrchestratorConfig {
    fn default() -> Self {
        Self {
            parallel_jobs: 8,
            timeout_secs: 3600,
            retry_on_failure: true,
            max_retries: 2,
        }
    }
}

impl BonsaiCIOrchestrator {
    pub fn new(config: CIOrchestratorConfig) -> Self {
        Self {
            pipelines: DashMap::new(),
            config,
        }
    }

    pub async fn create_pipeline(&self, name: &str, teams: Vec<TeamJob>) -> String {
        let pipeline_id = Uuid::new_v4().to_string();
        let pipeline = CIPipeline {
            id: pipeline_id.clone(),
            name: name.to_string(),
            teams,
            status: PipelineStatus::Pending,
            created_at: chrono::Utc::now(),
        };

        self.pipelines.insert(pipeline_id.clone(), pipeline);
        tracing::info!("Created pipeline: {} ({})", name, pipeline_id);
        pipeline_id
    }

    pub async fn run_pipeline(&self, pipeline_id: &str) -> Result<PipelineStatus, String> {
        let mut pipeline = self
            .pipelines
            .get_mut(pipeline_id)
            .ok_or("Pipeline not found".to_string())?;

        pipeline.status = PipelineStatus::Running;
        drop(pipeline);

        let mut failed_teams = Vec::new();
        let teams_count = {
            let p = self.pipelines.get(pipeline_id).unwrap();
            p.teams.len()
        };

        // Run teams in parallel batches
        for batch_start in (0..teams_count).step_by(self.config.parallel_jobs) {
            let batch_end = (batch_start + self.config.parallel_jobs).min(teams_count);

            let mut handles = Vec::new();
            for team_idx in batch_start..batch_end {
                let pipeline_id = pipeline_id.to_string();
                let team_name = {
                    let p = self.pipelines.get(&pipeline_id).unwrap();
                    p.teams[team_idx].team_name.clone()
                };
                let crate_name = {
                    let p = self.pipelines.get(&pipeline_id).unwrap();
                    p.teams[team_idx].crate_name.clone()
                };

                let handle = tokio::spawn(async move {
                    self.run_team_job(&pipeline_id, team_idx, &team_name, &crate_name)
                        .await
                });
                handles.push(handle);
            }

            for handle in handles {
                match handle.await {
                    Ok(Err(e)) => {
                        tracing::error!("Team job failed: {}", e);
                        failed_teams.push(e);
                    }
                    Err(e) => {
                        tracing::error!("Task error: {}", e);
                    }
                    _ => {}
                }
            }
        }

        let final_status = if failed_teams.is_empty() {
            PipelineStatus::Success
        } else {
            tracing::warn!("Pipeline {} failed: {} teams", pipeline_id, failed_teams.len());
            PipelineStatus::Failure
        };

        if let Some(mut pipeline) = self.pipelines.get_mut(pipeline_id) {
            pipeline.status = final_status;
        }

        Ok(final_status)
    }

    async fn run_team_job(
        &self,
        pipeline_id: &str,
        team_idx: usize,
        team_name: &str,
        crate_name: &str,
    ) -> Result<(), String> {
        tracing::info!("Running team job: {} ({})", team_name, crate_name);

        let output = Command::new("cargo")
            .args(&["test", "--package", crate_name, "--release"])
            .output()
            .map_err(|e| format!("Failed to run cargo: {}", e))?;

        let status_ok = output.status.success();

        if let Some(mut pipeline) = self.pipelines.get_mut(pipeline_id) {
            if team_idx < pipeline.teams.len() {
                pipeline.teams[team_idx].status = if status_ok {
                    JobStatus::Passed
                } else {
                    JobStatus::Failed
                };
                pipeline.teams[team_idx].output = String::from_utf8_lossy(&output.stdout).to_string();
            }
        }

        if !status_ok {
            return Err(format!("Team {} failed", team_name));
        }

        tracing::info!("✅ Team {} passed", team_name);
        Ok(())
    }

    pub fn get_pipeline(&self, pipeline_id: &str) -> Option<CIPipeline> {
        self.pipelines.get(pipeline_id).map(|p| p.clone())
    }

    pub fn list_pipelines(&self) -> Vec<CIPipeline> {
        self.pipelines.iter().map(|entry| entry.value().clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let config = CIOrchestratorConfig::default();
        let orch = BonsaiCIOrchestrator::new(config);
        assert_eq!(orch.config.parallel_jobs, 8);
    }

    #[tokio::test]
    async fn test_create_pipeline() {
        let config = CIOrchestratorConfig::default();
        let orch = BonsaiCIOrchestrator::new(config);
        let teams = vec![TeamJob {
            team_id: "A".to_string(),
            team_name: "Fuzzing".to_string(),
            crate_name: "bonsai-bedf-fuzzing".to_string(),
            status: JobStatus::Pending,
            output: String::new(),
        }];

        let pipeline_id = orch.create_pipeline("test_pipeline", teams).await;
        assert!(!pipeline_id.is_empty());
    }

    #[tokio::test]
    async fn test_get_pipeline() {
        let config = CIOrchestratorConfig::default();
        let orch = BonsaiCIOrchestrator::new(config);
        let teams = vec![];
        let pipeline_id = orch.create_pipeline("test", teams).await;
        let pipeline = orch.get_pipeline(&pipeline_id);
        assert!(pipeline.is_some());
    }
}
