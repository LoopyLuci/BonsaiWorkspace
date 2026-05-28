//! Coq proof-assistant sidecar.
//!
//! Drives a locally-installed `coqc` binary. No network calls are made.
//!
//! # Protocol
//! 1. Write source to a temporary `.v` file.
//! 2. Spawn `coqc <file>`.
//! 3. Parse exit code + stderr for "Error:" / "Proof completed" patterns.

use std::io::Write as _;
use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoqError {
    #[error("coqc not found — install Coq and ensure `coqc` is on PATH")]
    BinaryNotFound,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Coq reported errors:\n{0}")]
    VerificationFailed(String),
    #[error("{0}")]
    Other(String),
}

pub type CoqResult<T> = Result<T, CoqError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoqRequest {
    /// Coq source code (`.v` content).
    pub source: String,
    /// Logical name prefix for `coqc -Q` (optional).
    pub logical_name: Option<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoqResponse {
    pub success: bool,
    pub diagnostics: Vec<CoqDiagnostic>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoqDiagnostic {
    pub severity: String,
    pub message: String,
}

pub struct CoqSidecar {
    coqc_path: Option<std::path::PathBuf>,
}

impl Default for CoqSidecar {
    fn default() -> Self { Self::new() }
}

impl CoqSidecar {
    pub fn new() -> Self {
        Self { coqc_path: Self::find_coqc() }
    }

    pub fn with_path(path: impl Into<std::path::PathBuf>) -> Self {
        Self { coqc_path: Some(path.into()) }
    }

    pub fn coqc_available() -> bool { Self::find_coqc().is_some() }

    fn find_coqc() -> Option<std::path::PathBuf> {
        let names = if cfg!(windows) { vec!["coqc.exe", "coqc"] } else { vec!["coqc"] };
        for name in names {
            if let Some(p) = which_bin(name) { return Some(p); }
        }
        None
    }

    pub fn verify(&self, req: &CoqRequest) -> CoqResult<CoqResponse> {
        let coqc = self.coqc_path.as_ref().ok_or(CoqError::BinaryNotFound)?;

        let dir = tempfile_dir()?;
        let src_path = dir.join("proof.v");
        {
            let mut f = std::fs::File::create(&src_path)?;
            f.write_all(req.source.as_bytes())?;
        }

        let mut cmd = Command::new(coqc);
        cmd.arg(&src_path).stdout(Stdio::piped()).stderr(Stdio::piped());

        if let Some(secs) = req.timeout_secs {
            // coqc has no built-in timeout flag; rely on OS-level kill via thread
            return self.run_with_timeout(cmd, secs, &src_path);
        }

        let out = cmd.output()?;
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        let success = out.status.success() && !stderr.contains("Error:");

        let diagnostics = parse_coq_diagnostics(&stderr);

        if !success {
            return Err(CoqError::VerificationFailed(stderr));
        }

        Ok(CoqResponse { success, diagnostics, stdout, stderr })
    }

    fn run_with_timeout(
        &self,
        mut cmd: Command,
        secs: u64,
        src_path: &std::path::Path,
    ) -> CoqResult<CoqResponse> {
        use std::time::{Duration, Instant};

        let mut child = cmd.spawn()?;
        let deadline = Instant::now() + Duration::from_secs(secs);
        loop {
            if Instant::now() >= deadline {
                let _ = child.kill();
                return Err(CoqError::Other(format!("coqc timed out after {secs}s")));
            }
            match child.try_wait()? {
                Some(_) => break,
                None => std::thread::sleep(Duration::from_millis(100)),
            }
        }
        let out = child.wait_with_output()?;
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        let success = out.status.success() && !stderr.contains("Error:");
        let diagnostics = parse_coq_diagnostics(&stderr);
        if !success { return Err(CoqError::VerificationFailed(stderr)); }
        Ok(CoqResponse { success, diagnostics, stdout, stderr })
    }
}

fn parse_coq_diagnostics(stderr: &str) -> Vec<CoqDiagnostic> {
    stderr.lines()
        .filter(|l| l.contains("Error:") || l.contains("Warning:"))
        .map(|l| {
            let severity = if l.contains("Error:") { "error" } else { "warning" };
            CoqDiagnostic { severity: severity.into(), message: l.to_string() }
        })
        .collect()
}

fn tempfile_dir() -> CoqResult<std::path::PathBuf> {
    let dir = std::env::temp_dir().join("bonsai-coq");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn which_bin(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths)
                .map(|p| p.join(name))
                .find(|p| p.exists())
        })
        .flatten()
}
