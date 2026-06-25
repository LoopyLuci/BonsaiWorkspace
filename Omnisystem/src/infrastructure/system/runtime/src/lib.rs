#![allow(clippy::assertions_on_constants)]

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use std::sync::Once;

type JoinHandleOpt = tokio::task::JoinHandle<anyhow::Result<Option<i32>>>;
type InProcessJoin = std::sync::Arc<std::sync::Mutex<Option<JoinHandleOpt>>>;

static BB_VERSION_CHECK: Once = Once::new();

#[derive(Default)]
pub struct RuntimeManager {}

#[async_trait]
pub trait RuntimeController: Send + Sync {
    fn pid(&self) -> Option<i64>;
    async fn kill(&mut self) -> Result<()>;
    async fn wait(&mut self) -> Result<Option<i32>>;
}

// ── Windows Job Object wrapper ────────────────────────────────────────────────

/// RAII wrapper that closes a Windows Job Object handle on drop.
/// When `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` is set, closing this handle
/// terminates all processes in the job — enforcing resource limits.
#[cfg(windows)]
struct JobHandle(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for JobHandle {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

// SAFETY: HANDLE is a process-local value; we own it exclusively.
#[cfg(windows)]
unsafe impl Send for JobHandle {}
#[cfg(windows)]
unsafe impl Sync for JobHandle {}

/// Apply Windows Job Object resource limits to an already-spawned process.
///
/// Limits applied:
/// - `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`: process is terminated when the
///   `JobHandle` is dropped (i.e., when `ProcessController` is dropped).
/// - `JOB_OBJECT_LIMIT_PROCESS_TIME`: per-process CPU time limit.
/// - `JOB_OBJECT_LIMIT_PROCESS_MEMORY`: per-process virtual memory limit.
#[cfg(windows)]
fn create_job_for_pid(pid: u32, max_cpu_secs: u64, max_memory_mb: u64) -> Result<JobHandle> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::{
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
                SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
                JOB_OBJECT_LIMIT_PROCESS_TIME,
            },
            Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE},
        },
    };

    unsafe {
        let process: HANDLE = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
        if process.is_null() {
            anyhow::bail!("OpenProcess failed: {}", std::io::Error::last_os_error());
        }

        let job: HANDLE = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            CloseHandle(process);
            anyhow::bail!(
                "CreateJobObjectW failed: {}",
                std::io::Error::last_os_error()
            );
        }

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JOB_OBJECT_LIMIT_PROCESS_TIME
            | JOB_OBJECT_LIMIT_PROCESS_MEMORY;
        // CPU time in 100-nanosecond intervals.
        info.BasicLimitInformation.PerProcessUserTimeLimit = (max_cpu_secs * 10_000_000) as i64;
        // Virtual memory limit in bytes.
        info.ProcessMemoryLimit = max_memory_mb as usize * 1024 * 1024;

        let ok = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        if ok == 0 {
            CloseHandle(job);
            CloseHandle(process);
            anyhow::bail!(
                "SetInformationJobObject failed: {}",
                std::io::Error::last_os_error()
            );
        }

        let ok = AssignProcessToJobObject(job, process);
        CloseHandle(process);
        if ok == 0 {
            CloseHandle(job);
            anyhow::bail!(
                "AssignProcessToJobObject failed: {}",
                std::io::Error::last_os_error()
            );
        }

        Ok(JobHandle(job))
    }
}

// ── ProcessController ─────────────────────────────────────────────────────────

pub struct ProcessController {
    child: tokio::process::Child,
    /// On Windows, holds the Job Object handle alive so `KILL_ON_JOB_CLOSE`
    /// terminates the worker process if `ProcessController` is dropped.
    #[cfg(windows)]
    _job: Option<JobHandle>,
}

#[async_trait]
impl RuntimeController for ProcessController {
    fn pid(&self) -> Option<i64> {
        self.child.id().map(|p| p as i64)
    }
    async fn kill(&mut self) -> Result<()> {
        self.child.kill().await.map_err(|e| e.into())
    }
    async fn wait(&mut self) -> Result<Option<i32>> {
        let s = self.child.wait().await?;
        Ok(s.code())
    }
}

pub struct InProcessController {
    // Shared join handle (inside a Mutex) returning Option<i32> exit code
    join: InProcessJoin,
    // We keep an interrupt handle optionally to support polite interruption (reserved)
    #[cfg(feature = "wasmtime-host")]
    #[allow(dead_code)]
    interrupt_handle: Option<Box<dyn std::any::Any + Send + Sync>>,
    #[cfg(not(feature = "wasmtime-host"))]
    #[allow(dead_code)]
    interrupt_handle: Option<Box<dyn std::any::Any + Send + Sync>>,
}

