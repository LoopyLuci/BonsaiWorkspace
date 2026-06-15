use axum::{
    extract::Path,
    response::Json,
    routing::{get, post},
    Extension, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::agent_host::AgentHost;

#[derive(Serialize, Deserialize)]
pub struct AgentCard {
    pub name: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct TaskRequest {
    pub agent_id: String,
    pub task: String,
    pub input: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct TaskResponse {
    pub task_id: String,
    pub status: String,
    pub output: Option<serde_json::Value>,
}

pub async fn start_a2a_server(port: u16, agent_host: Arc<AgentHost>) {
    let app = Router::new()
        .route("/.well-known/agent.json", get(get_agent_card))
        .route("/tasks/send", post(send_task))
        .route("/tasks/get/:task_id", get(get_task))
        .layer(Extension(agent_host));

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind a2a");
    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("a2a server error: {e}");
    }
}

async fn get_agent_card() -> Json<AgentCard> {
    Json(AgentCard {
        name: "BonsAI".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        capabilities: vec![
            "text".into(),
            "code".into(),
            "vision".into(),
            "music".into(),
        ],
        url: "http://localhost".into(),
    })
}

async fn send_task(Json(payload): Json<TaskRequest>) -> Json<TaskResponse> {
    // TODO: enqueue into AgentHost task table and return task id
    Json(TaskResponse {
        task_id: uuid::Uuid::new_v4().to_string(),
        status: "accepted".into(),
        output: None,
    })
}

async fn get_task(Path(task_id): Path<String>) -> Json<TaskResponse> {
    // TODO: query task status
    Json(TaskResponse {
        task_id,
        status: "completed".into(),
        output: Some(json!({"result":"example"})),
    })
}
