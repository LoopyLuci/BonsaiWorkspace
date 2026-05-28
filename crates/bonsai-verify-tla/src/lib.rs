//! TLA+ model-checker sidecar.
//!
//! Drives a locally-installed TLC model checker (`tlc2` JAR or `tlc` script).
//! No network calls are made.
//!
//! # Protocol
//! 1. Write a temporary `.tla` spec file + optional `.cfg` model config.
//! 2. Spawn `java -jar tla2tools.jar <spec>` or the `tlc` wrapper.
//! 3. Parse TLC's stdout for "Model checking completed" / "Error:" lines.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum TlaError {
    #[error("TLC not found — install tla2tools.jar and set BONSAI_TLC_JAR, or put `tlc` on PATH")]
    BinaryNotFound,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TLC found a violation:\n{0}")]
    Violation(String),
    #[error("TLC reported an error:\n{0}")]
    TlcError(String),
    #[error("{0}")]
    Other(String),
}

pub type TlaResult<T> = Result<T, TlaError>;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlaRequest {
    /// TLA+ specification source.
    pub spec: String,
    /// TLC model config (`.cfg` content). If None, a minimal config is generated.
    pub config: Option<String>,
    /// Name of the SPECIFICATION in the TLA+ module (default: "Spec").
    pub spec_name: Option<String>,
    pub timeout_secs: Option<u64>,
    /// Extra flags to pass to TLC.
    pub extra_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlaResponse {
    pub success: bool,
    pub states_explored: Option<u64>,
    pub errors: Vec<String>,
    pub violations: Vec<String>,
    pub stdout: String,
}

// ── Sidecar ───────────────────────────────────────────────────────────────────

pub struct TlaSidecar {
    /// Path to tla2tools.jar (overrides env var BONSAI_TLC_JAR).
    pub jar_path: Option<PathBuf>,
    /// Path to `java` binary (default: search PATH).
    pub java_path: Option<PathBuf>,
}

impl TlaSidecar {
    pub fn new() -> Self { Self { jar_path: None, java_path: None } }

    pub fn verify(&self, req: &TlaRequest) -> TlaResult<TlaResponse> {
        let (java, jar) = self.find_tlc()?;

        // Write spec
        let spec_name = req.spec_name.clone().unwrap_or_else(|| "BonsaiSpec".into());
        let mut spec_path = std::env::temp_dir();
        spec_path.push(format!("{spec_name}_{}.tla", std::process::id()));
        {
            let mut f = std::fs::File::create(&spec_path)?;
            f.write_all(req.spec.as_bytes())?;
        }

        // Write config
        let cfg_content = req.config.clone().unwrap_or_else(|| {
            format!("SPECIFICATION {spec_name}\nINVARIANT TypeOK\n")
        });
        let mut cfg_path = spec_path.clone();
        cfg_path.set_extension("cfg");
        {
            let mut f = std::fs::File::create(&cfg_path)?;
            f.write_all(cfg_content.as_bytes())?;
        }

        let result = self.run_tlc(&java, &jar, &spec_path, &cfg_path, req);
        let _ = std::fs::remove_file(&spec_path);
        let _ = std::fs::remove_file(&cfg_path);
        result
    }

    fn find_tlc(&self) -> TlaResult<(PathBuf, PathBuf)> {
        let java = self.java_path.clone()
            .or_else(|| which_binary("java"))
            .ok_or(TlaError::BinaryNotFound)?;

        let jar = self.jar_path.clone()
            .or_else(|| std::env::var("BONSAI_TLC_JAR").ok().map(PathBuf::from))
            .or_else(|| {
                // Look for tla2tools.jar in common locations
                let candidates = [
                    "/usr/local/lib/tla2tools.jar",
                    "/opt/tla/tla2tools.jar",
                    "C:\\tla\\tla2tools.jar",
                ];
                candidates.iter().map(Path::new).find(|p| p.exists()).map(PathBuf::from)
            })
            .ok_or(TlaError::BinaryNotFound)?;

        Ok((java, jar))
    }

    fn run_tlc(
        &self,
        java: &PathBuf,
        jar: &PathBuf,
        spec: &PathBuf,
        cfg: &PathBuf,
        req: &TlaRequest,
    ) -> TlaResult<TlaResponse> {
        let mut cmd = Command::new(java);
        cmd.arg("-jar").arg(jar)
           .arg("-config").arg(cfg)
           .arg(spec);
        for flag in &req.extra_flags { cmd.arg(flag); }
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd.output().map_err(|e| if e.kind() == std::io::ErrorKind::NotFound {
            TlaError::BinaryNotFound
        } else {
            TlaError::Io(e)
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

        let mut errors     = Vec::new();
        let mut violations = Vec::new();
        let mut states_explored = None;
        let mut in_violation = false;
        let mut violation_buf = String::new();

        for line in stdout.lines() {
            if line.contains("Model checking completed.") {
                // Extract state count
                if let Some(idx) = line.find("states explored") {
                    let before = &line[..idx];
                    let n: Option<u64> = before.split_whitespace().last().and_then(|s| s.parse().ok());
                    states_explored = n;
                }
            } else if line.starts_with("Error:") || line.starts_with("TLC threw an unexpected") {
                errors.push(line.to_string());
            } else if line.contains("Invariant") && line.contains("violated") {
                in_violation = true;
                violation_buf.push_str(line);
                violation_buf.push('\n');
            } else if in_violation {
                if line.is_empty() || line.starts_with("The") || line.starts_with("Model") {
                    violations.push(violation_buf.trim().to_string());
                    violation_buf.clear();
                    in_violation = false;
                } else {
                    violation_buf.push_str(line);
                    violation_buf.push('\n');
                }
            }
        }
        if !violation_buf.is_empty() {
            violations.push(violation_buf.trim().to_string());
        }

        if !violations.is_empty() {
            return Err(TlaError::Violation(violations.join("\n---\n")));
        }
        if !errors.is_empty() && !output.status.success() {
            return Err(TlaError::TlcError(errors.join("\n")));
        }

        Ok(TlaResponse {
            success: output.status.success() && violations.is_empty(),
            states_explored,
            errors,
            violations,
            stdout,
        })
    }
}

impl Default for TlaSidecar {
    fn default() -> Self { Self::new() }
}

fn which_binary(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")?.to_str()?.split(if cfg!(windows) { ';' } else { ':' })
        .map(|dir| {
            let mut p = PathBuf::from(dir);
            p.push(name);
            if cfg!(windows) && !name.ends_with(".exe") { p.set_extension("exe"); }
            p
        })
        .find(|p| p.is_file())
}

/// Returns true if `java` is on PATH (necessary but not sufficient for TLC).
pub fn java_available() -> bool { which_binary("java").is_some() }

/// Returns true if BONSAI_TLC_JAR is set and points to an existing file.
pub fn tlc_jar_available() -> bool {
    std::env::var("BONSAI_TLC_JAR").ok().map(|p| Path::new(&p).is_file()).unwrap_or(false)
}

pub fn tla_available() -> bool { java_available() && tlc_jar_available() }
