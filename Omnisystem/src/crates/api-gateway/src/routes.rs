use axum::{
    Router, routing::{get, post}, extract::State, Json,
    http::{StatusCode, HeaderMap},
    response::sse::{Event, Sse},
};
use std::sync::Arc;
use futures::stream::Stream;
use crate::types::*;
use crate::auth;
use crate::AppState;

pub fn openai_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/chat/completions", post(chat_completions))
        .route("/models", get(list_models))
}

pub fn native_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/generate", post(native_generate))
        .route("/pull", post(pull_model))
        .route("/tags", get(list_local_models))
}

async fn chat_completions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>, (StatusCode, Json<serde_json::Value>)> {
    let _token = auth::extract_token(&headers).map_err(|e| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": e.to_string()}))) )?;
    let (name, version) = parse_model_string(&req.model);
    let model_info = state.model_registry.get(&name, &version).await
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Model not found"}))))?;
    let inference_req = convert_request(&req, &model_info);
    let output = state.inference_engine.generate(&inference_req)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;
    let response = ChatCompletionResponse {
        id: "test".into(),
        object: "chat.completion".into(),
        created: chrono::Utc::now().timestamp() as u64,
        model: format!("{}:{}", model_info.name, model_info.version),
        choices: vec![Choice {
            index: 0,
            message: ChatMessage {
                role: "assistant".into(),
                content: output.text,
                tool_calls: None,
            },
            finish_reason: output.finish_reason,
        }],
        usage: Usage {
            prompt_tokens: output.usage.prompt_tokens,
            completion_tokens: output.usage.completion_tokens,
            total_tokens: output.usage.total_tokens,
        },
    };
    Ok(Json(response))
}

async fn list_models(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let models = state.model_registry.list().await;
    let data = models.into_iter().map(|m| serde_json::json!({
        "id": format!("{}:{}", m.name, m.version),
        "object": "model",
        "created": m.created_at,
        "owned_by": m.author,
    })).collect();
    Json(serde_json::json!({ "object": "list", "data": data }))
}

async fn list_local_models(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let models = state.model_registry.list().await;
    let data = models.into_iter().map(|m| serde_json::json!({
        "name": format!("{}:{}", m.name, m.version),
        "modified_at": m.created_at,
        "size": m.size_bytes,
    })).collect();
    Json(serde_json::json!({ "models": data }))
}

async fn pull_model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let _token = auth::extract_token(&headers).map_err(|e| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": e.to_string()}))))?;
    let name = req["name"].as_str().unwrap_or("");
    let info = model_registry::pull::pull_model(name, &state.model_registry).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;
    Ok(Json(serde_json::json!({ "status": "success", "model": info })))
}

async fn native_generate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let _token = auth::extract_token(&headers).map_err(|e| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": e.to_string()}))))?;
    let name = req["model"].as_str().unwrap_or("");
    let prompt = req["prompt"].as_str().unwrap_or("");
    let (name, version) = parse_model_string(name);
    let model_info = state.model_registry.get(&name, &version).await
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Model not found"}))))?;
    let inference_req = inference::InferenceRequest {
        model_name: name,
        model_version: version,
        prompt: prompt.to_string(),
        messages: None,
        temperature: req["temperature"].as_f64().unwrap_or(0.7) as f32,
        top_p: req["top_p"].as_f64().unwrap_or(0.9) as f32,
        max_tokens: req["max_tokens"].as_u64().unwrap_or(4096) as u32,
        stream: false,
        tools: None,
        response_format: None,
        capability_token: "".into(),
    };
    let output = state.inference_engine.generate(&inference_req)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;
    Ok(Json(serde_json::json!({ "response": output.text })))
}

fn parse_model_string(s: &str) -> (String, String) {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    (parts[0].to_string(), parts.get(1).unwrap_or(&"latest").to_string())
}

fn convert_request(req: &ChatCompletionRequest, model_info: &model_registry::ModelInfo) -> inference::InferenceRequest {
    let mut prompt = String::new();
    for msg in &req.messages {
        prompt.push_str(&format!("{}: {}\n", msg.role, msg.content));
    }
    inference::InferenceRequest {
        model_name: model_info.name.clone(),
        model_version: model_info.version.clone(),
        prompt,
        messages: None,
        temperature: req.temperature.unwrap_or(0.7),
        top_p: req.top_p.unwrap_or(0.9),
        max_tokens: req.max_tokens.unwrap_or(4096),
        stream: req.stream.unwrap_or(false),
        tools: None,
        response_format: None,
        capability_token: "".into(),
    }
}
