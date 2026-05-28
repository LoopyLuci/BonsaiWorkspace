//! F* verification sidecar.
//!
//! Drives a locally-installed `fstar.exe` / `fstar` binary.  No network
//! calls are made — the binary must be installed offline.
//!
//! # Protocol
//! 1. Write a temporary `.fst` file.
//! 2. Spawn `fstar.exe --ide <file>` in batch mode (or plain `fstar.exe <file>`).
//! 3. Parse exit code + stderr for "Verified module" / error lines.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum FStarError {
    #[error("fstar binary not found — install F* and ensure `fstar.exe` is on PATH")]
    BinaryNotFound,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("F* reported errors:\n{0}")]
    VerificationFailed(String),
    #[error("{0}")]
    Other(String),
}

pub type FStarResult<T> = Result<T, FStarError>;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FStarRequest {
    /// F* source code (.fst content).
    pub source: String,
    /// Extra CLI flags (e.g. `--include` paths).
    pub extra_flags: Vec<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FStarResponse {
    pub success: bool,
    pub verified_modules: Vec<String>,
    pub errors: Vec<FStarDiagnostic>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FStarDiagnostic {
    pub level: String,
    pub message: String,
    pub range: Option<String>,
}

// ── Sidecar ───────────────────────────────────────────────────────────────────

pub struct FStarSidecar {
    pub fstar_path: Option<PathBuf>,
}

impl FStarSidecar {
    pub fn new() -> Self { Self { fstar_path: None } }

    pub fn with_path(path: PathBuf) -> Self { Self { fstar_path: Some(path) } }

    pub fn verify(&self, req: &FStarRequest) -> FStarResult<FStarResponse> {
        let bin = self.find_fstar()?;

        let mut tmp = std::env::temp_dir();
        tmp.push(format!("bonsai_fstar_{}.fst", std::process::id()));
        {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(req.source.as_bytes())?;
        }

        let result = self.run_fstar(&bin, &tmp, req);
        let _ = std::fs::remove_file(&tmp);
        result
    }

    fn find_fstar(&self) -> FStarResult<PathBuf> {
        if let Some(p) = &self.fstar_path {
            if p.exists() { return Ok(p.clone()); }
        }
        // Try fstar.exe (Windows) then fstar
        which_binary("fstar.exe")
            .or_else(|| which_binary("fstar"))
            .ok_or(FStarError::BinaryNotFound)
    }

    fn run_fstar(&self, bin: &PathBuf, file: &PathBuf, req: &FStarRequest) -> FStarResult<FStarResponse> {
        let mut cmd = Command::new(bin);
        cmd.arg(file);
        for flag in &req.extra_flags { cmd.arg(flag); }
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd.output().map_err(|e| if e.kind() == std::io::ErrorKind::NotFound {
            FStarError::BinaryNotFound
        } else {
            FStarError::Io(e)
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        // Parse output for verified modules and errors
        let mut verified = Vec::new();
        let mut errors   = Vec::new();

        for line in stdout.lines().chain(stderr.lines()) {
            if line.contains("Verified module:") {
                let module = line.split("Verified module:").nth(1).unwrap_or("").trim().to_string();
                verified.push(module);
            } else if line.contains("(Error") || line.starts_with("Error") {
                errors.push(FStarDiagnostic {
                    level: "error".into(),
                    message: line.to_string(),
                    range: None,
                });
            } else if line.contains("(Warning") {
                errors.push(FStarDiagnostic {
                    level: "warning".into(),
                    message: line.to_string(),
                    range: None,
                });
            }
        }

        let success = output.status.success() && errors.iter().all(|e| e.level != "error");

        if !success && !errors.is_empty() {
            let msg = errors.iter().filter(|e| e.level == "error")
                .map(|e| e.message.as_str()).collect::<Vec<_>>().join("\n");
            return Err(FStarError::VerificationFailed(msg));
        }

        Ok(FStarResponse { success, verified_modules: verified, errors, stdout, stderr })
    }
}

impl Default for FStarSidecar {
    fn default() -> Self { Self::new() }
}

fn which_binary(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")?.to_str()?.split(if cfg!(windows) { ';' } else { ':' })
        .map(|dir| {
            let mut p = PathBuf::from(dir);
            p.push(name);
            p
        })
        .find(|p| p.is_file())
}

pub fn fstar_available() -> bool {
    which_binary("fstar.exe").or_else(|| which_binary("fstar")).is_some()
}

pub fn verify_fstar_source(source: &str) -> FStarResult<bool> {
    if !fstar_available() { return Ok(false); }
    FStarSidecar::new().verify(&FStarRequest {
        source: source.into(),
        extra_flags: vec![],
        timeout_secs: Some(60),
    })?;
    Ok(true)
}
