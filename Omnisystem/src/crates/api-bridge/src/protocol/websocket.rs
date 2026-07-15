use axum::extract::ws::{Message, WebSocket};
use futures::StreamExt;
use tokio::sync::broadcast;

pub async fn stream_events(mut socket: WebSocket, mut rx: broadcast::Receiver<String>) {
    while let Ok(msg) = rx.recv().await {
        if socket.send(Message::Text(msg)).await.is_err() {
            return;
        }
        if let Ok(Some(Ok(Message::Close(_)))) = tokio::time::timeout(
            std::time::Duration::from_millis(1),
            socket.next(),
        )
        .await
        {
            return;
        }
    }
}
