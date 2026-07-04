mod auth;
mod event_store;
mod grpc_server;
mod mesh;
mod model_router;
mod sandbox;
mod session_store;
mod substrate;
mod tool_registry;
mod vector_store;

use anyhow::Result;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "omniharness_kernel=info,warn".to_string()),
        )
        .init();

    info!("═══════════════════════════════════════════════════════");
    info!("  OmniHarness Kernel v1.0.0 — Century Protocol         ");
    info!("═══════════════════════════════════════════════════════");

    let storage_dir = std::env::var("OMNIHARNESS_DATA")
        .unwrap_or_else(|_| "data".to_string());
    std::fs::create_dir_all(&storage_dir)?;

    // ── Event Store ─────────────────────────────────────────────
    let event_log = format!("{}/events.jsonl", storage_dir);
    let event_store = Arc::new(
        event_store::PersistentEventStore::new(&event_log).await?,
    );
    info!("[BOOT] Event store ready. Tip: {}", event_store.current_tip().await);

    // ── Vector Store ─────────────────────────────────────────────
    let vec_path = format!("{}/vectors.jsonl", storage_dir);
    let vector_store = Arc::new(vector_store::VectorStore::new(&vec_path).await?);
    info!("[BOOT] Vector store ready.");

    // ── Model Registry ────────────────────────────────────────────
    let model_registry = Arc::new(model_router::ModelRegistry::new());
    model_registry.register_from_env();
    info!("[BOOT] Model registry ready. {} backends.", model_registry.len());

    // ── Session Store ─────────────────────────────────────────────
    let sess_path = format!("{}/sessions.jsonl", storage_dir);
    let session_store = Arc::new(session_store::SessionStore::new(&sess_path).await?);
    info!("[BOOT] Session store ready.");

    // ── Tool Registry ─────────────────────────────────────────────
    let tool_registry = Arc::new(tool_registry::ToolRegistry::new());
    tool_registry.register_builtins();
    info!("[BOOT] Tool registry ready. {} tools.", tool_registry.len());

    // ── Auth Store ────────────────────────────────────────────────
    let auth_path = format!("{}/auth.json", storage_dir);
    let auth_store = Arc::new(auth::AuthStore::new(&auth_path)?);
    info!("[BOOT] Auth store ready.");

    // ── Sandbox ───────────────────────────────────────────────────
    let sandbox = Arc::new(sandbox::Sandbox::new()?);
    info!("[BOOT] WASM sandbox primed.");

    // ── Record boot event ─────────────────────────────────────────
    event_store
        .append_event("kernel", "KernelBoot", r#"{"version":"1.0.0"}"#, "system")
        .await?;

    // ── gRPC server ───────────────────────────────────────────────
    let grpc_addr = std::env::var("GRPC_ADDR")
        .unwrap_or_else(|_| "[::1]:50051".to_string())
        .parse()?;

    let grpc_state = grpc_server::HarnessState {
        event_store:    Arc::clone(&event_store),
        model_registry: Arc::clone(&model_registry),
        vector_store:   Arc::clone(&vector_store),
        session_store:  Arc::clone(&session_store),
        tool_registry:  Arc::clone(&tool_registry),
        auth_store:     Arc::clone(&auth_store),
        sandbox:        Arc::clone(&sandbox),
        start_time:     std::time::Instant::now(),
    };

    let grpc_handle = tokio::spawn(async move {
        info!("[GRPC] Listening on {}", grpc_addr);
        if let Err(e) = grpc_server::serve(grpc_addr, grpc_state).await {
            error!("[GRPC] Fatal: {}", e);
        }
    });

    // ── Mesh node ─────────────────────────────────────────────────
    let (mesh_tx, mut mesh_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(1024);
    let mesh_handle = match mesh::MeshNode::new(mesh_tx) {
        Ok(node) => {
            info!("[MESH] P2P node initialized.");
            let es = Arc::clone(&event_store);
            let mh = tokio::spawn(async move { node.run().await });
            // consumer
            let es2 = Arc::clone(&es);
            tokio::spawn(async move {
                while let Some(data) = mesh_rx.recv().await {
                    if let Ok(ev) = serde_json::from_slice::<event_store::SystemEvent>(&data) {
                        match es2.append_external_event(ev.clone()).await {
                            Ok(_)  => info!("[MESH] Replicated event {}", ev.id),
                            Err(e) => error!("[MESH] Rejected: {}", e),
                        }
                    }
                }
            });
            Some(mh)
        }
        Err(e) => {
            error!("[MESH] Init failed (non-fatal): {}", e);
            None
        }
    };

    // ── Shutdown ──────────────────────────────────────────────────
    match signal::ctrl_c().await {
        Ok(())  => info!("[SHUTDOWN] Ctrl+C received — shutting down."),
        Err(e)  => error!("[SHUTDOWN] Signal error: {}", e),
    }

    grpc_handle.abort();
    if let Some(h) = mesh_handle { h.abort(); }

    event_store
        .append_event("kernel", "KernelShutdown", r#"{"clean":true}"#, "system")
        .await
        .ok();

    info!("[SHUTDOWN] OmniHarness Kernel stopped cleanly.");
    Ok(())
}
