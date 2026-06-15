//! Driver Converter API handlers
//! Provides endpoints for DIS to driver conversion and UMS installation

use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use log::{info, warn};

use crate::models::*;
use crate::error::ApiResult;

/// Driver converter service state
#[derive(Clone)]
pub struct DriverState {
    pub jobs: Arc<RwLock<std::collections::HashMap<ConversionJobId, ConversionResult>>>,
}

impl DriverState {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
}

impl Default for DriverState {
    fn default() -> Self {
        Self::new()
    }
}

/// POST /driver/convert - Convert DIS to driver
#[axum::debug_handler]
pub async fn convert_driver(
    State(state): State<Arc<DriverState>>,
    Json(req): Json<DriverConversionRequest>,
) -> ApiResult<impl IntoResponse> {
    info!(
        "Starting driver conversion: {} -> {:?}",
        req.dis_name, req.target_platform
    );

    if req.dis_content.is_empty() {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "DIS content cannot be empty"})),
        )
            .into_response());
    }

    let job_id = ConversionJobId::new();
    let optimization = req.optimization.unwrap_or_default();
    let background = req.background.unwrap_or(true);

    let result = ConversionResult {
        job_id,
        dis_name: req.dis_name.clone(),
        target_platform: req.target_platform.clone(),
        status: ConversionStatus::Queued,
        started_at: Utc::now(),
        completed_at: None,
        driver_binary: None,
        driver_checksum: None,
        compilation_log: None,
        warnings: vec![],
        errors: vec![],
    };

    {
        let mut jobs = state.jobs.write().await;
        jobs.insert(job_id, result);
    }

    info!(
        "Conversion job {} queued for {} (background: {})",
        job_id.0, req.dis_name, background
    );

    let state_clone = state.clone();
    let dis_content = req.dis_content.clone();
    let dis_name = req.dis_name.clone();
    let target_platform = req.target_platform.clone();

    if background {
        tokio::spawn(async move {
            execute_driver_conversion(
                state_clone,
                job_id,
                dis_name,
                dis_content,
                target_platform,
                optimization,
            )
            .await;
        });

        Ok((
            StatusCode::ACCEPTED,
            Json(json!({
                "job_id": job_id.0.to_string(),
                "status": "queued",
                "background": true,
                "timestamp": Utc::now().to_rfc3339(),
            })),
        )
            .into_response())
    } else {
        execute_driver_conversion(state.clone(), job_id, dis_name, dis_content, target_platform, optimization)
            .await;

        let jobs = state.jobs.read().await;
        let result = jobs.get(&job_id).cloned().unwrap_or_else(|| ConversionResult {
            job_id,
            dis_name: req.dis_name,
            target_platform: req.target_platform,
            status: ConversionStatus::Failed,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            driver_binary: None,
            driver_checksum: None,
            compilation_log: None,
            warnings: vec![],
            errors: vec!["Conversion failed".to_string()],
        });

        Ok((StatusCode::OK, Json(result)).into_response())
    }
}

/// GET /driver/results/{id} - Get conversion status
#[axum::debug_handler]
pub async fn get_conversion_result(
    State(state): State<Arc<DriverState>>,
    Path(job_id_str): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let job_id = ConversionJobId(Uuid::parse_str(&job_id_str).map_err(|_| {
        crate::error::ApiError::InvalidRequest("Invalid job ID format".to_string())
    })?);

    let jobs = state.jobs.read().await;

    match jobs.get(&job_id) {
        Some(result) => {
            info!("Retrieved conversion result for job {}", job_id.0);
            Ok((StatusCode::OK, Json(result.clone())).into_response())
        }
        None => {
            warn!("Conversion job not found: {}", job_id.0);
            Ok((
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": format!("Conversion job not found: {}", job_id.0)
                })),
            )
                .into_response())
        }
    }
}

