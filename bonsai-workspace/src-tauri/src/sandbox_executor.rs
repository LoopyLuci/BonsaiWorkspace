//! Sandboxed code execution: Python venv tier with process isolation and timeout.
//! WASM tier is a stub — add wasmtime when targeting Linux/Mac.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxTier {
    Venv,
    Wasm,
}

#[derive(Debug, Deserialize)]
pub struct SandboxRequest {
    pub tier: SandboxTier,
    pub language: String,
    pub code: String,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct SandboxResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

#[tauri::command]
pub async fn run_sandboxed_code(request: SandboxRequest) -> Result<SandboxResult, String> {
    let timeout = Duration::from_secs(request.timeout_secs.unwrap_or(30));
    match request.tier {
        SandboxTier::Venv => execute_in_venv(&request.language, &request.code, timeout).await,
        SandboxTier::Wasm => execute_wasm_stub(&request.code),
    }
}

// ── Python venv tier ──────────────────────────────────────────────────────────

async fn execute_in_venv(language: &str, code: &str, timeout: Duration) -> Result<SandboxResult, String> {
    if language != "python" {
        return Err(format!("venv tier only supports 'python', got '{language}'"));
    }

    let venv_dir = get_or_create_venv().await?;
    let python = venv_python(&venv_dir);

    // Fall back to system Python if venv binary doesn't exist yet
    let python_exe = if python.exists() {
        python
    } else {
        std::path::PathBuf::from(find_python()?)
    };

    // Write code to a temp file so we don't hit command-line length limits
    let tmp = std::env::temp_dir().join(format!("bonsai_sandbox_{}.py", uuid_short()));
    tokio::fs::write(&tmp, code)
        .await
        .map_err(|e| format!("Cannot write sandbox script: {e}"))?;

    let mut cmd = tokio::process::Command::new(&python_exe);
    cmd.arg(&tmp)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    // Windows Job Object isolation would go here; for now we rely on timeout
    let result = tokio::time::timeout(timeout, cmd.output()).await;

    let _ = tokio::fs::remove_file(&tmp).await;

    match result {
        Err(_) => Ok(SandboxResult {
            stdout: String::new(),
            stderr: "Execution timed out".into(),
            exit_code: -1,
            timed_out: true,
        }),
        Ok(Err(e)) => Err(format!("Spawn failed: {e}")),
        Ok(Ok(out)) => Ok(SandboxResult {
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            exit_code: out.status.code().unwrap_or(-1),
            timed_out: false,
        }),
    }
}

async fn get_or_create_venv() -> Result<PathBuf, String> {
    let dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.bonsai.workspace")
        .join("sandbox_venv");

    if !dir.join("pyvenv.cfg").exists() {
        info!(?dir, "Creating sandbox venv");
        let python_cmd = find_python()?;
        let out = tokio::process::Command::new(&python_cmd)
            .args(["-m", "venv", dir.to_string_lossy().as_ref()])
            .output()
            .await
            .map_err(|e| format!("{python_cmd} -m venv failed: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "venv creation failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
    }
    Ok(dir)
}

fn find_python() -> Result<String, String> {
    for candidate in &["python", "python3", "py"] {
        if which_python(candidate) {
            return Ok(candidate.to_string());
        }
    }
    Err("Python not found. Install Python 3 and ensure it is on PATH.".into())
}

fn which_python(name: &str) -> bool {
    std::process::Command::new(name)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn venv_python(venv_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        venv_dir.join("Scripts").join("python.exe")
    } else {
        venv_dir.join("bin").join("python")
    }
}

fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| format!("{:x}", d.subsec_nanos()))
        .unwrap_or_else(|_| "0".into())
}

/// Public entry point for plugin_host: run Python code in the sandbox venv.
pub async fn execute_plugin_code(code: &str) -> Result<SandboxResult, String> {
    execute_in_venv("python", code, std::time::Duration::from_secs(30)).await
}

// ── WASM stub ─────────────────────────────────────────────────────────────────

fn execute_wasm_stub(_code: &str) -> Result<SandboxResult, String> {
    // wasmtime is a large dependency with platform-specific requirements.
    // Wire a real implementation when targeting a platform that supports it.
    Err("WASM execution tier not enabled in this build".into())
}
