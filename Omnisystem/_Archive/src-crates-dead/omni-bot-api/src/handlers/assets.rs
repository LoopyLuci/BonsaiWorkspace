//! Asset management API handlers
//! Provides endpoints for AI-powered asset generation, processing, and publishing

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

/// Asset storage and management
#[derive(Clone)]
pub struct AssetStore {
    assets: Arc<RwLock<HashMap<String, AssetInfo>>>,
    generation_tasks: Arc<RwLock<HashMap<String, AssetProgress>>>,
}

impl AssetStore {
    pub fn new() -> Self {
        Self {
            assets: Arc::new(RwLock::new(HashMap::new())),
            generation_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get_asset(&self, id: &str) -> Option<AssetInfo> {
        self.assets.read().await.get(id).cloned()
    }

    async fn save_asset(&self, asset: AssetInfo) {
        self.assets.write().await.insert(asset.spec.id.clone(), asset);
    }

    async fn delete_asset(&self, id: &str) -> bool {
        self.assets.write().await.remove(id).is_some()
    }

    async fn list_all_assets(&self, page: usize, per_page: usize) -> (Vec<AssetInfo>, usize) {
        let assets = self.assets.read().await;
        let total = assets.len();
        let start = page * per_page;
        let end = (start + per_page).min(total);

        let mut items: Vec<_> = assets.values().cloned().collect();
        items.sort_by(|a, b| b.spec.created_at.cmp(&a.spec.created_at));

        (items[start..end].to_vec(), total)
    }

    async fn search_assets(
        &self,
        query: &str,
        tags: &[String],
        page: usize,
        per_page: usize,
    ) -> (Vec<AssetInfo>, usize) {
        let assets = self.assets.read().await;
        let query_lower = query.to_lowercase();

        let mut filtered: Vec<_> = assets
            .values()
            .filter(|asset| {
                let matches_query = asset.spec.description.to_lowercase().contains(&query_lower)
                    || asset.spec.asset_type.to_lowercase().contains(&query_lower);
                let matches_tags = tags.is_empty()
                    || tags.iter().any(|t| asset.tags.contains(t));
                matches_query && matches_tags
            })
            .cloned()
            .collect();

        let total = filtered.len();
        filtered.sort_by(|a, b| b.spec.created_at.cmp(&a.spec.created_at));

        let start = page * per_page;
        let end = (start + per_page).min(total);

        (filtered[start..end].to_vec(), total)
    }

    async fn set_progress(&self, asset_id: String, progress: AssetProgress) {
        self.generation_tasks
            .write()
            .await
            .insert(asset_id, progress);
    }

    async fn get_progress(&self, asset_id: &str) -> Option<AssetProgress> {
        self.generation_tasks.read().await.get(asset_id).cloned()
    }
}

/// Query parameters for asset listing
#[derive(Debug, Deserialize)]
pub struct AssetListQuery {
    pub search: Option<String>,
    pub tags: Option<String>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

/// Initialize asset store
pub fn init_asset_store() -> AssetStore {
    log::info!("Initializing Asset Store");
    AssetStore::new()
}

/// Generate asset with AI (POST /assets/generate)
/// Creates new asset with progress tracking via WebSocket
pub async fn generate_asset(
    State(store): State<AssetStore>,
    Json(req): Json<AssetGenerationRequest>,
) -> ApiResult<(StatusCode, Json<AssetInfo>)> {
    // Validate request
    if req.asset_type.is_empty() {
        return Err(ApiError::InvalidAssetType("Asset type required".to_string()));
    }
    if req.description.is_empty() {
        return Err(ApiError::InvalidParameter("Description required".to_string()));
    }

    let asset_id = Uuid::new_v4().to_string();
    let quality = req.quality.unwrap_or(AssetQuality::Medium);
    let now = Utc::now();

    // Initialize progress tracking
    let progress = AssetProgress {
        asset_id: asset_id.clone(),
        stage: AssetGenerationStage::Queued,
        progress_percent: 0,
        message: "Queued for generation".to_string(),
        estimated_time_remaining_secs: Some(30),
        error: None,
    };
    store.set_progress(asset_id.clone(), progress).await;

    // Simulate asset generation with HDE integration
    let asset_spec = AssetSpec {
        id: asset_id.clone(),
        asset_type: req.asset_type.clone(),
        description: req.description.clone(),
        style: req.style.clone(),
        quality,
        size_bytes: estimate_size(quality),
        format: infer_format(&req.asset_type),
        width: Some(1024),
        height: Some(1024),
        duration_ms: None,
        metadata: req.metadata.unwrap_or_default(),
        created_at: now,
        updated_at: now,
    };

    let checksum = blake3::hash(asset_spec.id.as_bytes()).to_hex().to_string()[..16].to_string();

    let asset_info = AssetInfo {
        spec: asset_spec,
        preview_url: format!("/assets/{}/preview", asset_id),
        download_url: format!("/assets/{}/download", asset_id),
        published_to_ums: false,
        ums_reference_id: None,
        tags: vec![req.asset_type.clone()],
        checksum,
    };

    store.save_asset(asset_info.clone()).await;

    log::info!("Generated asset: {}", asset_id);
    Ok((StatusCode::CREATED, Json(asset_info)))
}

/// Get asset metadata and preview (GET /assets/{id})
pub async fn get_asset(
    State(store): State<AssetStore>,
    Path(id): Path<String>,
) -> ApiResult<Json<AssetInfo>> {
    store
        .get_asset(&id)
        .await
        .map(Json)
        .ok_or_else(|| ApiError::AssetNotFound(id))
}

/// Publish asset to UMS (POST /assets/{id}/publish)
/// Adds asset to Universal Module System with visibility settings
pub async fn publish_asset(
    State(store): State<AssetStore>,
    Path(id): Path<String>,
    Json(publish_req): Json<AssetPublishRequest>,
) -> ApiResult<Json<AssetInfo>> {
    let mut asset = store
        .get_asset(&id)
        .await
        .ok_or_else(|| ApiError::AssetNotFound(id.clone()))?;

    if asset.published_to_ums {
        return Err(ApiError::AssetAlreadyExists(
            "Asset already published".to_string(),
        ));
    }

    // Simulate UMS publishing with visibility settings
    let ums_id = format!("ums-{}", Uuid::new_v4());
    asset.published_to_ums = true;
    asset.ums_reference_id = Some(ums_id);

    if let Some(tags) = publish_req.tags {
        asset.tags.extend(tags);
        asset.tags.sort();
        asset.tags.dedup();
    }

    if let Some(metadata) = publish_req.metadata {
        asset.spec.metadata.extend(metadata);
    }

    asset.spec.updated_at = Utc::now();
    store.save_asset(asset.clone()).await;

    log::info!(
        "Published asset {} with visibility: {:?}",
        id,
        publish_req.visibility
    );
    Ok(Json(asset))
}

/// Batch asset operations (POST /assets/batch)
/// Perform resize, convert, optimize on multiple assets
pub async fn batch_asset_operation(
    State(store): State<AssetStore>,
    Json(batch_req): Json<BatchAssetOperation>,
) -> ApiResult<Json<BatchOperationResult>> {
    let operation_id = Uuid::new_v4().to_string();
    let total = batch_req.asset_ids.len();
    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    for asset_id in &batch_req.asset_ids {
        match store.get_asset(asset_id).await {
            Some(mut asset) => {
                match apply_batch_operation(&mut asset, &batch_req.operation, &batch_req.parameters) {
                    Ok(_) => {
                        asset.spec.updated_at = Utc::now();
                        store.save_asset(asset.clone()).await;
                        results.push(AssetOperationResult {
                            asset_id: asset_id.clone(),
                            success: true,
                            message: "Operation completed successfully".to_string(),
                            result: Some(asset),
                        });
                        succeeded += 1;
                    }
                    Err(msg) => {
                        results.push(AssetOperationResult {
                            asset_id: asset_id.clone(),
                            success: false,
                            message: msg,
                            result: None,
                        });
                        failed += 1;
                    }
                }
            }
            None => {
                results.push(AssetOperationResult {
                    asset_id: asset_id.clone(),
                    success: false,
                    message: "Asset not found".to_string(),
                    result: None,
                });
                failed += 1;
            }
        }
    }

    log::info!(
        "Batch operation {}: {} succeeded, {} failed",
        operation_id,
        succeeded,
        failed
    );

    Ok(Json(BatchOperationResult {
        operation_id,
        total,
        succeeded,
        failed,
        results,
    }))
}

/// List and search assets (GET /assets)
/// Filter by search query, tags, pagination
pub async fn list_assets(
    State(store): State<AssetStore>,
    Query(params): Query<AssetListQuery>,
) -> ApiResult<Json<AssetListResponse>> {
    let page = params.page.unwrap_or(0);
    let per_page = params.per_page.unwrap_or(20).min(100);

    let (assets, total) = if let Some(search) = &params.search {
        let tags: Vec<String> = params
            .tags
            .as_ref()
            .map(|t| t.split(',').map(|s| s.to_string()).collect())
            .unwrap_or_default();
        store.search_assets(search, &tags, page, per_page).await
    } else {
        store.list_all_assets(page, per_page).await
    };

    Ok(Json(AssetListResponse {
        assets,
        total,
        page,
        per_page,
    }))
}

/// Delete asset (DELETE /assets/{id})
pub async fn delete_asset(
    State(store): State<AssetStore>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if store.delete_asset(&id).await {
        log::info!("Deleted asset: {}", id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::AssetNotFound(id))
    }
}

/// Get asset preview thumbnail (GET /assets/{id}/preview)
pub async fn get_asset_preview(
    State(store): State<AssetStore>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Vec<u8>)> {
    let asset = store
        .get_asset(&id)
        .await
        .ok_or_else(|| ApiError::AssetNotFound(id))?;

    // Simulate preview generation (would be actual image processing)
    let preview_data = generate_preview_data(&asset);
    Ok((StatusCode::OK, preview_data))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Estimate asset size based on quality
fn estimate_size(quality: AssetQuality) -> u64 {
    match quality {
        AssetQuality::Low => 512_000,
        AssetQuality::Medium => 2_048_000,
        AssetQuality::High => 8_192_000,
        AssetQuality::Ultra => 32_768_000,
    }
}

/// Infer asset format from type
fn infer_format(asset_type: &str) -> String {
    match asset_type.to_lowercase().as_str() {
        "image" => "png".to_string(),
        "video" => "mp4".to_string(),
        "audio" => "mp3".to_string(),
        "3d_model" => "gltf".to_string(),
        _ => "bin".to_string(),
    }
}

/// Apply batch operation to asset
fn apply_batch_operation(
    asset: &mut AssetInfo,
    operation: &BatchOperation,
    _params: &HashMap<String, serde_json::Value>,
) -> Result<(), String> {
    match operation {
        BatchOperation::Resize { width, height } => {
            asset.spec.width = Some(*width);
            asset.spec.height = Some(*height);
            asset.spec.size_bytes = ((*width as u64) * (*height as u64) * 4) / 2; // Approx
            Ok(())
        }
        BatchOperation::Convert { format } => {
            asset.spec.format = format.clone();
            Ok(())
        }
        BatchOperation::Optimize { compression } => {
            asset.spec.size_bytes = (asset.spec.size_bytes * (*compression as u64)) / 100;
            Ok(())
        }
        BatchOperation::ApplyFilter { filter_name } => {
            asset
                .spec
                .metadata
                .insert("filter".to_string(), serde_json::json!(filter_name));
            Ok(())
        }
        BatchOperation::Tag { tags } => {
            asset.tags.extend(tags.clone());
            asset.tags.sort();
            asset.tags.dedup();
            Ok(())
        }
        BatchOperation::Delete => {
            Err("Delete operation handled separately".to_string())
        }
    }
}

/// Generate preview thumbnail data
fn generate_preview_data(_asset: &AssetInfo) -> Vec<u8> {
    // Simulate preview generation - return minimal PNG header
    vec![
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, // PNG signature
        0x00, 0x00, 0x00, 0x0d, // IHDR length
        0x49, 0x48, 0x44, 0x52, // IHDR
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_asset_generation_request_validation() {
        let store = AssetStore::new();
        let req = AssetGenerationRequest {
            asset_type: "image".to_string(),
            description: "Test image".to_string(),
            style: Some("realistic".to_string()),
            quality: Some(AssetQuality::High),
            metadata: None,
        };

        let result = generate_asset(State(store), Json(req)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_asset_quality_scale_factor() {
        assert_eq!(AssetQuality::Low.scale_factor(), 0.5);
        assert_eq!(AssetQuality::Ultra.scale_factor(), 2.0);
    }

    #[tokio::test]
    async fn test_batch_operation_resize() {
        let mut asset = AssetInfo {
            spec: AssetSpec {
                id: "test".to_string(),
                asset_type: "image".to_string(),
                description: "test".to_string(),
                style: None,
                quality: AssetQuality::Medium,
                size_bytes: 1024,
                format: "png".to_string(),
                width: Some(512),
                height: Some(512),
                duration_ms: None,
                metadata: HashMap::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            preview_url: "/preview".to_string(),
            download_url: "/download".to_string(),
            published_to_ums: false,
            ums_reference_id: None,
            tags: vec![],
            checksum: "abc".to_string(),
        };

        let op = BatchOperation::Resize {
            width: 256,
            height: 256,
        };
        let result = apply_batch_operation(&mut asset, &op, &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(asset.spec.width, Some(256));
        assert_eq!(asset.spec.height, Some(256));
    }

    #[test]
    fn test_format_inference() {
        assert_eq!(infer_format("image"), "png");
        assert_eq!(infer_format("video"), "mp4");
        assert_eq!(infer_format("audio"), "mp3");
        assert_eq!(infer_format("3d_model"), "gltf");
    }

    #[test]
    fn test_size_estimation() {
        assert!(estimate_size(AssetQuality::Ultra) > estimate_size(AssetQuality::Low));
    }
}