#[async_trait]
impl RuntimeController for InProcessController {
    fn pid(&self) -> Option<i64> {
        None
    }
    async fn kill(&mut self) -> Result<()> {
        // Best-effort: try polite interruption first (if available), then abort the join
        // (interrupt_handle reserved for future use)
        if let Ok(guard) = self.join.lock() {
            if let Some(h) = guard.as_ref() {
                h.abort();
            }
        }
        Ok(())
    }
    async fn wait(&mut self) -> Result<Option<i32>> {
        // Take the join handle out of the mutex and await it
        let handle_opt = {
            let mut guard = self.join.lock().unwrap();
            guard.take()
        };
        if let Some(h) = handle_opt {
            match h.await {
                Ok(res) => match res {
                    Ok(code_opt) => Ok(code_opt),
                    Err(e) => Err(e),
                },
                Err(e) => Err(e.into()),
            }
        } else {
            Ok(None)
        }
    }
}

impl RuntimeManager {
    pub fn new() -> Self {
        Self::default()
    }

    const DEFAULT_SKILL_MAX_CPU_SECONDS: u64 = 30;
    const DEFAULT_SKILL_MAX_MEMORY_MB: u64 = 512;

    /// Start a Python worker by spawning the given script path with the provided port.
    /// Returns a boxed `RuntimeController` that can be used to manage the runtime.
    pub async fn start_python_worker(
        &self,
        script_path: &str,
        port: u16,
    ) -> Result<Box<dyn RuntimeController + Send + Sync>> {
        let mut cmd = tokio::process::Command::new(resolve_python_binary());
        cmd.arg(script_path)
            .arg(port.to_string())
            .arg("--max-cpu-seconds")
            .arg(Self::DEFAULT_SKILL_MAX_CPU_SECONDS.to_string())
            .arg("--max-memory-mb")
            .arg(Self::DEFAULT_SKILL_MAX_MEMORY_MB.to_string());
        let child = cmd.spawn()?;

        #[cfg(windows)]
        let _job = {
            // Give the OS a moment to schedule the new process so its PID is valid.
            if let Some(pid) = child.id() {
                match create_job_for_pid(
                    pid,
                    Self::DEFAULT_SKILL_MAX_CPU_SECONDS,
                    Self::DEFAULT_SKILL_MAX_MEMORY_MB,
                ) {
                    Ok(job) => Some(job),
                    Err(e) => {
                        tracing::warn!("Windows Job Object setup failed for pid {pid}: {e}");
                        None
                    }
                }
            } else {
                None
            }
        };

        Ok(Box::new(ProcessController {
            child,
            #[cfg(windows)]
            _job,
        }))
    }

