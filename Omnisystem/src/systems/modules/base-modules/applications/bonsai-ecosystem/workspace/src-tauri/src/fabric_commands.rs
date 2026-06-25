use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use fabric::catalog;

pub struct FabricState {
    pub coordinator: Arc<fabric::CoordinatorActor>,
}

impl FabricState {
    pub fn new() -> Self {
        Self {
            coordinator: Arc::new(fabric::CoordinatorActor::new()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TaskRequest {
    pub project_id: String,
    pub task_type: String,
    pub payload_json: String,
    pub priority: u8,
    pub required_memory_mb: u64,
    pub required_cores: u32,
    pub deadline_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub task_id: String,
    pub status: String,
    pub output_json: Option<String>,
    pub duration_ms: u64,
}

#[tauri::command]
pub async fn fabric_submit_and_await(
    state: State<'_, FabricState>,
    request: TaskRequest,
) -> Result<TaskResponse, String> {
    use fabric::types::{FabricTask, TaskStatus, TaskType};

    let task_id = uuid::Uuid::new_v4().to_string();
    let task_type = match request.task_type.as_str() {
        "inference" => TaskType::Inference,
        "data_process" => TaskType::DataProcess,
        "script" => TaskType::Script,
        _ => TaskType::Wasm,
    };

    let task = FabricTask {
        task_id: task_id.clone(),
        project_id: request.project_id,
        task_type,
        payload: request.payload_json.into_bytes(),
        priority: request.priority,
        required_memory_mb: request.required_memory_mb,
        required_cores: request.required_cores,
    };

    let deadline = request.deadline_ms.unwrap_or(30_000);
    match state.coordinator.submit_task(task, deadline).await {
        Some(result) => {
            let status = match &result.status {
                TaskStatus::Completed => "completed".to_string(),
                TaskStatus::Failed { reason } => format!("failed:{reason}"),
                TaskStatus::Running { node_id } => format!("running:{node_id}"),
                TaskStatus::Assigned { node_id } => format!("assigned:{node_id}"),
                TaskStatus::Queued => "queued".to_string(),
            };
            let output_json = result
                .output
                .map(|b| String::from_utf8_lossy(&b).to_string());
            Ok(TaskResponse {
                task_id: result.task_id,
                status,
                output_json,
                duration_ms: result.duration_ms,
            })
        }
        None => Err("Task timed out or no capable node available".to_string()),
    }
}

#[tauri::command]
pub async fn fabric_register_node(
    state: State<'_, FabricState>,
    node_id: String,
    display_name: String,
    available_cores: u32,
    available_memory_mb: u64,
) -> Result<(), String> {
    use fabric::types::ComputeNode;
    state
        .coordinator
        .add_node(ComputeNode {
            node_id,
            display_name,
            available_cores,
            available_memory_mb,
            is_online: true,
        })
        .await;
    Ok(())
}

// ─── Task Catalog commands ────────────────────────────────────────────────────

/// List all registered distributed-computing task profiles.
#[tauri::command]
pub async fn fabric_list_catalog() -> Result<Vec<catalog::TaskProfile>, String> {
    Ok(catalog::CATALOG.to_vec())
}

/// List task profiles in a single category.
#[tauri::command]
pub async fn fabric_catalog_by_category(category: String) -> Result<Vec<catalog::TaskProfile>, String> {
    use catalog::TaskCategory::*;
    let cat = match category.as_str() {
        "ai_ml" => AiMl,
        "build_ci" => BuildCi,
        "multimedia" => Multimedia,
        "simulation" => Simulation,
        "data_analytics" => DataAnalytics,
        "cryptography" => Cryptography,
        "security" => Security,
        "edge_iot" => EdgeIot,
        "bonsai_native" => BonsaiNative,
        "interop" => Interop,
        other => return Err(format!("unknown category: {other}")),
    };
    Ok(catalog::by_category(cat).into_iter().cloned().collect())
}

/// List task profiles a node with the given resources can run.
#[tauri::command]
pub async fn fabric_catalog_runnable(
    cores: u32,
    memory_mb: u64,
    has_gpu: bool,
) -> Result<Vec<catalog::TaskProfile>, String> {
    Ok(catalog::runnable_on(cores, memory_mb, has_gpu).into_iter().cloned().collect())
}

/// (category, count) summary across the catalog.
#[tauri::command]
pub async fn fabric_catalog_summary() -> Result<serde_json::Value, String> {
    let summary: Vec<_> = catalog::category_summary()
        .into_iter()
        .map(|(name, n)| serde_json::json!({ "category": name, "count": n }))
        .collect();
    Ok(serde_json::json!({ "total": catalog::count(), "categories": summary }))
}

/// Submit a catalog task by its id, instantiating a FabricTask from the profile.
#[tauri::command]
pub async fn fabric_submit_catalog_task(
    state: State<'_, FabricState>,
    profile_id: String,
    project_id: String,
    payload: Vec<u8>,
    priority: u8,
) -> Result<String, String> {
    let profile = catalog::get(&profile_id)
        .ok_or_else(|| format!("unknown task profile: {profile_id}"))?;
    let task = profile.to_fabric_task(project_id, payload, priority);
    match state.coordinator.submit_task(task, 30_000).await {
        Some(r) => Ok(serde_json::to_string(&r).unwrap_or_default()),
        None => Err("task timed out or no capable node".to_string()),
    }
}
