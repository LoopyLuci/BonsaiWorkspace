use tauri::command;

#[command]
pub async fn list_models() -> Result<Vec<String>, String> {
    // In production this would query `model_registry` or `model_orchestrator`.
    Ok(vec!["bonsai-small-v1".to_string(), "bonsai-chat-v2".to_string()])
}

#[command]
pub async fn submit_inference(model: String, prompt: String) -> Result<String, String> {
    // Submit an inference request to the task queue or orchestrator.
    // Here we return a fake request id for UI testing.
    let req_id = format!("req-{}", uuid::Uuid::new_v4());
    tracing::info!("submit_inference: model={} req_id={}", model, req_id);
    Ok(req_id)
}

#[command]
pub async fn get_inference_status(request_id: String) -> Result<serde_json::Value, String> {
    // Return a mock status object. Replace with real task lookup.
    Ok(serde_json::json!({"id": request_id, "state": "completed", "output": "Hello from mock inference"}))
}

#[command]
pub async fn cancel_inference(request_id: String) -> Result<bool, String> {
    tracing::info!("cancel_inference {}", request_id);
    Ok(true)
}
