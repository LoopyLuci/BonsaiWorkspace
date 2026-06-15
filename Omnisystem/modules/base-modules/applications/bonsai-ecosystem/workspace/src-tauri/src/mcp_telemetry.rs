use std::sync::Arc;
use serde_json::Value;
use tokio::sync::broadcast;
use crate::system_event_bus::SystemEventBus;

pub async fn start_mcp_server(event_bus: Arc<SystemEventBus>) {
    // Channel to send JSON-serialized events into the MCP server
    let (tx, _rx) = broadcast::channel::<Value>(1024);

    // Start the MCP server in background
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        if let Err(e) = mcp_server::server::run_server_with_event_tx("127.0.0.1", 11425, tx_clone).await {
            tracing::error!("MCP server failed: {}", e);
        }
    });

    // Forward SystemEventBus events into the MCP server channel
    let mut rx = event_bus.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            match serde_json::to_value(&ev) {
                Ok(v) => { let _ = tx.send(v); }
                Err(e) => tracing::warn!("serialize event failed: {}", e),
            }
        }
    });
}