/// POST /driver/{id}/install - Install driver to UMS
#[axum::debug_handler]
pub async fn install_driver(
    State(state): State<Arc<DriverState>>,
    Path(job_id_str): Path<String>,
    Json(req): Json<DriverInstallRequest>,
) -> ApiResult<impl IntoResponse> {
    let job_id = ConversionJobId(Uuid::parse_str(&job_id_str).map_err(|_| {
        crate::error::ApiError::InvalidRequest("Invalid job ID format".to_string())
    })?);

    info!("Installing driver from job {} version {}", job_id.0, req.version);

    let jobs = state.jobs.read().await;
    let conversion = jobs.get(&job_id).ok_or(crate::error::ApiError::InvalidRequest(
        format!("Conversion job not found: {}", job_id.0),
    ))?;

    if conversion.status != ConversionStatus::Completed {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("Conversion job not completed: {:?}", conversion.status)
            })),
        )
            .into_response());
    }

    if conversion.driver_binary.is_none() {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "No driver binary available"})),
        )
            .into_response());
    }

    let auto_activate = req.auto_activate.unwrap_or(true);
    let rollback_on_error = req.rollback_on_error.unwrap_or(true);

    info!(
        "Driver installation started: {}, auto_activate: {}, rollback: {}",
        conversion.dis_name, auto_activate, rollback_on_error
    );

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "installation_id": Uuid::new_v4().to_string(),
            "driver_name": conversion.dis_name,
            "version": req.version,
            "status": "installing",
            "auto_activate": auto_activate,
            "rollback_on_error": rollback_on_error,
            "timestamp": Utc::now().to_rfc3339(),
        })),
    )
        .into_response())
}

async fn execute_driver_conversion(
    state: Arc<DriverState>,
    job_id: ConversionJobId,
    dis_name: String,
    dis_content: String,
    target_platform: TargetPlatform,
    optimization: OptimizationFlags,
) {
    let start = std::time::Instant::now();
    let mut result = ConversionResult {
        job_id,
        dis_name: dis_name.clone(),
        target_platform: target_platform.clone(),
        status: ConversionStatus::Converting,
        started_at: Utc::now(),
        completed_at: None,
        driver_binary: None,
        driver_checksum: None,
        compilation_log: None,
        warnings: vec![],
        errors: vec![],
    };

    info!("Starting conversion process for {}", dis_name);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    result.warnings.push("Warning: Using mock DIS parser".to_string());

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    let mut log = "DIS to IR conversion started\n".to_string();

    {
        let mut jobs = state.jobs.write().await;
        result.status = ConversionStatus::Compiling;
        jobs.insert(job_id, result.clone());
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    log.push_str(&format!("Compiling for {:?}\n", target_platform));
    log.push_str("Codegen units: ");
    log.push_str(&optimization.codegen_units.to_string());
    log.push('\n');

    {
        let mut jobs = state.jobs.write().await;
        result.status = ConversionStatus::Optimizing;
        jobs.insert(job_id, result.clone());
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    if optimization.enable_lto {
        log.push_str("Enabling LTO (Link-Time Optimization)\n");
    }
    if optimization.vectorization {
        log.push_str("Enabling vectorization\n");
    }
    log.push_str(&format!(
        "Inline threshold: {}\n",
        optimization.inline_threshold
    ));

    let binary_data = generate_mock_binary(&dis_content);
    let checksum = blake3::hash(&binary_data).to_hex().to_string();

    log.push_str(&format!("Generated binary: {} bytes\n", binary_data.len()));
    log.push_str(&format!("Checksum (BLAKE3): {}\n", checksum));

    result.status = ConversionStatus::Completed;
    result.completed_at = Some(Utc::now());
    result.driver_binary = Some(binary_data);
    result.driver_checksum = Some(checksum);
    result.compilation_log = Some(log);

    {
        let mut jobs = state.jobs.write().await;
        jobs.insert(job_id, result);
    }

    info!(
        "Driver conversion completed for {} in {}ms",
        dis_name,
        start.elapsed().as_millis()
    );
}

fn generate_mock_binary(dis_content: &str) -> Vec<u8> {
    let mut binary = vec![0x7Fu8, 0x45, 0x4C, 0x46]; // ELF magic
    binary.extend(dis_content.as_bytes());
    binary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_driver_state_creation() {
        let state = DriverState::new();
        assert_eq!(state.jobs.read().await.len(), 0);
    }
}
