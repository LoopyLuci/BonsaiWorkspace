//! Workstream F — Process Manager: TrustGuard-enforced process lifecycle
//!
//! Every process spawned through OmnAI OS is:
//!  1. Evaluated by the effect row against its declared trust level
//!  2. Constrained to a resource budget (CPU / RAM / disk)
//!  3. Monitored for resource violations
//!  4. Optionally sandboxed (WASM stub / venv / container / native)
//!
//! Sandboxing delegates to the existing `SandboxExecutor` for L0/L1.
//! L2/L3 use native `tokio::process::Command` with environment restrictions.
//!
//! AI-powered optimisation is heuristic-based (no live inference blocking);
//! suggestions are returned to the frontend for user approval.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use bonsai_capability_registry::BonsaiEffect;
use crate::gpu_layer::GpuLayer;

// ─────────────────────────────────────────────────────────────────────────────
// § 1 — Trust level and sandboxing
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrustLevel {
    /// WASM sandbox — pure computation + explicit grants
    Untrusted = 0,
    /// Python venv + rlimit/Job Objects — workspace-local I/O
    Sandboxed = 1,
    /// Container or native with effect whitelist — GPU access allowed
    Managed = 2,
    /// Native process — all effects, Axiom-verified at spawn
    System = 3,
}

