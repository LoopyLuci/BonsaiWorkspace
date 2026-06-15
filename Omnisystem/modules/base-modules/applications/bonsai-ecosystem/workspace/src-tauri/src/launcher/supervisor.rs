//! Launch supervisor: probes services in dependency order and emits progress
//! events to the frontend. No processes are spawned here — Bonsai's servers
//! are started by lib.rs; the supervisor only verifies they are healthy.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::Emitter;
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout};
use tracing::{info, warn};

use super::component::{ComponentSpec, ComponentState};

// ── Public status snapshot ────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct LaunchStatus {
    pub all_ready: bool,
    pub components: Vec<ComponentProgress>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ComponentProgress {
    pub name: String,
    pub state: ComponentState,
    pub message: String,
}

// ── Supervisor ────────────────────────────────────────────────────────────────

pub struct LaunchSupervisor {
    specs: Vec<ComponentSpec>,
    states: Arc<RwLock<HashMap<String, ComponentState>>>,
}

impl LaunchSupervisor {
    pub fn new(specs: Vec<ComponentSpec>) -> Self {
        let mut states = HashMap::new();
        for s in &specs {
            states.insert(s.name.clone(), ComponentState::Pending);
        }
        Self {
            specs,
            states: Arc::new(RwLock::new(states)),
        }
    }

    pub async fn status(&self) -> LaunchStatus {
        let states = self.states.read().await;
        let components: Vec<ComponentProgress> = self
            .specs
            .iter()
            .map(|s| {
                let state = states
                    .get(&s.name)
                    .cloned()
                    .unwrap_or(ComponentState::Pending);
                let message = match &state {
                    ComponentState::Pending => "waiting for dependencies".into(),
                    ComponentState::Starting => format!("probing port {}", s.health_port),
                    ComponentState::Ready => "ready".into(),
                    ComponentState::Failed(e) => e.clone(),
                    ComponentState::Skipped => "skipped (optional)".into(),
                };
                ComponentProgress {
                    name: s.name.clone(),
                    state,
                    message,
                }
            })
            .collect();
        let all_ready = components
            .iter()
            .all(|c| matches!(c.state, ComponentState::Ready | ComponentState::Skipped));
        LaunchStatus {
            all_ready,
            components,
        }
    }

    /// Run the full probe sequence. Returns Ok when all required components are ready.
    /// Emits `bonsai:launch-progress` events via `app_handle` if provided.
    pub async fn probe_all(
        self: Arc<Self>,
        app_handle: Option<tauri::AppHandle>,
    ) -> Result<(), String> {
        // Topo order: process specs ensuring deps come first.
        let order = topo_sort(&self.specs)?;

        for name in &order {
            let spec = self.specs.iter().find(|s| &s.name == name).unwrap();

            // Wait for all deps to be Ready.
            self.await_dependencies(spec).await?;

            // Set Starting state.
            self.set_state(&spec.name, ComponentState::Starting).await;
            self.emit_progress(&app_handle).await;

            // Probe with retries.
            let result = self.probe_component(spec).await;
            match result {
                Ok(()) => {
                    self.set_state(&spec.name, ComponentState::Ready).await;
                    info!(component=%spec.name, "[launcher] ready");
                }
                Err(e) => {
                    if spec.required {
                        self.set_state(&spec.name, ComponentState::Failed(e.clone()))
                            .await;
                        self.emit_progress(&app_handle).await;
                        return Err(format!("Required component '{}' failed: {}", spec.name, e));
                    } else {
                        warn!(component=%spec.name, error=%e, "[launcher] optional component not ready, skipping");
                        self.set_state(&spec.name, ComponentState::Skipped).await;
                    }
                }
            }
            self.emit_progress(&app_handle).await;
        }
        Ok(())
    }

