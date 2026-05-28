//! Isabelle proof-assistant sidecar.
//!
//! Drives a locally-installed Isabelle installation via `isabelle process`.
//! No network calls are made.
//!
//! # Protocol
//! 1. Write source to a temporary `.thy` file.
//! 2. Spawn `isabelle process -T <theory>`.
//! 3. Parse stdout for "*** ERROR" / "Finished" patterns.

use std::io::Write as _;
use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IsabelleError {
    #[error("isabelle not found — install Isabelle and ensure `isabelle` is on PATH")]
    BinaryNotFound,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Isabelle reported errors:\n{0}")]
    VerificationFailed(String),
    #[error("{0}")]
    Other(String),
}

pub type IsabelleResult<T> = Result<T, IsabelleError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsabelleRequest {
    /// Isabelle theory source (`.thy` content). Must include `theory <name> ...` header.
    pub source: String,
    /// Theory name (must match the `theory` declaration in source).
    pub theory_name: String,
    /// Import list for the theory header (default: `["Main"]`).
    pub imports: Vec<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsabelleResponse {
    pub success: bool,
    pub diagnostics: Vec<IsabelleDiagnostic>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsabelleDiagnostic {
    pub severity: String,
    pub message: String,
}

pub struct IsabelleSidecar {
    isabelle_path: Option<std::path::PathBuf>,
}

impl Default for IsabelleSidecar {
    fn default() -> Self { Self::new() }
}

impl IsabelleSidecar {
    pub fn new() -> Self {
        Self { isabelle_path: Self::find_isabelle() }
    }

    pub fn with_path(path: impl Into<std::path::PathBuf>) -> Self {
        Self { isabelle_path: Some(path.into()) }
    }

    pub fn isabelle_available() -> bool { Self::find_isabelle().is_some() }

    fn find_isabelle() -> Option<std::path::PathBuf> {
        let names = if cfg!(windows) {
            vec!["isabelle.exe", "Isabelle.exe", "isabelle"]
        } else {
            vec!["isabelle"]
        };
        for name in names {
            if let Some(p) = which(name) { return Some(p); }
        }
        // Check common installation paths
        let common = if cfg!(windows) {
            vec![r"C:\Isabelle\bin\isabelle.exe"]
        } else {
            vec!["/usr/local/Isabelle/bin/isabelle", "/opt/Isabelle/bin/isabelle"]
        };
        for path in common {
            let p = std::path::Path::new(path);
            if p.exists() { return Some(p.to_path_buf()); }
        }
        None
    }

    pub fn verify(&self, req: &IsabelleRequest) -> IsabelleResult<IsabelleResponse> {
        let isabelle = self.isabelle_path.as_ref().ok_or(IsabelleError::BinaryNotFound)?;

        let dir = tempfile_dir()?;
        let theory_path = dir.join(format!("{}.thy", req.theory_name));

        let imports = if req.imports.is_empty() {
            vec!["Main".to_string()]
        } else {
            req.imports.clone()
        };
        let imports_str = imports.join(" ");

        // Wrap source in a proper theory header if not already present
        let source = if req.source.trim_start().starts_with("theory") {
            req.source.clone()
        } else {
            format!(
                "theory {}\n  imports {imports_str}\nbegin\n\n{}\n\nend",
                req.theory_name, req.source
            )
        };

        {
            let mut f = std::fs::File::create(&theory_path)?;
            f.write_all(source.as_bytes())?;
        }

        let mut cmd = Command::new(isabelle);
        cmd.arg("process")
            .arg("-T").arg(theory_path.to_str().unwrap_or(""))
            .stdout(Stdio::piped()).stderr(Stdio::piped());

        let timeout = req.timeout_secs.unwrap_or(120);
        let out = run_with_timeout(cmd, timeout)
            .map_err(IsabelleError::Other)?;

        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        let combined = format!("{stdout}\n{stderr}");

        let success = out.status.success()
            && !combined.contains("*** ERROR")
            && !combined.contains("FAILED");

        let diagnostics = parse_isabelle_diagnostics(&combined);

        if !success {
            return Err(IsabelleError::VerificationFailed(combined));
        }

        Ok(IsabelleResponse { success, diagnostics, stdout, stderr })
    }
}

fn parse_isabelle_diagnostics(output: &str) -> Vec<IsabelleDiagnostic> {
    output.lines()
        .filter(|l| l.starts_with("*** ERROR") || l.starts_with("*** WARNING"))
        .map(|l| {
            let severity = if l.contains("ERROR") { "error" } else { "warning" };
            IsabelleDiagnostic { severity: severity.into(), message: l.trim_start_matches("*** ").to_string() }
        })
        .collect()
}

fn tempfile_dir() -> IsabelleResult<std::path::PathBuf> {
    let dir = std::env::temp_dir().join("bonsai-isabelle");
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
            return Err(format!("isabelle timed out after {secs}s"));
        }
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(_) => break,
            None => std::thread::sleep(Duration::from_millis(200)),
        }
    }
    child.wait_with_output().map_err(|e| e.to_string())
}
