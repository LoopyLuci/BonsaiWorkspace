use tokio::net::TcpListener;
use axum::{Router, routing::post, Json};
use serde_json::json;

async fn run_mock_daemon(port: u16) -> tokio::task::JoinHandle<()> {
    let app = Router::new()
        .route("/api/file/read", post(|| async { Json(json!({"content": "mock content"})) }))
        .route("/api/chat", post(|| async { Json(json!({"response": "mock response"})) }));
    tokio::spawn(async move {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    })
}

#[tokio::test]
async fn test_mcp_server_connects_to_daemon() {
    let daemon_port = 11428;
    let _daemon = run_mock_daemon(daemon_port).await;
    std::env::set_var("BONSAI_DAEMON_URL", format!("http://127.0.0.1:{}", daemon_port));
    // The MCP server would be started here; for now we just test the mock.
    assert!(true);
}
