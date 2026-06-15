use crate::thoughts::{ThinkingHistoryEntry, ThoughtSegment};
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[tauri::command]
pub async fn add_thought(
    state: State<'_, AppState>,
    thought: ThoughtSegment,
) -> Result<(), String> {
    state.thoughts_db.add_thought(thought).await
}

#[tauri::command]
pub async fn get_thoughts_for_turn(
    state: State<'_, AppState>,
    turn_id: String,
) -> Result<Vec<ThoughtSegment>, String> {
    state.thoughts_db.get_thoughts_for_turn(&turn_id).await
}

#[tauri::command]
pub async fn clear_thoughts_for_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    state
        .thoughts_db
        .clear_thoughts_for_session(&session_id)
        .await
}

#[derive(Debug, Deserialize)]
pub struct SearchThinkingRequest {
    pub query: String,
    pub session_id: Option<String>,
    pub model_role: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SearchThinkingResponse {
    pub entries: Vec<ThinkingHistoryEntry>,
    pub limit: i64,
    pub offset: i64,
}

#[tauri::command]
pub async fn search_thinking_history(
    state: State<'_, AppState>,
    request: SearchThinkingRequest,
) -> Result<SearchThinkingResponse, String> {
    let limit = request.limit.unwrap_or(50).min(200);
    let offset = request.offset.unwrap_or(0);
    let entries = state
        .thoughts_db
        .search_thinking_history(
            &request.query,
            request.session_id.as_deref(),
            request.model_role.as_deref(),
            limit,
            offset,
        )
        .await?;
    Ok(SearchThinkingResponse {
        entries,
        limit,
        offset,
    })
}

#[derive(Debug, Deserialize)]
pub struct RecordThinkingRequest {
    pub session_id: String,
    pub turn_id: String,
    pub model_role: String,
    pub content: String,
}

#[tauri::command]
pub async fn record_thinking(
    state: State<'_, AppState>,
    request: RecordThinkingRequest,
) -> Result<String, String> {
    state
        .thoughts_db
        .record_thinking(
            &request.session_id,
            &request.turn_id,
            &request.model_role,
            &request.content,
        )
        .await
}

#[tauri::command]
pub async fn get_thinking_settings(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    Ok(state.thinking_settings.read().await.clone())
}

#[tauri::command]
pub async fn set_thinking_settings(
    state: State<'_, AppState>,
    settings: serde_json::Value,
) -> Result<(), String> {
    *state.thinking_settings.write().await = settings;
    Ok(())
}
