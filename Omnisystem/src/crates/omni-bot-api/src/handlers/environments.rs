//! Environment management handlers
//! Implements 10 REST endpoints for environment lifecycle, execution, and operations

use crate::error::{ApiError, ApiResult};
use crate::models::*;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Shared environment state store
pub type EnvironmentStore = Arc<RwLock<HashMap<String, EnvironmentInfo>>>;

/// Initialization function
pub fn init_store() -> EnvironmentStore {
    Arc::new(RwLock::new(HashMap::new()))
}

// ============================================================================
// REQUEST MODELS
// ============================================================================

/// Query parameters for listing environments
#[derive(Debug, Clone, Deserialize)]
pub struct ListEnvironmentsQuery {
    /// Filter by state
    pub state: Option<String>,
    /// Filter by type
    pub env_type: Option<String>,
    /// Filter by tags
    pub tags: Option<String>,
    /// Page number (0-indexed)
    pub page: Option<u32>,
    /// Items per page
    pub per_page: Option<u32>,
    /// Sort field (name, created_at, updated_at)
    pub sort_by: Option<String>,
    /// Sort order (asc, desc)
    pub sort_order: Option<String>,
}

// ============================================================================
// ENDPOINT HANDLERS
// ============================================================================

/// GET /environments - List all environments with filtering
pub async fn list_environments(
    State(store): State<EnvironmentStore>,
    Query(params): Query<ListEnvironmentsQuery>,
) -> ApiResult<Json<ListResponse<EnvironmentInfo>>> {
    let envs = store.read().await;

    let mut filtered: Vec<_> = envs
        .values()
        .filter(|env| {
            if let Some(ref state) = params.state {
                if !format!("{:?}", env.state).to_lowercase().contains(&state.to_lowercase())
                {
                    return false;
                }
            }
            if let Some(ref env_type) = params.env_type {
                if !env.env_type.contains(env_type) {
                    return false;
                }
            }
            if let Some(ref tags) = params.tags {
                let search_tags: Vec<&str> = tags.split(',').map(|s| s.trim()).collect();
                if let Some(env_tags) = &env.tags {
                    if !search_tags.iter().any(|t| env_tags.contains(&t.to_string())) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    // Sorting
    if let Some(sort_by) = params.sort_by {
        match sort_by.as_str() {
            "name" => filtered.sort_by(|a, b| a.name.cmp(&b.name)),
            "created_at" => filtered.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
            "updated_at" => filtered.sort_by(|a, b| a.updated_at.cmp(&b.updated_at)),
            _ => {}
        }
    }

    if params.sort_order == Some("desc".to_string()) {
        filtered.reverse();
    }

    // Pagination
    let page = params.page.unwrap_or(0);
    let per_page = params.per_page.unwrap_or(20).min(100);
    let total = filtered.len() as u32;
    let start = (page * per_page) as usize;
    let end = ((page + 1) * per_page) as usize;

    let items: Vec<_> = filtered
        .into_iter()
        .skip(start)
        .take(end - start)
        .collect();

    Ok(Json(ListResponse::new(items, total, page, per_page)))
}

/// POST /environments - Create a new environment
pub async fn create_environment(
    State(store): State<EnvironmentStore>,
    Json(spec): Json<EnvironmentSpec>,
) -> ApiResult<(StatusCode, Json<EnvironmentInfo>)> {
    // Validate resource allocation
    if spec.resources.memory_mb == 0 || spec.resources.cpu_cores == 0 {
        return Err(ApiError::InvalidResourceAllocation(
            "Memory and CPU cores must be > 0".to_string(),
        ));
    }

    let mut envs = store.write().await;

    if envs.contains_key(&spec.id) {
        return Err(ApiError::EnvironmentAlreadyExists(spec.id.clone()));
    }

    let now = Utc::now();
    let env_info = EnvironmentInfo {
        id: spec.id.clone(),
        name: spec.name,
        env_type: spec.env_type,
        state: EnvironmentState::Created,
        resources: spec.resources,
        resource_usage: ResourceUsage::default(),
        env_vars: spec.env_vars,
        modules: Vec::new(),
        snapshots: Vec::new(),
        created_at: now,
        updated_at: now,
        last_started: None,
        tags: spec.tags,
        metadata: spec.metadata,
    };

    envs.insert(spec.id.clone(), env_info.clone());

    Ok((StatusCode::CREATED, Json(env_info)))
}

/// POST /environments/{id}/start - Start an environment
pub async fn start_environment(
    State(store): State<EnvironmentStore>,
    Path(id): Path<String>,
) -> ApiResult<Json<EnvironmentInfo>> {
    let mut envs = store.write().await;

    let env = envs.get_mut(&id).ok_or_else(|| {
        ApiError::EnvironmentNotFound(format!("Environment '{}' not found", id))
    })?;

    if !env.state.can_start() {
        return Err(ApiError::OperationNotAllowed(format!(
            "Cannot start environment in state: {:?}",
            env.state
        )));
    }

    env.state = EnvironmentState::Starting;
    env.updated_at = Utc::now();

    // Simulate async startup
    let env_clone = env.clone();
    let store_clone = Arc::clone(&store);
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let mut envs = store_clone.write().await;
        if let Some(e) = envs.get_mut(&env_clone.id) {
            e.state = EnvironmentState::Running;
            e.last_started = Some(Utc::now());
            e.updated_at = Utc::now();
        }
    });

    Ok(Json(env.clone()))
}

/// POST /environments/{id}/stop - Stop a running environment
pub async fn stop_environment(
    State(store): State<EnvironmentStore>,
    Path(id): Path<String>,
) -> ApiResult<Json<EnvironmentInfo>> {
    let mut envs = store.write().await;

    let env = envs.get_mut(&id).ok_or_else(|| {
        ApiError::EnvironmentNotFound(format!("Environment '{}' not found", id))
    })?;

    if !env.state.can_stop() {
        return Err(ApiError::OperationNotAllowed(format!(
            "Cannot stop environment in state: {:?}",
            env.state
        )));
    }

    env.state = EnvironmentState::Stopping;
    env.updated_at = Utc::now();

    // Simulate async shutdown
    let env_clone = env.clone();
    let store_clone = Arc::clone(&store);
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let mut envs = store_clone.write().await;
        if let Some(e) = envs.get_mut(&env_clone.id) {
            e.state = EnvironmentState::Stopped;
            e.updated_at = Utc::now();
        }
    });

    Ok(Json(env.clone()))
}

