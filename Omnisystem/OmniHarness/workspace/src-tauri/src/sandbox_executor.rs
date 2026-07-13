//! Sandboxed code execution: Python venv tier with process isolation and timeout.
//! WASM tier is a stub — add wasmtime when targeting Linux/Mac.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

// ── Windows Job Object isolation ──────────────────────────────────────────────
// `kill_on_drop` on a tokio::process::Child only terminates the direct child —
// if sandboxed code spawns its own subprocess, that grandchild survives a
// timeout or drop untouched. A Job Object closes that gap: every process
// assigned to it dies together when the job handle closes, and it also caps
// per-process memory and total process count to blunt memory exhaustion and
// fork-bomb style abuse. Linux/Mac still fall back to timeout-only isolation
// (no equivalent wired in here yet — would need cgroups/rlimit).
#[cfg(windows)]
mod job_object {
    use std::ffi::c_void;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOB_OBJECT_LIMIT_PROCESS_MEMORY,
    };

    const MAX_PROCESS_MEMORY_BYTES: usize = 512 * 1024 * 1024; // 512 MB per process
    const MAX_ACTIVE_PROCESSES: u32 = 16; // blunt fork bombs, allow legitimate subprocess use

    /// An anonymous Job Object that caps memory/process-count for anything
    /// assigned to it, and kills the whole tree when dropped.
    pub struct SandboxJob {
        handle: HANDLE,
    }

    // SAFETY: HANDLE is an opaque OS handle value; the job it names isn't
    // tied to the thread that created it.
    unsafe impl Send for SandboxJob {}

    impl SandboxJob {
        pub fn new() -> Result<Self, String> {
            // SAFETY: standard Win32 Job Objects sequence — create an
            // anonymous job, then set its limit info via a correctly-sized
            // struct pointer, per the documented CreateJobObjectW /
            // SetInformationJobObject contract.
            unsafe {
                let handle = CreateJobObjectW(None, PCWSTR::null())
                    .map_err(|e| format!("CreateJobObjectW failed: {e}"))?;

                let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                    | JOB_OBJECT_LIMIT_PROCESS_MEMORY
                    | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
                info.BasicLimitInformation.ActiveProcessLimit = MAX_ACTIVE_PROCESSES;
                info.ProcessMemoryLimit = MAX_PROCESS_MEMORY_BYTES;

                SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
                .map_err(|e| format!("SetInformationJobObject failed: {e}"))?;

                Ok(Self { handle })
            }
        }

        /// Assign a spawned child to this job. Call this as soon as possible
        /// after spawn — there's a narrow window before this call where a
        /// child that forks instantly could escape the job; closing it fully
        /// would require launching suspended and resuming after assignment,
        /// which tokio's process API doesn't expose a thread handle for.
        pub fn assign(&self, child: &tokio::process::Child) -> Result<(), String> {
            let raw = child
                .raw_handle()
                .ok_or_else(|| "child process handle unavailable".to_string())?;
            // SAFETY: `raw` is the live handle of a process we just spawned
            // and still own; `self.handle` is a valid job object handle.
            unsafe {
                AssignProcessToJobObject(self.handle, HANDLE(raw as *mut c_void))
                    .map_err(|e| format!("AssignProcessToJobObject failed: {e}"))
            }
        }
    }

    impl Drop for SandboxJob {
        fn drop(&mut self) {
            // Closing the last handle to a job with KILL_ON_JOB_CLOSE set
            // terminates every process still running in it — this is what
            // actually closes the "detached grandchild survives" gap.
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

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

async fn execute_in_venv(
    language: &str,
    code: &str,
    timeout: Duration,
) -> Result<SandboxResult, String> {
    if language != "python" {
        return Err(format!(
            "venv tier only supports 'python', got '{language}'"
        ));
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
    let tmp = std::env::temp_dir().join(format!("workspace_sandbox_{}.py", uuid_short()));
    tokio::fs::write(&tmp, code)
        .await
        .map_err(|e| format!("Cannot write sandbox script: {e}"))?;

    let mut cmd = tokio::process::Command::new(&python_exe);
    cmd.arg(&tmp)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    #[cfg(windows)]
    let job = match job_object::SandboxJob::new() {
        Ok(job) => Some(job),
        Err(e) => {
            warn!(error = %e, "sandbox: failed to create Job Object, falling back to timeout-only isolation");
            None
        }
    };

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Spawn failed: {e}"))?;

    #[cfg(windows)]
    if let Some(job) = &job {
        if let Err(e) = job.assign(&child) {
            warn!(error = %e, "sandbox: failed to assign process to Job Object");
        }
    }

    let result = tokio::time::timeout(timeout, child.wait_with_output()).await;

    // On Windows, `job` drops here (if it was created), which kills any
    // surviving process in it, including grandchildren that `kill_on_drop`
    // on `child` alone would never reach.
    let _ = tokio::fs::remove_file(&tmp).await;

    match result {
        Err(_) => Ok(SandboxResult {
            stdout: String::new(),
            stderr: "Execution timed out".into(),
            exit_code: -1,
            timed_out: true,
        }),
        Ok(Err(e)) => Err(format!("Failed to collect process output: {e}")),
        Ok(Ok(out)) => Ok(SandboxResult {
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            exit_code: out.status.code().unwrap_or(-1),
            timed_out: false,
        }),
    }
}

// Two concurrent sandbox executions (e.g. two agent tool calls in flight at
// once) can both see the venv missing and race to create it — `python -m
// venv` on the same target directory from two processes at once fails with
// WinError 183/5 rather than one just winning cleanly. Serialize creation so
// only the first caller does the work; everyone else finds it already there.
static VENV_INIT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn get_or_create_venv() -> Result<PathBuf, String> {
    let dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.omnisystem.workspace")
        .join("sandbox_venv");

    let _guard = VENV_INIT_LOCK.lock().await;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn venv_executes_python_and_returns_stdout() {
        let result = execute_in_venv("python", "print(2 + 2)", Duration::from_secs(20))
            .await
            .expect("execution should succeed");
        assert_eq!(result.stdout.trim(), "4");
        assert!(!result.timed_out);
    }

    #[tokio::test]
    async fn non_python_language_is_rejected() {
        let result = execute_in_venv("ruby", "puts 1", Duration::from_secs(5)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn execution_that_exceeds_timeout_is_reported_as_timed_out() {
        let result = execute_in_venv(
            "python",
            "import time; time.sleep(5)",
            Duration::from_millis(200),
        )
        .await
        .expect("timeout path should still return Ok with timed_out=true");
        assert!(result.timed_out);
    }

    /// Proves the Job Object actually closes the gap `kill_on_drop` leaves
    /// open: a detached grandchild process spawned by sandboxed code must
    /// not outlive the sandbox call that spawned its parent.
    #[cfg(windows)]
    #[tokio::test]
    async fn job_object_kills_detached_grandchild() {
        let dir = std::env::temp_dir();
        let heartbeat = dir.join(format!("sandbox_test_heartbeat_{}.txt", uuid_short()));
        let grandchild_script = dir.join(format!("sandbox_test_grandchild_{}.py", uuid_short()));

        std::fs::write(
            &grandchild_script,
            format!(
                "import time\nwith open(r'{}', 'a') as f:\n    for _ in range(50):\n        f.write('beat\\n')\n        f.flush()\n        time.sleep(0.1)\n",
                heartbeat.display()
            ),
        )
        .expect("write grandchild script");

        let parent_code = format!(
            "import subprocess, sys\nsubprocess.Popen([sys.executable, r'{}'])\n",
            grandchild_script.display()
        );

        execute_in_venv("python", &parent_code, Duration::from_secs(15))
            .await
            .expect("parent script should run and exit");

        // Give the grandchild a moment to have actually started writing.
        tokio::time::sleep(Duration::from_millis(400)).await;
        let count_after_return = std::fs::read_to_string(&heartbeat)
            .unwrap_or_default()
            .lines()
            .count();

        // The sandbox job (dropped when execute_in_venv returned above)
        // should have killed the grandchild by now; if it hadn't, this sleep
        // window is long enough for several more heartbeats to land.
        tokio::time::sleep(Duration::from_millis(1500)).await;
        let count_later = std::fs::read_to_string(&heartbeat)
            .unwrap_or_default()
            .lines()
            .count();

        let _ = std::fs::remove_file(&heartbeat);
        let _ = std::fs::remove_file(&grandchild_script);

        assert!(
            count_after_return > 0,
            "grandchild should have started writing heartbeats before the parent returned"
        );
        assert_eq!(
            count_later, count_after_return,
            "grandchild kept writing heartbeats after execute_in_venv returned — Job Object failed to kill it"
        );
    }
}
