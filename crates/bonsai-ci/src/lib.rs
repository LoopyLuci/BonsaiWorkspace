use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;
use tokio::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StageDef {
    /// A simple shell command (first element is executable, rest are args).
    pub cmd: Vec<String>,
    /// Optional working directory for the stage.
    pub cwd: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PipelineDef {
    pub id: Option<String>,
    pub stages: Vec<StageDef>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunResult {
    pub status: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Run a single job (command vector) and capture stdout/stderr.
pub async fn run_job(cmd: Vec<String>, cwd: Option<PathBuf>) -> Result<RunResult> {
    if cmd.is_empty() {
        return Ok(RunResult { status: "empty".to_string(), exit_code: None, stdout: String::new(), stderr: String::new() });
    }

    let mut command = Command::new(&cmd[0]);
    if cmd.len() > 1 {
        command.args(&cmd[1..]);
    }
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }

    let out = command.output().await?;

    Ok(RunResult {
        status: if out.status.success() { "ok".to_string() } else { "failed".to_string() },
        exit_code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    })
}

/// A tiny orchestrator struct for Phase 1. For now it's a light-weight holder
/// around spawn/execute helpers. In later phases this will be replaced with
/// a full actor-based supervisor.
#[derive(Debug, Default)]
pub struct OrchestratorActor {}

impl OrchestratorActor {
    pub fn new() -> Self {
        Self {}
    }

    /// Submit a pipeline and run only the first stage (Phase 1).
    pub async fn submit_pipeline(&self, pipeline: PipelineDef) -> Result<RunResult> {
        let first = match pipeline.stages.into_iter().next() {
            Some(s) => s,
            None => return Ok(RunResult { status: "no-stages".to_string(), exit_code: None, stdout: String::new(), stderr: String::new() }),
        };

        let cwd = first.cwd.map(PathBuf::from);
        run_job(first.cmd, cwd).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_echo() {
        let orch = OrchestratorActor::new();
        #[cfg(windows)]
        let cmd = vec!["cmd".to_string(), "/C".to_string(), "echo".to_string(), "hello".to_string()];
        #[cfg(not(windows))]
        let cmd = vec!["echo".to_string(), "hello".to_string()];
        let pipeline = PipelineDef { id: Some("t1".to_string()), stages: vec![StageDef { cmd, cwd: None }] };
        let r = orch.submit_pipeline(pipeline).await.unwrap();
        assert!(r.stdout.contains("hello"));
    }
}