/// POST /environments/{id}/snapshot - Create an environment snapshot
pub async fn snapshot_environment(
    State(store): State<EnvironmentStore>,
    Path(id): Path<String>,
) -> ApiResult<Json<SnapshotInfo>> {
    let mut envs = store.write().await;

    let env = envs.get_mut(&id).ok_or_else(|| {
        ApiError::EnvironmentNotFound(format!("Environment '{}' not found", id))
    })?;

    if !env.state.is_running() {
        return Err(ApiError::OperationNotAllowed(
            "Can only snapshot running environments".to_string(),
        ));
    }

    let snapshot_id = format!("{}-snap-{}", id, Uuid::new_v4().to_string()[..8].to_string());
    let cas_hash = format!(
        "blake3-{}",
        blake3::hash(snapshot_id.as_bytes()).to_hex()[..16].to_string()
    );

    let snapshot = SnapshotInfo {
        id: snapshot_id,
        created_at: Utc::now(),
        size_mb: env.resource_usage.disk_mb,
        description: Some(format!("Snapshot of {}", env.name)),
        cas_hash,
    };

    env.snapshots.push(snapshot.clone());
    env.updated_at = Utc::now();

    Ok(Json(snapshot))
}

/// POST /environments/{id}/restore - Restore environment from snapshot
pub async fn restore_environment(
    State(store): State<EnvironmentStore>,
    Path(id): Path<String>,
    Json(req): Json<RestoreSnapshotRequest>,
) -> ApiResult<Json<AsyncOperationResponse>> {
    let mut envs = store.write().await;

    let env = envs.get_mut(&id).ok_or_else(|| {
        ApiError::EnvironmentNotFound(format!("Environment '{}' not found", id))
    })?;

    let snapshot_exists = env.snapshots.iter().any(|s| s.id == req.snapshot_id);
    if !snapshot_exists {
        return Err(ApiError::SnapshotNotFound(req.snapshot_id));
    }

    let task_id = format!("restore-{}-{}", id, Uuid::new_v4().to_string()[..8].to_string());

    Ok(Json(AsyncOperationResponse {
        task_id,
        status: "started".to_string(),
        created_at: Utc::now(),
        progress_url: None,
    }))
}

/// POST /environments/{id}/migrate - Migrate environment to different host/region
pub async fn migrate_environment(
    State(store): State<EnvironmentStore>,
    Path(id): Path<String>,
    Json(_req): Json<MigrationRequest>,
) -> ApiResult<Json<AsyncOperationResponse>> {
    let mut envs = store.write().await;

    let env = envs.get_mut(&id).ok_or_else(|| {
        ApiError::EnvironmentNotFound(format!("Environment '{}' not found", id))
    })?;

    if !env.state.is_running() {
        return Err(ApiError::OperationNotAllowed(
            "Can only migrate running environments".to_string(),
        ));
    }

    env.state = EnvironmentState::Migrating;
    env.updated_at = Utc::now();

    let task_id = format!("migrate-{}-{}", id, Uuid::new_v4().to_string()[..8].to_string());

    Ok(Json(AsyncOperationResponse {
        task_id: task_id.clone(),
        status: "started".to_string(),
        created_at: Utc::now(),
        progress_url: Some(format!("/api/operations/{}/progress", task_id)),
    }))
}

