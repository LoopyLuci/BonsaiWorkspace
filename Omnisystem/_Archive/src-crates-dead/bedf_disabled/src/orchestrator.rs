use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationConfig {
    pub max_parallel_runs: usize,
    pub timeout_secs: u64,
    pub enable_fuzzing: bool,
    pub enable_concurrency: bool,
    pub enable_sanitizers: bool,
    pub enable_property: bool,
    pub enable_pentest: bool,
    pub enable_sandbox: bool,
    pub enable_triage: bool,
}

impl Default for OrchestrationConfig {
    fn default() -> Self {
        Self {
            max_parallel_runs: 8,
            timeout_secs: 3600,
            enable_fuzzing: true,
            enable_concurrency: true,
            enable_sanitizers: true,
            enable_property: true,
            enable_pentest: true,
            enable_sandbox: true,
            enable_triage: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BEDFOrchestrator {
    pub id: String,
    pub config: OrchestrationConfig,
    pub created_at: DateTime<Utc>,
    pub active_runs: usize,
}

impl BEDFOrchestrator {
    pub fn new(config: OrchestrationConfig) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            config,
            created_at: Utc::now(),
            active_runs: 0,
        }
    }

    pub async fn execute_analysis(&mut self, target: &str) -> Result<AnalysisResult> {
        if self.active_runs >= self.config.max_parallel_runs {
            anyhow::bail!("Max parallel runs reached");
        }

        self.active_runs += 1;

        let result = AnalysisResult {
            id: Uuid::new_v4().to_string(),
            target: target.to_string(),
            started_at: Utc::now(),
            engines_executed: vec![],
            total_crashes: 0,
            total_warnings: 0,
            status: AnalysisStatus::InProgress,
        };

        Ok(result)
    }

    pub fn complete_run(&mut self) {
        if self.active_runs > 0 {
            self.active_runs -= 1;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisStatus {
    InProgress,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub id: String,
    pub target: String,
    pub started_at: DateTime<Utc>,
    pub engines_executed: Vec<String>,
    pub total_crashes: u32,
    pub total_warnings: u32,
    pub status: AnalysisStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = OrchestrationConfig::default();
        assert_eq!(config.max_parallel_runs, 8);
        assert!(config.enable_fuzzing);
    }

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let config = OrchestrationConfig::default();
        let orch = BEDFOrchestrator::new(config);
        assert_eq!(orch.active_runs, 0);
        assert!(orch.id.len() > 0);
    }

    #[tokio::test]
    async fn test_execute_analysis() {
        let config = OrchestrationConfig::default();
        let mut orch = BEDFOrchestrator::new(config);
        let result = orch.execute_analysis("target").await;
        assert!(result.is_ok());
        assert_eq!(orch.active_runs, 1);
    }

    #[tokio::test]
    async fn test_max_runs_limit() {
        let mut config = OrchestrationConfig::default();
        config.max_parallel_runs = 1;
        let mut orch = BEDFOrchestrator::new(config);

        let _r1 = orch.execute_analysis("target1").await;
        let r2 = orch.execute_analysis("target2").await;
        assert!(r2.is_err());
    }
}
