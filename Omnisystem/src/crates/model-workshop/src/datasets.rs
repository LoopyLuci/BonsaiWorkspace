use axum::{Json, extract::{State, Path}};
use serde::Deserialize;
use crate::{AppState, DatasetInfo};

#[derive(Debug, Deserialize)]
pub struct CreateDatasetRequest {
    pub name: String,
    pub source_module: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImportDataRequest {
    pub file_path: String,
    pub format: String,
}

pub async fn list_datasets(State(state): State<AppState>) -> Json<Vec<DatasetInfo>> {
    let datasets = state.datasets.read().await;
    let list: Vec<DatasetInfo> = datasets.values().cloned().collect();
    Json(list)
}

pub async fn create_dataset(
    State(state): State<AppState>,
    Json(req): Json<CreateDatasetRequest>,
) -> Json<serde_json::Value> {
    let dataset_id = format!("ds-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());

    let dataset = DatasetInfo {
        id: dataset_id.clone(),
        name: req.name.clone(),
        num_examples: 0,
        domains: vec![],
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let _ = state.datasets.write().await.insert(dataset_id.clone(), dataset);

    Json(serde_json::json!({
        "status": "created",
        "dataset_id": dataset_id,
        "name": req.name,
        "source_module": req.source_module
    }))
}

pub async fn delete_dataset(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let _ = state.datasets.write().await.remove(&id);
    Json(serde_json::json!({
        "status": "deleted",
        "dataset_id": id
    }))
}

pub async fn import_data(
    State(_state): State<AppState>,
    Path(dataset_id): Path<String>,
    Json(req): Json<ImportDataRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "importing",
        "dataset_id": dataset_id,
        "file": req.file_path,
        "format": req.format,
        "estimated_time": "30-60 seconds"
    }))
}