/// DELETE /environments/{id} - Delete an environment
pub async fn delete_environment(
    State(store): State<EnvironmentStore>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let mut envs = store.write().await;

    let env = envs.get(&id).ok_or_else(|| {
        ApiError::EnvironmentNotFound(format!("Environment '{}' not found", id))
    })?;

    if env.state != EnvironmentState::Stopped && env.state != EnvironmentState::Created {
        return Err(ApiError::OperationNotAllowed(
            "Can only delete stopped or created environments".to_string(),
        ));
    }

    envs.remove(&id);

    Ok(StatusCode::NO_CONTENT)
}

/// GET /environments/{id}/status - Get environment status
pub async fn get_environment_status(
    State(store): State<EnvironmentStore>,
    Path(id): Path<String>,
) -> ApiResult<Json<EnvironmentStatus>> {
    let envs = store.read().await;

    let env = envs.get(&id).ok_or_else(|| {
        ApiError::EnvironmentNotFound(format!("Environment '{}' not found", id))
    })?;

    let uptime_seconds = if let Some(started) = env.last_started {
        (Utc::now() - started).num_seconds() as u64
    } else {
        0
    };

    Ok(Json(EnvironmentStatus {
        id: env.id.clone(),
        state: env.state,
        resource_usage: env.resource_usage.clone(),
        uptime_seconds,
        last_health_check: Utc::now(),
        error: None,
    }))
}

/// POST /environments/{id}/exec - Execute command in environment
pub async fn execute_command(
    State(store): State<EnvironmentStore>,
    Path(id): Path<String>,
    Json(req): Json<ExecuteCommandRequest>,
) -> ApiResult<Json<CommandExecutionResult>> {
    let envs = store.read().await;

    let env = envs.get(&id).ok_or_else(|| {
        ApiError::EnvironmentNotFound(format!("Environment '{}' not found", id))
    })?;

    if !env.state.is_running() {
        return Err(ApiError::OperationNotAllowed(
            "Can only execute commands in running environments".to_string(),
        ));
    }

    // Validate command
    if req.command.is_empty() {
        return Err(ApiError::InvalidRequest("Command cannot be empty".to_string()));
    }

    // Simulate command execution with timeout handling
    let timeout = req.timeout_seconds.unwrap_or(30);
    let duration_ms = (timeout as u64 * 100).min(5000);

    Ok(Json(CommandExecutionResult {
        exit_code: 0,
        stdout: format!("Executed: {}", req.command),
        stderr: String::new(),
        duration_ms,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_environment() {
        let store = init_store();

        let spec = EnvironmentSpec {
            id: "test-env".to_string(),
            name: "Test Environment".to_string(),
            env_type: "container".to_string(),
            resources: ResourceAllocation::default(),
            env_vars: HashMap::new(),
            modules: None,
            snapshot_id: None,
            metadata: None,
            tags: None,
        };

        let result = create_environment(State(store), Json(spec)).await;
        assert!(result.is_ok());
        let (status, env) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(env.id, "test-env");
        assert_eq!(env.state, EnvironmentState::Created);
    }

    #[tokio::test]
    async fn test_start_environment() {
        let store = init_store();
        let mut envs = store.write().await;

        let env = EnvironmentInfo {
            id: "test-start".to_string(),
            name: "Test Start".to_string(),
            env_type: "container".to_string(),
            state: EnvironmentState::Created,
            resources: ResourceAllocation::default(),
            resource_usage: ResourceUsage::default(),
            env_vars: HashMap::new(),
            modules: Vec::new(),
            snapshots: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_started: None,
            tags: None,
            metadata: None,
        };
        envs.insert("test-start".to_string(), env);
        drop(envs);

        let result = start_environment(State(store), Path("test-start".to_string())).await;
        assert!(result.is_ok());
        let env = result.unwrap();
        assert_eq!(env.state, EnvironmentState::Starting);
    }

    #[tokio::test]
    async fn test_invalid_resource_allocation() {
        let store = init_store();

        let spec = EnvironmentSpec {
            id: "test-env".to_string(),
            name: "Test".to_string(),
            env_type: "container".to_string(),
            resources: ResourceAllocation {
                memory_mb: 0,
                cpu_cores: 2,
                cpu_percent_max: 100,
                disk_mb: 1024,
                iops_limit: 1000,
                bandwidth_mbps: 100,
            },
            env_vars: HashMap::new(),
            modules: None,
            snapshot_id: None,
            metadata: None,
            tags: None,
        };

        let result = create_environment(State(store), Json(spec)).await;
        assert!(result.is_err());
    }
}
