//! Agda proof-assistant sidecar.
//!
//! Drives a locally-installed `agda` binary. No network calls are made.
//!
//! # Protocol
//! 1. Write source to a temporary `.agda` file.
//! 2. Spawn `agda --no-libraries <file>`.
//! 3. Parse exit code + stdout for error patterns.

use std::io::Write as _;
use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgdaError {
    #[error("agda not found — install Agda and ensure `agda` is on PATH")]
    BinaryNotFound,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Agda reported errors:\n{0}")]
    VerificationFailed(String),
    #[error("{0}")]
    Other(String),
}

pub type AgdaResult<T> = Result<T, AgdaError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgdaRequest {
    /// Agda source code (`.agda` content). Must include a `module` declaration.
    pub source: String,
    /// Module name (must match the `module` declaration in source).
    pub module_name: String,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgdaResponse {
    pub success: bool,
    pub diagnostics: Vec<AgdaDiagnostic>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgdaDiagnostic {
    pub severity: String,
    pub message: String,
    pub location: Option<String>,
}

pub struct AgdaSidecar {
    agda_path: Option<std::path::PathBuf>,
}

impl Default for AgdaSidecar {
    fn default() -> Self { Self::new() }
}

impl AgdaSidecar {
    pub fn new() -> Self {
        Self { agda_path: Self::find_agda() }
    }

    pub fn with_path(path: impl Into<std::path::PathBuf>) -> Self {
        Self { agda_path: Some(path.into()) }
    }

    pub fn agda_available() -> bool { Self::find_agda().is_some() }

    fn find_agda() -> Option<std::path::PathBuf> {
        let names = if cfg!(windows) { vec!["agda.exe", "agda"] } else { vec!["agda"] };
        for name in names {
            if let Some(p) = which(name) { return Some(p); }
        }
        None
    }

    pub fn verify(&self, req: &AgdaRequest) -> AgdaResult<AgdaResponse> {
        let agda = self.agda_path.as_ref().ok_or(AgdaError::BinaryNotFound)?;

        let dir = tempfile_dir()?;
        // Agda requires the file name to match the module name.
        let src_path = dir.join(format!("{}.agda", req.module_name));
        {
            let mut f = std::fs::File::create(&src_path)?;
            f.write_all(req.source.as_bytes())?;
        }

        let mut cmd = Command::new(agda);
        cmd.arg("--no-libraries").arg(&src_path)
            .stdout(Stdio::piped()).stderr(Stdio::piped());

        let timeout = req.timeout_secs;
        let out = if let Some(secs) = timeout {
            run_with_timeout(cmd, secs).map_err(AgdaError::Other)?
        } else {
            cmd.output()?
        };

        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        let combined = format!("{stdout}\n{stderr}");
        let success = out.status.success()
            && !combined.contains("error")
            && !combined.contains("Error");

        let diagnostics = parse_agda_diagnostics(&combined);

        if !success {
            return Err(AgdaError::VerificationFailed(combined));
        }

        Ok(AgdaResponse { success, diagnostics, stdout, stderr })
    }
}

fn parse_agda_diagnostics(output: &str) -> Vec<AgdaDiagnostic> {
    output.lines()
        .filter(|l| l.contains("error") || l.contains("Error") || l.contains("warning"))
        .map(|l| {
            let severity = if l.to_lowercase().contains("error") { "error" } else { "warning" };
            AgdaDiagnostic { severity: severity.into(), message: l.to_string(), location: None }
        })
        .collect()
}

fn tempfile_dir() -> AgdaResult<std::path::PathBuf> {
    let dir = std::env::temp_dir().join("bonsai-agda");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn which(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths)
                .map(|p| p.join(name))
                .find(|p| p.exists())
        })
        .flatten()
}

fn run_with_timeout(mut cmd: Command, secs: u64) -> Result<std::process::Output, String> {
    use std::time::{Duration, Instant};
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        if Instant::now() >= deadline {
            let _ = child.kill();
            return Err(format!("agda timed out after {secs}s"));
        }
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(_) => break,
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    }
    child.wait_with_output().map_err(|e| e.to_string())
}
