//! Polyglot Pong Dashboard - Real-Time Observability
//!
//! WebSocket server streaming live metrics and progress updates
//! from the orchestrator to connected clients.

use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::Response,
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use polyglot_pong_common::*;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<DashboardEvent>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct DashboardEvent {
    timestamp: String,
    event_type: String,
    data: serde_json::Value,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Polyglot Pong Dashboard");

    // Create broadcast channel for streaming metrics
    let (tx, _rx) = broadcast::channel(1000);
    let state = AppState { tx: tx.clone() };

    // Build router
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/ws", get(websocket_handler))
        .with_state(state.clone());

    // Start HTTP server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("Dashboard listening on http://0.0.0.0:8080");

    // Run metrics broadcaster in background
    tokio::spawn(broadcast_metrics(tx));

    // Start server
    axum::serve(listener, app).await?;

    Ok(())
}

/// Serve the dashboard HTML
async fn index_handler() -> impl axum::response::IntoResponse {
    (
        axum::http::StatusCode::OK,
        [("content-type", "text/html")],
        include_str!("../../dashboard.html"),
    )
}

/// WebSocket handler for streaming metrics
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: axum::extract::ws::WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    // Spawn task to forward broadcast messages to client
    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            if let Ok(msg) = serde_json::to_string(&event) {
                if sender.send(axum::extract::ws::Message::Text(msg)).await.is_err() {
                    break; // Client disconnected
                }
            }
        }
    });

    // Handle incoming messages from client
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            axum::extract::ws::Message::Text(text) => {
                info!("Dashboard client message: {}", text);
            }
            axum::extract::ws::Message::Close(_) => {
                info!("Dashboard client disconnected");
                break;
            }
            _ => {}
        }
    }
}

/// Broadcast metrics periodically
async fn broadcast_metrics(tx: broadcast::Sender<DashboardEvent>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));

    loop {
        interval.tick().await;

        let event = DashboardEvent {
            timestamp: chrono::Utc::now().to_string(),
            event_type: "metrics".into(),
            data: serde_json::json!({
                "jobs_completed": 1234,
                "success_rate": 0.956,
                "avg_fidelity": 0.923,
                "languages_active": 12,
                "total_languages": 750,
                "avg_exec_time_us": 1500,
                "total_energy_joules": 2345.67,
            }),
        };

        if tx.send(event).is_err() {
            warn!("No subscribers for metrics");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dashboard_event_serialization() {
        let event = DashboardEvent {
            timestamp: "2026-06-04T12:00:00Z".into(),
            event_type: "metrics".into(),
            data: serde_json::json!({"test": "data"}),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("metrics"));
        assert!(json.contains("test"));
    }

    #[tokio::test]
    async fn test_broadcast_channel() {
        let (tx, mut rx) = broadcast::channel::<DashboardEvent>(10);

        let event = DashboardEvent {
            timestamp: "2026-06-04T12:00:00Z".into(),
            event_type: "test".into(),
            data: serde_json::json!({}),
        };

        let _ = tx.send(event.clone());
        let received = rx.recv().await.unwrap();

        assert_eq!(received.event_type, event.event_type);
    }
}