    /// Start a Babashka (Clojure) worker by spawning `bb` with the given script.
    pub async fn start_babashka_worker(
        &self,
        script_path: &str,
    ) -> Result<Box<dyn RuntimeController + Send + Sync>> {
        BB_VERSION_CHECK.call_once(|| {
            let required = std::env::var("BONSAI_REQUIRED_BB_VERSION").ok();
            warn_if_bb_version_mismatch(required.as_deref());
        });

        let script = std::path::PathBuf::from(script_path);
        let script_root = script.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        });
        let workspace_root = std::env::current_dir().unwrap_or_else(|_| script_root.clone());
        let allowed_paths =
            build_allowed_paths_env(&workspace_root, std::slice::from_ref(&script_root));

        let mut cmd = tokio::process::Command::new("bb");
        cmd.current_dir(&script_root)
            .env("BONSAI_ALLOWED_PATHS", &allowed_paths)
            .arg(script_path);

        if script
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case("bb_runner.clj"))
        {
            cmd.arg("--allowed-paths").arg(&allowed_paths);
        }

        let child = cmd.spawn()?;
        Ok(Box::new(ProcessController {
            child,
            #[cfg(windows)]
            _job: None,
        }))
    }

    /// Start a ClojureWasm (Wasm/WASI) module.
    /// If the crate feature `wasmtime-host` is enabled, this will run the module in-process
    /// via the `wasmtime` crate. Otherwise it will fall back to spawning the `wasmtime` CLI.
    pub async fn start_clojurewasm_worker(
        &self,
        module_path: &str,
        _timeout_secs: Option<u64>,
    ) -> Result<Box<dyn RuntimeController + Send + Sync>> {
        let p = Path::new(module_path);
        if !p.exists() {
            // fall back to trying to spawn CLI which may accept WAT files too
            let mut cmd = tokio::process::Command::new("wasmtime");
            cmd.arg(module_path);
            let child = cmd.spawn()?;
            return Ok(Box::new(ProcessController {
                child,
                #[cfg(windows)]
                _job: None,
            }));
        }

        // Try in-process wasmtime if feature is enabled
        #[cfg(feature = "wasmtime-host")]
        {
            use std::sync::{Arc, Mutex};
            use wasmtime::*;
            use wasmtime_wasi::sync::WasiCtxBuilder;

            let module_path = module_path.to_string();

            // Basic engine/config for in-process execution
            let mut config = Config::new();
            config.consume_fuel(true);
            config.epoch_interruption(true);
            let engine = Engine::new(&config)?;
            let module = Module::from_file(&engine, &module_path)?;

            // Shared join handle so watchdog can abort if needed
            let join_arc: Arc<Mutex<Option<tokio::task::JoinHandle<anyhow::Result<Option<i32>>>>>> =
                Arc::new(Mutex::new(None));

            let engine_for_task = engine.clone();
            let module_for_task = module.clone();
            let join_arc_clone = join_arc.clone();

            let join_handle =
                tokio::task::spawn_blocking(move || -> anyhow::Result<Option<i32>> {
                    // Build WASI context
                    let wasi_ctx = WasiCtxBuilder::new().inherit_stdio().build();

                    let mut store = Store::new(&engine_for_task, wasi_ctx);

                    // Add some fuel budget if requested
                    if let Some(sec) = _timeout_secs {
                        let _ = store.add_fuel(1_000_000_u64.saturating_mul(sec));
                    } else {
                        let _ = store.add_fuel(1_000_000_u64);
                    }

                    let mut linker = Linker::new(&engine_for_task);
                    wasmtime_wasi::add_to_linker(&mut linker, |cx: &mut _| cx).unwrap();

                    let instance = linker.instantiate(&mut store, &module_for_task)?;

                    // call _start if present
                    if let Some(start) = instance.get_func(&mut store, "_start") {
                        let typed = start.typed::<(), ()>(&mut store)?;
                        let _ = typed.call(&mut store, ())?;
                    }
                    Ok(Some(0))
                });

            // store join handle in Arc<Mutex<Option<JoinHandle>>> so we can abort/wait later
            {
                let mut guard = join_arc_clone.lock().unwrap();
                *guard = Some(join_handle);
            }

            // If timeout was requested, spawn a watchdog to increment engine epoch
            if let Some(sec) = _timeout_secs {
                let engine_watch = engine.clone();
                let join_watch = join_arc.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(sec)).await;
                    // trigger epoch increment to interrupt running stores
                    engine_watch.increment_epoch();
                    // give a grace period then abort the join as last resort
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    if let Ok(guard) = join_watch.lock() {
                        if let Some(h) = guard.as_ref() {
                            h.abort();
                        }
                    }
                });
            }

            // No real PID for in-process runs
            let controller = InProcessController {
                join: join_arc,
                #[cfg(feature = "wasmtime-host")]
                interrupt_handle: None,
                #[cfg(not(feature = "wasmtime-host"))]
                interrupt_handle: None,
            };
            return Ok(Box::new(controller));
        }

        // If feature is not enabled, spawn the CLI as fallback
        #[cfg(not(feature = "wasmtime-host"))]
        {
            let mut cmd = tokio::process::Command::new("wasmtime");
            cmd.arg(module_path);
            let child = cmd.spawn()?;
            Ok(Box::new(ProcessController {
                child,
                #[cfg(windows)]
                _job: None,
            }))
        }
    }
}

fn resolve_python_binary() -> String {
    if cfg!(target_os = "windows") {
        return "python".to_string();
    }

    if which::which("python3").is_ok() {
        "python3".to_string()
    } else {
        "python".to_string()
    }
}

fn build_allowed_paths_env(
    workspace_root: &Path,
    additional_paths: &[std::path::PathBuf],
) -> String {
    let mut unique = std::collections::BTreeSet::new();

    unique.insert(workspace_root.to_string_lossy().to_string());
    for p in additional_paths {
        unique.insert(p.to_string_lossy().to_string());
    }

    let sep = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };
    unique.into_iter().collect::<Vec<_>>().join(sep)
}

fn warn_if_bb_version_mismatch(required: Option<&str>) {
    let required = required.map(str::trim).filter(|v| !v.is_empty());
    let Some(required) = required else {
        return;
    };

    let output = std::process::Command::new("bb").arg("--version").output();
    match output {
        Ok(out) => {
            let installed = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !installed.contains(required) {
                eprintln!(
                    "[runtime] WARN: babashka version mismatch. required={required}, installed='{}'",
                    if installed.is_empty() { "unknown" } else { &installed }
                );
            }
        }
        Err(e) => {
            eprintln!("[runtime] WARN: unable to run 'bb --version' for version check: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_manager() {
        let m = RuntimeManager::new();
        let _ = m;
        assert!(true);
    }
}