    async fn probe_component(&self, spec: &ComponentSpec) -> Result<(), String> {
        let mut last_err = String::new();
        for attempt in 0..=spec.retries {
            if attempt > 0 {
                sleep(Duration::from_millis(spec.retry_delay_ms)).await;
            }
            match probe_health(spec).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_err = e;
                    warn!(component=%spec.name, attempt, error=%last_err, "[launcher] probe failed");
                }
            }
        }
        Err(last_err)
    }

    async fn await_dependencies(&self, spec: &ComponentSpec) -> Result<(), String> {
        if spec.dependencies.is_empty() {
            return Ok(());
        }
        // Poll until all deps are terminal.
        loop {
            let states = self.states.read().await;
            let mut all_ok = true;
            for dep in &spec.dependencies {
                match states.get(dep) {
                    Some(ComponentState::Ready) => {}
                    Some(ComponentState::Skipped) => {}
                    Some(ComponentState::Failed(e)) => {
                        return Err(format!("Dependency '{dep}' failed: {e}"));
                    }
                    _ => {
                        all_ok = false;
                    }
                }
            }
            if all_ok {
                return Ok(());
            }
            drop(states);
            sleep(Duration::from_millis(200)).await;
        }
    }

    async fn set_state(&self, name: &str, state: ComponentState) {
        self.states.write().await.insert(name.to_string(), state);
    }

    async fn emit_progress(&self, app_handle: &Option<tauri::AppHandle>) {
        if let Some(h) = app_handle {
            let status = self.status().await;
            let _ = h.emit("bonsai:launch-progress", &status);
        }
    }

    /// Run a continuous background health-check loop after startup.
    /// When a previously-Ready component fails its health probe, emits
    /// `bonsai:service-lost` and marks it Failed so the frontend can show
    /// a reconnecting overlay.
    pub async fn monitor(self: Arc<Self>, app_handle: tauri::AppHandle, interval: Duration) {
        loop {
            sleep(interval).await;

            let states = self.states.read().await;
            let ready_names: Vec<String> = states
                .iter()
                .filter(|(_, s)| matches!(s, ComponentState::Ready))
                .map(|(n, _)| n.clone())
                .collect();
            drop(states);

            for name in ready_names {
                let spec = match self.specs.iter().find(|s| s.name == name) {
                    Some(s) => s,
                    None => continue,
                };
                if probe_health(spec).await.is_err() {
                    warn!(component=%name, "[launcher] component became unhealthy");
                    self.set_state(&name, ComponentState::Failed("health check failed".into()))
                        .await;
                    let _ = app_handle.emit(
                        "bonsai:service-lost",
                        serde_json::json!({ "component": name }),
                    );
                }
            }
        }
    }
}

// ── Health probe ──────────────────────────────────────────────────────────────

async fn probe_health(spec: &ComponentSpec) -> Result<(), String> {
    let port = spec.health_port;
    let deadline = Duration::from_secs(spec.timeout_secs);

    // Always check TCP first.
    let tcp_ok = timeout(deadline, wait_tcp(port)).await.unwrap_or(false);
    if !tcp_ok {
        return Err(format!(
            "port {} not reachable within {}s",
            port, spec.timeout_secs
        ));
    }

    // Optionally check HTTP.
    if let Some(url) = &spec.health_url {
        let http_ok = timeout(deadline, wait_http(url)).await.unwrap_or(false);
        if !http_ok {
            return Err(format!("HTTP health check failed for {url}"));
        }
    }
    Ok(())
}

async fn wait_tcp(port: u16) -> bool {
    loop {
        if tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .is_ok()
        {
            return true;
        }
        sleep(Duration::from_millis(250)).await;
    }
}

async fn wait_http(url: &str) -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_default();
    loop {
        if let Ok(r) = client.get(url).send().await {
            if r.status().is_success() {
                return true;
            }
        }
        sleep(Duration::from_millis(500)).await;
    }
}

// ── Topological sort ──────────────────────────────────────────────────────────

fn topo_sort(specs: &[ComponentSpec]) -> Result<Vec<String>, String> {
    let mut order = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let mut in_progress = std::collections::HashSet::new();

    fn visit(
        name: &str,
        specs: &[ComponentSpec],
        visited: &mut std::collections::HashSet<String>,
        in_progress: &mut std::collections::HashSet<String>,
        order: &mut Vec<String>,
    ) -> Result<(), String> {
        if visited.contains(name) {
            return Ok(());
        }
        if in_progress.contains(name) {
            return Err(format!("Circular dependency involving '{name}'"));
        }
        in_progress.insert(name.to_string());
        if let Some(spec) = specs.iter().find(|s| s.name == name) {
            for dep in &spec.dependencies {
                visit(dep, specs, visited, in_progress, order)?;
            }
        }
        in_progress.remove(name);
        visited.insert(name.to_string());
        order.push(name.to_string());
        Ok(())
    }

    for spec in specs {
        visit(
            &spec.name,
            specs,
            &mut visited,
            &mut in_progress,
            &mut order,
        )?;
    }
    Ok(order)
}
