//! Lean 4 verification sidecar.
//!
//! Drives a locally-installed `lean` binary via subprocess.  The binary must
//! be on PATH (installed offline from the Lean4 release archive) — no network
//! traffic is initiated by this crate.
//!
//! # Protocol
//! 1. Write a temporary `.lean` file with the proposition + tactics.
//! 2. Spawn `lean --json <file>` and capture stdout.
//! 3. Parse the JSON diagnostics stream to determine `Ok / Err`.
//! 4. Delete the temp file and return the result.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum LeanError {
    #[error("lean binary not found — install Lean 4 and ensure `lean` is on PATH")]
    BinaryNotFound,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("lean reported errors:\n{0}")]
    VerificationFailed(String),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}

pub type LeanResult<T> = Result<T, LeanError>;

// ── Types ─────────────────────────────────────────────────────────────────────

/// A Lean 4 verification request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanRequest {
    /// Lean 4 source (imports + theorem statement + proof).
    pub source: String,
    /// Optional timeout in seconds (default: 30).
    pub timeout_secs: Option<u64>,
}

/// Result of a Lean verification run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanResponse {
    pub success: bool,
    pub diagnostics: Vec<LeanDiagnostic>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanDiagnostic {
    pub severity: String, // "error" | "warning" | "information"
    pub message: String,
    pub pos: Option<LeanPos>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanPos {
    pub line: u32,
    pub column: u32,
}

// ── Sidecar ───────────────────────────────────────────────────────────────────

pub struct LeanSidecar {
    /// Path to the `lean` binary. If None, we search PATH.
    pub lean_path: Option<PathBuf>,
}

impl LeanSidecar {
    pub fn new() -> Self { Self { lean_path: None } }

    pub fn with_path(path: PathBuf) -> Self { Self { lean_path: Some(path) } }

    /// Verify a Lean 4 source file. Returns `Ok(LeanResponse)` if the
    /// subprocess exited cleanly; the response's `success` field indicates
    /// whether Lean itself reported errors.
    pub fn verify(&self, req: &LeanRequest) -> LeanResult<LeanResponse> {
        let lean_bin = self.find_lean()?;

        // Write source to temp file
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("bonsai_lean_{}.lean", std::process::id()));
        {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(req.source.as_bytes())?;
        }

        let timeout = req.timeout_secs.unwrap_or(30);
        let result = self.run_lean(&lean_bin, &tmp, timeout);
        let _ = std::fs::remove_file(&tmp);
        result
    }

    fn find_lean(&self) -> LeanResult<PathBuf> {
        if let Some(p) = &self.lean_path {
            if p.exists() { return Ok(p.clone()); }
        }
        which_binary("lean").ok_or(LeanError::BinaryNotFound)
    }

    fn run_lean(&self, lean: &PathBuf, file: &PathBuf, _timeout_secs: u64) -> LeanResult<LeanResponse> {
        let output = Command::new(lean)
            .arg("--json")
            .arg(file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| if e.kind() == std::io::ErrorKind::NotFound {
                LeanError::BinaryNotFound
            } else {
                LeanError::Io(e)
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        // Parse JSON diagnostics (one JSON object per line)
        let mut diagnostics = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                let severity = v["severity"].as_str().unwrap_or("information").to_string();
                let message  = v["data"].as_str().or_else(|| v["message"].as_str()).unwrap_or("").to_string();
                let pos = v["pos"].as_object().map(|p| LeanPos {
                    line:   p["line"].as_u64().unwrap_or(0) as u32,
                    column: p["column"].as_u64().unwrap_or(0) as u32,
                });
                diagnostics.push(LeanDiagnostic { severity, message, pos });
            }
        }

        let has_errors = diagnostics.iter().any(|d| d.severity == "error")
            || !output.status.success();

        if has_errors {
            let msg = diagnostics.iter()
                .filter(|d| d.severity == "error")
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            return Err(LeanError::VerificationFailed(msg));
        }

        Ok(LeanResponse { success: true, diagnostics, stdout, stderr })
    }
}

impl Default for LeanSidecar {
    fn default() -> Self { Self::new() }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn which_binary(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")?.to_str()?.split(if cfg!(windows) { ';' } else { ':' })
        .map(|dir| {
            let mut p = PathBuf::from(dir);
            p.push(name);
            if cfg!(windows) { p.set_extension("exe"); }
            p
        })
        .find(|p| p.is_file())
}

// ── Convenience ───────────────────────────────────────────────────────────────

/// Quick check: is `lean` available on PATH?
pub fn lean_available() -> bool {
    which_binary("lean").is_some()
}

/// Verify a simple proposition expressed as a Lean 4 source string.
/// Returns `Ok(true)` if Lean accepts it, `Ok(false)` if not installed,
/// `Err` if Lean is installed but rejects the proof.
pub fn verify_lean_source(source: &str) -> LeanResult<bool> {
    if !lean_available() { return Ok(false); }
    let sidecar = LeanSidecar::new();
    sidecar.verify(&LeanRequest { source: source.into(), timeout_secs: Some(30) })?;
    Ok(true)
}
