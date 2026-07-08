use axum::{Json, extract::{State, Path}};
use serde::Deserialize;
use crate::{AppState, ModuleInfo};

#[derive(Debug, Deserialize)]
pub struct CreateModuleRequest {
    pub name: String,
    pub description: String,
    pub domains: Vec<String>,
    pub chunks: Vec<ChunkInput>,
}

#[derive(Debug, Deserialize)]
pub struct ChunkInput {
    pub text: String,
    pub domain: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModuleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub domains: Option<Vec<String>>,
}

pub async fn list_modules(State(state): State<AppState>) -> Json<Vec<ModuleInfo>> {
    let modules = state.modules.read().await;
    let list: Vec<ModuleInfo> = modules.values().cloned().collect();
    Json(list)
}

pub async fn create_module(
    State(state): State<AppState>,
    Json(req): Json<CreateModuleRequest>,
) -> Json<serde_json::Value> {
    let module_id = format!(
        "{}-{}",
        req.name.to_lowercase().replace(' ', "-"),
        uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
    );

    let module = ModuleInfo {
        id: module_id.clone(),
        name: req.name.clone(),
        version: "1.0.0".into(),
        description: req.description.clone(),
        num_chunks: req.chunks.len(),
        domains: req.domains.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let _ = state.modules.write().await.insert(module_id.clone(), module);

    Json(serde_json::json!({
        "status": "created",
        "module_id": module_id,
        "name": req.name,
        "chunks_added": req.chunks.len()
    }))
}

pub async fn get_module(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let modules = state.modules.read().await;
    if let Some(module) = modules.get(&id) {
        Json(serde_json::to_value(module).unwrap())
    } else {
        Json(serde_json::json!({"error": "Module not found"}))
    }
}

pub async fn update_module(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateModuleRequest>,
) -> Json<serde_json::Value> {
    let mut modules = state.modules.write().await;
    if let Some(module) = modules.get_mut(&id) {
        if let Some(name) = req.name {
            module.name = name;
        }
        if let Some(description) = req.description {
            module.description = description;
        }
        if let Some(domains) = req.domains {
            module.domains = domains;
        }
        Json(serde_json::json!({
            "status": "updated",
            "module_id": id
        }))
    } else {
        Json(serde_json::json!({"error": "Module not found"}))
    }
}

pub async fn delete_module(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let _ = state.modules.write().await.remove(&id);
    Json(serde_json::json!({
        "status": "deleted",
        "module_id": id
    }))
}

pub async fn add_chunk(
    State(state): State<AppState>,
    Path(module_id): Path<String>,
    Json(_req): Json<ChunkInput>,
) -> Json<serde_json::Value> {
    let mut modules = state.modules.write().await;
    if let Some(module) = modules.get_mut(&module_id) {
        module.num_chunks += 1;
        let chunk_id = uuid::Uuid::new_v4().to_string();
        Json(serde_json::json!({
            "status": "chunk_added",
            "chunk_id": chunk_id,
            "module_id": module_id,
            "total_chunks": module.num_chunks
        }))
    } else {
        Json(serde_json::json!({"error": "Module not found"}))
    }
}

pub async fn remove_chunk(
    State(state): State<AppState>,
    Path((module_id, chunk_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    let mut modules = state.modules.write().await;
    if let Some(module) = modules.get_mut(&module_id) {
        if module.num_chunks > 0 {
            module.num_chunks -= 1;
        }
        Json(serde_json::json!({
            "status": "chunk_removed",
            "chunk_id": chunk_id,
            "remaining_chunks": module.num_chunks
        }))
    } else {
        Json(serde_json::json!({"error": "Module not found"}))
    }
}