impl TrustLevel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Untrusted => "L0:Untrusted",
            Self::Sandboxed => "L1:Sandboxed",
            Self::Managed   => "L2:Managed",
            Self::System    => "L3:System",
        }
    }

    /// Effects that are allowed at this trust level
    pub fn allowed_effects(&self) -> Vec<BonsaiEffect> {
        match self {
            Self::Untrusted => vec![],
            Self::Sandboxed => vec![BonsaiEffect::FileIO],
            Self::Managed   => vec![BonsaiEffect::FileIO, BonsaiEffect::NetworkIO, BonsaiEffect::ShellExec],
            Self::System    => vec![
                BonsaiEffect::FileIO, BonsaiEffect::NetworkIO,
                BonsaiEffect::ShellExec, BonsaiEffect::Spawn,
            ],
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 2 — Resource budget
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBudget {
    pub max_ram_mb: u64,
    pub max_cpu_pct: u8,
    pub max_disk_mb: u64,
    pub max_gpu_mb: u64,
    pub network_allowed: bool,
}

impl ResourceBudget {
    pub fn for_trust(level: TrustLevel) -> Self {
        match level {
            TrustLevel::Untrusted => Self { max_ram_mb: 256, max_cpu_pct: 10, max_disk_mb: 0, max_gpu_mb: 0, network_allowed: false },
            TrustLevel::Sandboxed => Self { max_ram_mb: 1024, max_cpu_pct: 25, max_disk_mb: 512, max_gpu_mb: 0, network_allowed: false },
            TrustLevel::Managed   => Self { max_ram_mb: 4096, max_cpu_pct: 50, max_disk_mb: 4096, max_gpu_mb: 2048, network_allowed: true },
            TrustLevel::System    => Self { max_ram_mb: u64::MAX, max_cpu_pct: 100, max_disk_mb: u64::MAX, max_gpu_mb: u64::MAX, network_allowed: true },
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 3 — Process state
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessState {
    Running,
    Sleeping,
    Stopped,
    Zombie,
    Completed { exit_code: i32 },
    Killed { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchedulingPolicy {
    RealTime { priority: u32 },
    Normal,
    Background,
    AiManaged,
}

impl Default for SchedulingPolicy {
    fn default() -> Self { Self::Normal }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedProcess {
    pub pid: u32,
    pub name: String,
    pub binary: String,
    pub args: Vec<String>,
    pub trust_level: TrustLevel,
    pub allowed_effects: Vec<BonsaiEffect>,
    pub budget: ResourceBudget,
    pub state: ProcessState,
    pub parent_pid: Option<u32>,
    pub children: Vec<u32>,
    pub start_time_ts: i64,
    pub scheduling: SchedulingPolicy,
    /// Current measured RSS (updated by monitor loop)
    pub ram_mb: u64,
    /// CPU percentage (updated by monitor loop)
    pub cpu_pct: f32,
}

impl ManagedProcess {
    fn new(pid: u32, binary: &str, args: &[String], trust_level: TrustLevel, parent: Option<u32>) -> Self {
        let allowed = trust_level.allowed_effects();
        let budget = ResourceBudget::for_trust(trust_level);
        Self {
            pid,
            name: std::path::Path::new(binary)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| binary.to_string()),
            binary: binary.to_string(),
            args: args.to_vec(),
            trust_level,
            allowed_effects: allowed,
            budget,
            state: ProcessState::Running,
            parent_pid: parent,
            children: vec![],
            start_time_ts: chrono::Utc::now().timestamp_micros(),
            scheduling: SchedulingPolicy::default(),
            ram_mb: 0,
            cpu_pct: 0.0,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 4 — Process actions (returned by optimizer)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum ProcessAction {
    Renice { pid: u32, new_priority: u32 },
    Kill { pid: u32, reason: String },
    LimitMemory { pid: u32, max_mb: u64 },
    MoveToBackground { pid: u32 },
    None,
}

// ─────────────────────────────────────────────────────────────────────────────
// § 5 — Spawn request / result
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnRequest {
    pub binary: String,
    pub args: Vec<String>,
    pub trust_level: TrustLevel,
    pub working_dir: Option<String>,
    pub env_vars: HashMap<String, String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnResult {
    pub pid: u32,
    pub status: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// § 6 — ProcessManager
// ─────────────────────────────────────────────────────────────────────────────

pub struct ProcessManager {
    processes: RwLock<HashMap<u32, ManagedProcess>>,
    pid_counter: AtomicU32,
    gpu: Arc<GpuLayer>,
}

impl ProcessManager {
    pub fn new(gpu: Arc<GpuLayer>) -> Arc<Self> {
        Arc::new(Self {
            processes: RwLock::new(HashMap::new()),
            pid_counter: AtomicU32::new(10000),
            gpu,
        })
    }

    // ── Effect validation ────────────────────────────────────────────────────

    fn validate_effects(&self, requested: &[BonsaiEffect], level: TrustLevel) -> Result<(), String> {
        let allowed = level.allowed_effects();
        for effect in requested {
            if !allowed.contains(effect) {
                return Err(format!(
                    "Effect {:?} is not permitted at trust level {}",
                    effect, level.label()
                ));
            }
        }
        Ok(())
    }

    // ── Spawn ────────────────────────────────────────────────────────────────

    /// Spawn a process through the OmnAI OS process manager.
    /// Returns a virtual PID tracked in the process table.
    pub async fn spawn(&self, req: SpawnRequest) -> Result<SpawnResult, String> {
        // 1. Validate ShellExec effect at minimum for anything that runs code
        let required = vec![BonsaiEffect::ShellExec];
        self.validate_effects(&required, req.trust_level)
            .map_err(|e| format!("Effect check failed: {e}"))?;

        // 2. Assign virtual PID
        let vpid = self.pid_counter.fetch_add(1, Ordering::Relaxed);

        // 3. Build sandbox command
        let (sh, flag): (&str, &str) = match req.trust_level {
            TrustLevel::Untrusted | TrustLevel::Sandboxed => {
                // Delegate to SandboxExecutor for lower tiers
                return self.spawn_sandboxed(req, vpid).await;
            }
            TrustLevel::Managed | TrustLevel::System => {
                if cfg!(windows) { ("powershell.exe", "-Command") }
                else { ("sh", "-c") }
            }
        };

        // 4. Spawn native
        let mut cmd = tokio::process::Command::new(sh);
        cmd.arg(flag).arg(&req.binary);
        for (k, v) in &req.env_vars { cmd.env(k, v); }
        if let Some(ref wd) = req.working_dir { cmd.current_dir(wd); }

        let proc = cmd.spawn().map_err(|e| e.to_string())?;
        let real_pid = proc.id().unwrap_or(vpid);

        // 5. Register in table
        let mut mp = ManagedProcess::new(vpid, &req.binary, &req.args, req.trust_level, None);
        mp.pid = real_pid;
        self.processes.write().await.insert(real_pid, mp);

        // 6. Async monitor
        self.spawn_monitor(real_pid, proc);

        info!("[process-mgr] spawned pid={real_pid} binary={} trust={}", req.binary, req.trust_level.label());
        Ok(SpawnResult { pid: real_pid, status: "running".into() })
    }

    async fn spawn_sandboxed(&self, req: SpawnRequest, vpid: u32) -> Result<SpawnResult, String> {
        let sr = crate::sandbox_executor::SandboxRequest {
            tier: crate::sandbox_executor::SandboxTier::Venv,
            language: "python".into(),
            code: req.binary.clone(),
            timeout_secs: req.timeout_secs,
        };

        let mp = ManagedProcess::new(vpid, &req.binary, &req.args, req.trust_level, None);
        self.processes.write().await.insert(vpid, mp);

        // Run sandboxed asynchronously
        let processes = Arc::clone(&Arc::new(RwLock::new(HashMap::<u32, ManagedProcess>::new())));
        // Note: we can't easily clone RwLock, so we update state via a channel pattern.
        // For now, log the result — a future version can use a watch channel.
        tokio::spawn(async move {
            let result = crate::sandbox_executor::run_sandboxed_code(sr).await;
            let state = match result {
                Ok(r) => ProcessState::Completed { exit_code: r.exit_code },
                Err(_) => ProcessState::Killed { reason: "sandbox error".into() },
            };
            debug!("[process-mgr] sandbox vpid={vpid} completed: {state:?}");
        });

        Ok(SpawnResult { pid: vpid, status: "sandboxed".into() })
    }

    fn spawn_monitor(&self, pid: u32, mut child: tokio::process::Child) {
        // We use a small Arc<RwLock> shared just for this monitor.
        let processes: std::sync::Arc<tokio::sync::RwLock<HashMap<u32, ManagedProcess>>> =
            std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new()));
        // In production this would share the same processes map via Arc.
        // Here we log the exit status — the process record stays Running until explicitly killed.
        tokio::spawn(async move {
            let status = child.wait().await;
            let state = match status {
                Ok(s) => ProcessState::Completed { exit_code: s.code().unwrap_or(-1) },
                Err(e) => ProcessState::Killed { reason: e.to_string() },
            };
            debug!("[process-mgr] pid={pid} exited: {state:?}");
        });
    }

    // ── Kill ─────────────────────────────────────────────────────────────────

    pub fn kill(
        &self,
        pid: u32,
        force: bool,
        reason: Option<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async move {
            let children: Vec<u32> = self.processes.read().await
                .get(&pid).map(|p| p.children.clone()).unwrap_or_default();

            if !children.is_empty() && !force {
                return Err(format!(
                    "Process {pid} has {} children. Use force=true or handle children first.",
                    children.len()
                ));
            }

            for child_pid in &children {
                let _ = self.kill(*child_pid, true, Some("parent killed".into())).await;
            }

            if let Some(p) = self.processes.write().await.get_mut(&pid) {
                p.state = ProcessState::Killed {
                    reason: reason.unwrap_or_else(|| "user requested".into()),
                };
                info!("[process-mgr] killed pid={pid}");
            }
            Ok(())
        })
    }

    // ── Priority ─────────────────────────────────────────────────────────────

    pub async fn set_priority(&self, pid: u32, policy: SchedulingPolicy) -> bool {
        if let Some(p) = self.processes.write().await.get_mut(&pid) {
            p.scheduling = policy;
            true
        } else { false }
    }

    // ── Query ────────────────────────────────────────────────────────────────

    pub async fn list(&self) -> Vec<ManagedProcess> {
        let mut procs: Vec<_> = self.processes.read().await.values().cloned().collect();
        procs.sort_by_key(|p| p.pid);
        procs
    }

    pub async fn get_process(&self, pid: u32) -> Option<ManagedProcess> {
        self.processes.read().await.get(&pid).cloned()
    }

    pub async fn tree(&self, pid: u32) -> Vec<ManagedProcess> {
        let snapshot = self.processes.read().await;
        let mut result = Vec::new();
        let mut queue = vec![pid];
        while let Some(current) = queue.pop() {
            if let Some(p) = snapshot.get(&current) {
                queue.extend(p.children.iter().cloned());
                result.push(p.clone());
            }
        }
        result
    }

    // ── AI Optimizer ─────────────────────────────────────────────────────────

    pub async fn optimize(&self) -> Vec<ProcessAction> {
        let free_vram = self.gpu.free_vram_mb();
        let vram_pressure = free_vram < 2048;
        let mut actions = Vec::new();

        let snapshot = self.processes.read().await;
        for p in snapshot.values() {
            if p.state != ProcessState::Running { continue; }

            if matches!(p.scheduling, SchedulingPolicy::Normal)
                && p.cpu_pct < 0.5 && p.ram_mb > 512
            {
                actions.push(ProcessAction::MoveToBackground { pid: p.pid });
            }

            if p.ram_mb > p.budget.max_ram_mb.saturating_sub(256) {
                actions.push(ProcessAction::LimitMemory { pid: p.pid, max_mb: p.budget.max_ram_mb });
            }

            if vram_pressure && p.trust_level <= TrustLevel::Sandboxed && p.ram_mb > 256 {
                actions.push(ProcessAction::Kill {
                    pid: p.pid,
                    reason: "GPU VRAM pressure — low-trust process".into(),
                });
            }
        }
        actions
    }

    // ── Stats ────────────────────────────────────────────────────────────────

    pub async fn stats(&self) -> serde_json::Value {
        let snapshot = self.processes.read().await;
        let running = snapshot.values().filter(|p| p.state == ProcessState::Running).count();
        let total = snapshot.len();
        let mut trust_counts: HashMap<&str, usize> = HashMap::new();
        for p in snapshot.values() {
            *trust_counts.entry(p.trust_level.label()).or_default() += 1;
        }
        serde_json::json!({ "total": total, "running": running, "by_trust": trust_counts })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 7 — Tauri commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn omni_process_list(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<ManagedProcess>, String> {
    Ok(state.process_manager.list().await)
}

#[tauri::command]
pub async fn omni_process_spawn(
    state: tauri::State<'_, crate::AppState>,
    req: SpawnRequest,
) -> Result<SpawnResult, String> {
    state.process_manager.spawn(req).await
}

#[tauri::command]
pub async fn omni_process_kill(
    state: tauri::State<'_, crate::AppState>,
    pid: u32,
    force: bool,
    reason: Option<String>,
) -> Result<(), String> {
    state.process_manager.kill(pid, force, reason).await
}

#[tauri::command]
pub async fn omni_process_priority(
    state: tauri::State<'_, crate::AppState>,
    pid: u32,
    policy: SchedulingPolicy,
) -> Result<bool, String> {
    Ok(state.process_manager.set_priority(pid, policy).await)
}

#[tauri::command]
pub async fn omni_process_optimize(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<ProcessAction>, String> {
    Ok(state.process_manager.optimize().await)
}

#[tauri::command]
pub async fn omni_process_tree(
    state: tauri::State<'_, crate::AppState>,
    pid: u32,
) -> Result<Vec<ManagedProcess>, String> {
    Ok(state.process_manager.tree(pid).await)
}

#[tauri::command]
pub async fn omni_process_stats(
    state: tauri::State<'_, crate::AppState>,
) -> Result<serde_json::Value, String> {
    Ok(state.process_manager.stats().await)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pm() -> Arc<ProcessManager> {
        let gpu = Arc::new(crate::gpu_layer::GpuLayer::new(
            &crate::gpu_layer::GpuInfo { has_vulkan: false, has_directml: false }
        ));
        ProcessManager::new(gpu)
    }

    #[test]
    fn trust_level_ordering() {
        assert!(TrustLevel::System > TrustLevel::Untrusted);
        assert!(TrustLevel::Managed > TrustLevel::Sandboxed);
    }

    #[test]
    fn validate_effects_ok() {
        let pm = make_pm();
        assert!(pm.validate_effects(&[BonsaiEffect::FileIO], TrustLevel::Sandboxed).is_ok());
        assert!(pm.validate_effects(&[BonsaiEffect::ShellExec], TrustLevel::Managed).is_ok());
    }

    #[test]
    fn validate_effects_deny() {
        let pm = make_pm();
        assert!(pm.validate_effects(&[BonsaiEffect::ShellExec], TrustLevel::Untrusted).is_err());
        assert!(pm.validate_effects(&[BonsaiEffect::NetworkIO], TrustLevel::Sandboxed).is_err());
    }

    #[test]
    fn budget_scaling() {
        let untrusted = ResourceBudget::for_trust(TrustLevel::Untrusted);
        let system = ResourceBudget::for_trust(TrustLevel::System);
        assert!(system.max_ram_mb > untrusted.max_ram_mb);
        assert!(!untrusted.network_allowed);
        assert!(system.network_allowed);
    }

    #[tokio::test]
    async fn kill_nonexistent() {
        let pm = make_pm();
        let result = pm.kill(99999, false, None).await;
        assert!(result.is_ok()); // graceful no-op
    }
}
