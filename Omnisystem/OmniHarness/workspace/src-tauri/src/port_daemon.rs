//! Centralized port allocation + health-supervised auto-restart for every
//! internal Omnisystem/OmniHarness network service (API server, buddy API,
//! in-process MCP server, A2A agent server, ...).
//!
//! Individual servers keep their own bind/serve logic (`api_server::start`,
//! `buddy_api_server::start`, ...) — this daemon adds two things none of them
//! had on their own:
//!   1. A genuinely wide port probe (`find_free_port`). Each server used to
//!      fall back across only its preferred port +0..+4/+5, which is nowhere
//!      near enough to escape a Windows-reserved/excluded port range (these
//!      can be hundreds of ports wide, e.g. Hyper-V/WSL reservations) — every
//!      fallback attempt landed inside the same excluded band and failed the
//!      same way. Probing forward across a few thousand candidates costs
//!      nothing (a failed bind returns immediately) and reliably finds real
//!      free space on the other side.
//!   2. A live registry + health-watch loop so a service that dies after
//!      startup (not just one that fails to bind) gets noticed and restarted
//!      with exponential backoff, instead of silently staying down forever.
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceState {
    Starting,
    Running,
    Failed,
    Restarting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub preferred_port: u16,
    pub bound_port: Option<u16>,
    pub state: ServiceState,
    pub restart_count: u32,
    pub last_error: Option<String>,
}

pub struct PortDaemon {
    services: RwLock<HashMap<String, ServiceStatus>>,
}

impl PortDaemon {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            services: RwLock::new(HashMap::new()),
        })
    }

    /// Probe-bind candidate ports starting at `preferred`, scanning up to
    /// `max_scan` ports forward. A bind failure — whether AddrInUse or a
    /// Windows-excluded-range PermissionDenied — just moves on to the next
    /// candidate, so this survives being placed inside a wide OS-reserved band.
    pub async fn find_free_port(preferred: u16, max_scan: u16) -> Option<u16> {
        for offset in 0..=max_scan {
            let candidate = preferred.checked_add(offset)?;
            if let Ok(listener) = tokio::net::TcpListener::bind(("127.0.0.1", candidate)).await {
                drop(listener);
                return Some(candidate);
            }
        }
        None
    }

    pub async fn register(&self, name: &str, preferred_port: u16) {
        self.services.write().await.insert(
            name.to_string(),
            ServiceStatus {
                name: name.to_string(),
                preferred_port,
                bound_port: None,
                state: ServiceState::Starting,
                restart_count: 0,
                last_error: None,
            },
        );
    }

    pub async fn mark_bound(&self, name: &str, port: u16) {
        if let Some(s) = self.services.write().await.get_mut(name) {
            s.bound_port = Some(port);
            s.state = ServiceState::Running;
            s.last_error = None;
        }
    }

    pub async fn mark_failed(&self, name: &str, error: String) {
        if let Some(s) = self.services.write().await.get_mut(name) {
            s.state = ServiceState::Failed;
            s.last_error = Some(error);
        }
    }

    pub async fn mark_restarting(&self, name: &str) {
        if let Some(s) = self.services.write().await.get_mut(name) {
            s.state = ServiceState::Restarting;
            s.restart_count += 1;
        }
    }

    pub async fn snapshot(&self) -> Vec<ServiceStatus> {
        let mut v: Vec<ServiceStatus> = self.services.read().await.values().cloned().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    /// Poll `health_url` every `check_every`; if it stops responding
    /// successfully, mark the service restarting and call `restart` (which
    /// should re-bind, possibly on a new port, and relaunch the server).
    /// Backoff doubles on repeated restart failures, capped at 60s.
    pub fn watch_health<F, Fut>(
        self: &Arc<Self>,
        name: &str,
        health_url: String,
        check_every: Duration,
        restart: F,
    ) where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        let daemon = self.clone();
        let name = name.to_string();
        tauri::async_runtime::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(3))
                .build()
                .unwrap_or_default();
            let mut backoff = Duration::from_secs(2);
            loop {
                tokio::time::sleep(check_every).await;
                let healthy = client
                    .get(&health_url)
                    .send()
                    .await
                    .map(|r| r.status().is_success())
                    .unwrap_or(false);
                if healthy {
                    backoff = Duration::from_secs(2);
                    continue;
                }
                tracing::warn!("[port-daemon] {name} unhealthy — restarting");
                daemon.mark_restarting(&name).await;
                match restart().await {
                    Ok(()) => {
                        tracing::info!("[port-daemon] {name} restarted");
                        backoff = Duration::from_secs(2);
                    }
                    Err(e) => {
                        tracing::error!("[port-daemon] {name} restart failed: {e}");
                        daemon.mark_failed(&name, e).await;
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(Duration::from_secs(60));
                    }
                }
            }
        });
    }
}

#[tauri::command]
pub async fn port_daemon_status(
    daemon: tauri::State<'_, Arc<PortDaemon>>,
) -> Result<Vec<ServiceStatus>, String> {
    Ok(daemon.snapshot().await)
}
