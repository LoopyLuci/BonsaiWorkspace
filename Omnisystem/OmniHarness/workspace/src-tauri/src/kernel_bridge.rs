//! Optional gRPC bridge to the OmniHarness Rust kernel (`../../kernel`,
//! `omniharness-kernel.exe`, port 50051) — the cross-language trust anchor
//! (hash-chained event store, model registry, harness status) shared with
//! the Python orchestrator and Clojure orchestrator (see
//! `../../proto/omniharness.proto`). The kernel is a separate, independently
//! launched process (`OmniHarness/start.ps1`); everything here degrades to a
//! silent no-op when it isn't running, mirroring the same graceful-degrade
//! pattern the Python orchestrator's `grpc_client.py` already uses.
//!
//! This gives workspace's own audit trail (`assistant_audit_log.rs`) a
//! second, cross-process, SHA-256 hash-chained home that the orchestrator
//! and Clojure side can independently verify and query — instead of
//! workspace's governance/audit primitives (crates/audit-log, crates/credits,
//! crates/capability-registry) staying invisible to the rest of OmniHarness.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, RwLock};
use tonic::transport::Channel;

pub mod proto {
    tonic::include_proto!("omniharness.v1");
}

use proto::{
    event_store_service_client::EventStoreServiceClient,
    harness_service_client::HarnessServiceClient,
    model_service_client::ModelServiceClient,
    AppendRequest, ListModelsRequest, ModelInfo, StatusRequest, StatusResponse,
};

// "localhost" (not a literal 127.0.0.1) so the OS resolver picks whichever
// of IPv4/IPv6 loopback the kernel actually bound — it defaults to the IPv6
// loopback `[::1]:50051` (see kernel/src/main.rs's GRPC_ADDR default), which
// a literal 127.0.0.1 would silently fail to reach.
const KERNEL_ADDR: &str = "http://localhost:50051";

/// Wraps a request with the `x-omniharness-key` metadata header when
/// `OMNIHARNESS_ADMIN_KEY` is set — a no-op (and harmless) when the kernel
/// is running with its default `OMNIHARNESS_REQUIRE_AUTH` unset, only
/// actually required once that opt-in enforcement is turned on.
fn authed<T>(payload: T) -> tonic::Request<T> {
    let mut req = tonic::Request::new(payload);
    if let Ok(key) = std::env::var("OMNIHARNESS_ADMIN_KEY") {
        if let Ok(val) = key.parse() {
            req.metadata_mut().insert("x-omniharness-key", val);
        }
    }
    req
}
const RECONNECT_INTERVAL: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(800);

/// One workspace-side event to mirror into the kernel's hash-chained event store.
pub struct KernelMirrorEvent {
    pub module_source: String,
    pub event_type: String,
    pub payload_json: String,
    pub session_id: String,
}

pub struct KernelBridge {
    channel: Arc<RwLock<Option<Channel>>>,
    mirror_tx: mpsc::UnboundedSender<KernelMirrorEvent>,
}

impl KernelBridge {
    /// Starts the background reconnect loop and the audit-mirror drain loop.
    /// Must be called from within a Tauri async context (uses
    /// `tauri::async_runtime::spawn`, not raw `tokio::spawn`, since Tauri's
    /// `.setup()` hook is not itself a tokio task).
    pub fn spawn() -> Self {
        let channel: Arc<RwLock<Option<Channel>>> = Arc::new(RwLock::new(None));
        let (mirror_tx, mirror_rx) = mpsc::unbounded_channel::<KernelMirrorEvent>();

        {
            let channel = channel.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    if channel.read().await.is_none() {
                        if let Ok(endpoint) = Channel::from_static(KERNEL_ADDR)
                            .connect_timeout(CONNECT_TIMEOUT)
                            .timeout(Duration::from_secs(10))
                            .connect()
                            .await
                        {
                            tracing::info!(
                                "[kernel_bridge] connected to OmniHarness kernel at {KERNEL_ADDR}"
                            );
                            *channel.write().await = Some(endpoint);
                        }
                    }
                    tokio::time::sleep(RECONNECT_INTERVAL).await;
                }
            });
        }

        {
            let channel = channel.clone();
            tauri::async_runtime::spawn(Self::drain_mirror_events(mirror_rx, channel));
        }

        KernelBridge { channel, mirror_tx }
    }

    async fn drain_mirror_events(
        mut rx: mpsc::UnboundedReceiver<KernelMirrorEvent>,
        channel: Arc<RwLock<Option<Channel>>>,
    ) {
        while let Some(ev) = rx.recv().await {
            let Some(ch) = channel.read().await.clone() else {
                continue; // kernel not connected — drop this event, no queueing/backpressure
            };
            let mut client = EventStoreServiceClient::new(ch);
            let req = authed(AppendRequest {
                module_source: ev.module_source,
                event_type: ev.event_type,
                payload_json: ev.payload_json,
                session_id: ev.session_id,
            });
            if client.append_event(req).await.is_err() {
                // Connection is likely dead; drop it so the reconnect loop retries.
                *channel.write().await = None;
            }
        }
    }

    /// Sender workspace subsystems can clone to mirror events into the
    /// kernel's audit chain (see `assistant_audit_log::AuditLog::attach_kernel_bridge`).
    pub fn mirror_sender(&self) -> mpsc::UnboundedSender<KernelMirrorEvent> {
        self.mirror_tx.clone()
    }

    pub async fn is_connected(&self) -> bool {
        self.channel.read().await.is_some()
    }

    pub async fn status(&self) -> Option<StatusResponse> {
        let ch = self.channel.read().await.clone()?;
        let mut client = HarnessServiceClient::new(ch);
        match client.status(authed(StatusRequest {})).await {
            Ok(r) => Some(r.into_inner()),
            Err(_) => {
                *self.channel.write().await = None;
                None
            }
        }
    }

    pub async fn list_models(&self, provider: &str) -> Vec<ModelInfo> {
        let Some(ch) = self.channel.read().await.clone() else {
            return vec![];
        };
        let mut client = ModelServiceClient::new(ch);
        client
            .list_models(authed(ListModelsRequest {
                provider: provider.to_string(),
            }))
            .await
            .map(|r| r.into_inner().models)
            .unwrap_or_default()
    }
}
