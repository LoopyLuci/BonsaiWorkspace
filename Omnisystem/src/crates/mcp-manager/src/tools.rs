use axum::{Json, extract::{State, Path}};
use crate::{AppState, ToolEntry};

pub async fn list_tools(State(state): State<AppState>) -> Json<Vec<ToolEntry>> {
    let tools = state.tool_registry.read().await;
    Json(tools.clone())
}

pub async fn enable_tool(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<serde_json::Value> {
    let mut tools = state.tool_registry.write().await;
    if let Some(tool) = tools.iter_mut().find(|t| t.name == name) {
        tool.enabled = true;
        Json(serde_json::json!({
            "status": "enabled",
            "tool": name,
            "message": "Tool enabled"
        }))
    } else {
        Json(serde_json::json!({"error": "Tool not found"}))
    }
}

pub async fn disable_tool(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<serde_json::Value> {
    let mut tools = state.tool_registry.write().await;
    if let Some(tool) = tools.iter_mut().find(|t| t.name == name) {
        tool.enabled = false;
        Json(serde_json::json!({
            "status": "disabled",
            "tool": name,
            "message": "Tool disabled"
        }))
    } else {
        Json(serde_json::json!({"error": "Tool not found"}))
    }
}
