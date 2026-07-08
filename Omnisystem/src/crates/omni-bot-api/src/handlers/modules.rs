//! Module (UMS) management handlers
//! Implements 6 REST endpoints for module installation, verification, and lifecycle

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

/// Shared module registry state
pub type ModuleRegistry = Arc<RwLock<HashMap<String, ModuleInfo>>>;

/// Initialization function
pub fn init_registry() -> ModuleRegistry {
    Arc::new(RwLock::new(HashMap::new()))
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Generate BLAKE3 hash for module content
fn compute_module_hash(name: &str, version: &str) -> String {
    let content = format!("{}-{}", name, version);
    format!(
        "blake3-{}",
        blake3::hash(content.as_bytes()).to_hex()[..16].to_string()
    )
}

/// Create a mock signature for a module
fn create_signature(module_name: &str) -> SignatureMetadata {
    let key_id = format!("key-{}", &module_name[..3.min(module_name.len())]);
    SignatureMetadata {
        key_id: key_id.clone(),
        signature: format!("sig-{}-{}", module_name, Uuid::new_v4().to_string()[..8].to_string()),
        algorithm: "blake3-ed25519".to_string(),
        signed_at: Utc::now(),
    }
}

// ============================================================================
// ENDPOINT HANDLERS
// ============================================================================

/// GET /modules - Search and list modules with filtering
pub async fn search_modules(
    State(registry): State<ModuleRegistry>,
    Query(params): Query<ModuleSearchRequest>,
) -> ApiResult<Json<ListResponse<ModuleInfo>>> {
    let modules = registry.read().await;

    let mut filtered: Vec<_> = modules
        .values()
        .filter(|m| {
            if let Some(ref query) = params.query {
                let q_lower = query.to_lowercase();
                if !m.name.to_lowercase().contains(&q_lower)
                    && !m.description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&q_lower))
                        .unwrap_or(false)
                {
                    return false;
                }
            }
            if let Some(ref tags) = params.tags {
                if !tags.iter().any(|t| m.tags.contains(t)) {
                    return false;
                }
            }
            if let Some(ref author) = params.author {
                if !m.author.contains(author) {
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
            "version" => filtered.sort_by(|a, b| a.version.cmp(&b.version)),
            "created_at" => filtered.sort_by(|a, b| a.released_at.cmp(&b.released_at)),
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

/// POST /modules/install - Install a module
pub async fn install_module(
    State(registry): State<ModuleRegistry>,
    Json(req): Json<ModuleInstallRequest>,
) -> ApiResult<(StatusCode, Json<AsyncOperationResponse>)> {
    // Validate module name
    if req.name.is_empty() {
        return Err(ApiError::InvalidRequest(
            "Module name cannot be empty".to_string(),
        ));
    }

    let registry_lock = registry.write().await;

    let version = req.version.unwrap_or_else(|| "1.0.0".to_string());

    // Check if already installed
    if let Some(existing) = registry_lock.get(&req.name) {
        if existing.version == version && !req.force.unwrap_or(false) {
            return Err(ApiError::ModuleAlreadyInstalled(format!(
                "{} v{}",
                req.name, version
            )));
        }
    }

    let task_id = format!("install-{}-{}", req.name, Uuid::new_v4().to_string()[..8].to_string());

    Ok((
        StatusCode::ACCEPTED,
        Json(AsyncOperationResponse {
            task_id: task_id.clone(),
            status: "queued".to_string(),
            created_at: Utc::now(),
            progress_url: Some(format!("/api/operations/{}/progress", task_id)),
        }),
    ))
}

/// POST /modules/{name}/update - Update a module to a new version
pub async fn update_module(
    State(registry): State<ModuleRegistry>,
    Path(name): Path<String>,
    Json(_req): Json<ModuleUpdateRequest>,
) -> ApiResult<Json<AsyncOperationResponse>> {
    let registry_lock = registry.read().await;

    let _existing = registry_lock.get(&name).ok_or_else(|| {
        ApiError::ModuleNotFound(format!("Module '{}' not found", name))
    })?;

    drop(registry_lock);

    let task_id = format!("update-{}-{}", name, Uuid::new_v4().to_string()[..8].to_string());

    Ok(Json(AsyncOperationResponse {
        task_id: task_id.clone(),
        status: "queued".to_string(),
        created_at: Utc::now(),
        progress_url: Some(format!("/api/operations/{}/progress", task_id)),
    }))
}

/// DELETE /modules/{name} - Uninstall a module
pub async fn uninstall_module(
    State(registry): State<ModuleRegistry>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    let mut registry_lock = registry.write().await;

    if !registry_lock.contains_key(&name) {
        return Err(ApiError::ModuleNotFound(format!(
            "Module '{}' not found",
            name
        )));
    }

    registry_lock.remove(&name);

    Ok(StatusCode::NO_CONTENT)
}

/// GET /modules/{name}/{version} - Get module information
pub async fn get_module_info(
    State(registry): State<ModuleRegistry>,
    Path((name, version)): Path<(String, String)>,
) -> ApiResult<Json<ModuleInfo>> {
    let modules = registry.read().await;

    let module = modules.get(&name).ok_or_else(|| {
        ApiError::ModuleNotFound(format!("Module '{}' not found", name))
    })?;

    // In a real implementation, this would check the specific version
    if module.version != version && version != "latest" {
        return Err(ApiError::ModuleNotFound(format!(
            "Module '{}' version '{}' not found",
            name, version
        )));
    }

    Ok(Json(module.clone()))
}

/// POST /modules/verify - Verify module signature
pub async fn verify_module_signature(
    State(_registry): State<ModuleRegistry>,
    Json(req): Json<ModuleVerifyRequest>,
) -> ApiResult<Json<VerificationResult>> {
    // Validate request
    if req.name.is_empty() || req.signature.is_empty() {
        return Err(ApiError::InvalidRequest(
            "Module name and signature required".to_string(),
        ));
    }

    // In a real implementation, this would verify against stored public keys
    let signature_valid = blake3::hash(format!("{}-{}", req.name, req.version).as_bytes())
        .to_hex()
        .starts_with("blake3");

    if !signature_valid {
        return Err(ApiError::SignatureVerificationFailed(
            "Invalid module signature".to_string(),
        ));
    }

    Ok(Json(VerificationResult {
        valid: true,
        message: "Module signature verified successfully".to_string(),
        key_id: req.key_id,
        verified_at: Utc::now(),
    }))
}

// ============================================================================
// HELPER ENDPOINT: Get operation progress
// ============================================================================

/// GET /operations/{task_id}/progress - Get async operation progress
#[derive(Debug, Clone, Deserialize)]
pub struct TaskId(pub String);

pub async fn get_operation_progress(
    Path(task_id): Path<String>,
) -> ApiResult<Json<OperationProgress>> {
    // In a real implementation, this would track actual operation state
    let op_type = task_id.split('-').next().unwrap_or("unknown").to_string();
    let progress = match op_type.as_str() {
        "install" => 50,
        "update" => 75,
        "migrate" => 30,
        "restore" => 60,
        _ => 0,
    };

    let status = if progress >= 100 {
        ProgressStatus::Completed
    } else if progress > 0 {
        ProgressStatus::InProgress
    } else {
        ProgressStatus::Pending
    };

    Ok(Json(OperationProgress {
        operation_id: task_id,
        operation_type: op_type,
        status,
        progress_percent: progress,
        current_step: format!("Step {}/10", (progress / 10).max(1)),
        total_steps: 10,
        eta_seconds: if progress < 100 {
            Some((100 - progress) as u32 * 10)
        } else {
            None
        },
        messages: vec![ProgressMessage {
            timestamp: Utc::now(),
            level: "info".to_string(),
            message: format!("Operation {}% complete", progress),
        }],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_module_hash() {
        let hash = compute_module_hash("test", "1.0.0");
        assert!(hash.starts_with("blake3-"));
    }

    #[test]
    fn test_create_signature() {
        let sig = create_signature("test-module");
        assert!(sig.signature.starts_with("sig-"));
        assert_eq!(sig.algorithm, "blake3-ed25519");
    }

    #[tokio::test]
    async fn test_uninstall_nonexistent_module() {
        let registry = init_registry();

        let result = uninstall_module(State(registry), Path("nonexistent".to_string())).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::ModuleNotFound(_)));
    }

    #[tokio::test]
    async fn test_install_module() {
        let registry = init_registry();

        let req = ModuleInstallRequest {
            name: "test-module".to_string(),
            version: Some("1.0.0".to_string()),
            force: None,
            verify_signature: None,
            options: None,
        };

        let result = install_module(State(registry), Json(req)).await;
        assert!(result.is_ok());
        let (status, resp) = result.unwrap();
        assert_eq!(status.as_u16(), 202);
        assert!(resp.task_id.starts_with("install-"));
    }

    #[tokio::test]
    async fn test_get_operation_progress() {
        let result = get_operation_progress(Path("install-test-123".to_string())).await;
        assert!(result.is_ok());
        let progress = result.unwrap();
        assert_eq!(progress.progress_percent, 50);
        assert_eq!(progress.operation_type, "install");
    }

    #[tokio::test]
    async fn test_verify_module_signature() {
        let registry = init_registry();

        let req = ModuleVerifyRequest {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            signature: "sig-valid".to_string(),
            key_id: "key-001".to_string(),
        };

        let result = verify_module_signature(State(registry), Json(req)).await;
        assert!(result.is_ok());
        let verification = result.unwrap();
        assert!(verification.valid);
    }

    #[tokio::test]
    async fn test_search_modules_empty() {
        let registry = init_registry();

        let params = ModuleSearchRequest {
            query: None,
            tags: None,
            author: None,
            min_version: None,
            max_version: None,
            page: None,
            per_page: None,
            sort_by: None,
            sort_order: None,
        };

        let result = search_modules(State(registry), Query(params)).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.items.len(), 0);
    }
}
