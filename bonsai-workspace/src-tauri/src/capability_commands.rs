use tauri::State;
use serde_json::json;

#[tauri::command]
pub async fn get_capability_summary(
    state: State<'_, crate::AppState>,
) -> Result<String, String> {
    let summary = state.capability_registry.get_manifest().await.summary;
    Ok(summary)
}

#[tauri::command]
pub async fn get_capability_manifest(
    state: State<'_, crate::AppState>,
) -> Result<serde_json::Value, String> {
    let manifest = state.capability_registry.get_manifest().await;
    serde_json::to_value(&manifest).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn query_capabilities(
    state: State<'_, crate::AppState>,
    query: String,
    top_k: Option<usize>,
) -> Result<serde_json::Value, String> {
    let top = top_k.unwrap_or(6);
    let q = bonsai_query::CapabilityQuery::new(state.capability_registry.clone());
    let scored = q.search(&query, None, top).await;
    let arr: Vec<serde_json::Value> = scored.into_iter().map(|s| {
        json!({ "entry": s.entry, "score": s.score })
    }).collect();
    Ok(serde_json::Value::Array(arr))
}
