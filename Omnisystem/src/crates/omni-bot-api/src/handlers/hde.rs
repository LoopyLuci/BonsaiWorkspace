//! HDE (Hybrid Deterministic Execution) Management API handlers
//! Provides endpoints for AI model promotion, demotion, and safety validation

use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use log::info;

use crate::models::*;
use crate::error::ApiResult;

/// HDE management service state
#[derive(Clone)]
pub struct HdeState {
    pub models: Arc<RwLock<std::collections::HashMap<String, ModelInfo>>>,
    pub shadow_reports: Arc<RwLock<std::collections::HashMap<String, ShadowValidationReport>>>,
}

impl HdeState {
    pub fn new() -> Self {
        Self {
            models: Arc::new(RwLock::new(std::collections::HashMap::new())),
            shadow_reports: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn with_defaults() -> Self {
        let state = Self::new();
        let mut models = state.models.write().await;

        models.insert(
            "claude-v3.1".to_string(),
            ModelInfo {
                id: ModelId("claude-v3.1".to_string()),
                name: "Claude v3.1".to_string(),
                version: "3.1.0".to_string(),
                state: ModelState::Active,
                created_at: Utc::now(),
                last_updated: Utc::now(),
                metrics: ModelMetrics {
                    accuracy: 0.98,
                    latency_ms: 45.0,
                    throughput_rps: 1200.0,
                    error_rate: 0.001,
                    resource_efficiency: 0.92,
                },
                safety_envelope: SafetyEnvelope {
                    max_context_length: 200000,
                    allowed_operations: vec![
                        "text_generation".to_string(),
                        "code_analysis".to_string(),
                    ],
                    resource_limits: ResourceLimits::default(),
                    validation_required: true,
                },
            },
        );

        models.insert(
            "claude-v3.2-shadow".to_string(),
            ModelInfo {
                id: ModelId("claude-v3.2-shadow".to_string()),
                name: "Claude v3.2 (Shadow)".to_string(),
                version: "3.2.0-rc1".to_string(),
                state: ModelState::Shadow,
                created_at: Utc::now(),
                last_updated: Utc::now(),
                metrics: ModelMetrics {
                    accuracy: 0.985,
                    latency_ms: 42.0,
                    throughput_rps: 1300.0,
                    error_rate: 0.0008,
                    resource_efficiency: 0.94,
                },
                safety_envelope: SafetyEnvelope {
                    max_context_length: 200000,
                    allowed_operations: vec![
                        "text_generation".to_string(),
                        "code_analysis".to_string(),
                    ],
                    resource_limits: ResourceLimits::default(),
                    validation_required: true,
                },
            },
        );

        drop(models);
        state
    }
}

impl Default for HdeState {
    fn default() -> Self {
        Self::new()
    }
}

/// GET /hde/models - List all AI models
#[axum::debug_handler]
pub async fn list_models(State(state): State<Arc<HdeState>>) -> ApiResult<impl IntoResponse> {
    info!("Listing all AI models");

    let models = state.models.read().await;
    let model_list: Vec<ModelInfo> = models.values().cloned().collect();

    Ok((
        StatusCode::OK,
        Json(json!({
            "models": model_list,
            "total": model_list.len(),
            "timestamp": Utc::now().to_rfc3339(),
        })),
    )
        .into_response())
}

/// POST /hde/models/{name}/promote - Promote model to active
#[axum::debug_handler]
pub async fn promote_model(
    State(state): State<Arc<HdeState>>,
    Path(model_name): Path<String>,
    Json(req): Json<ModelPromoteRequest>,
) -> ApiResult<impl IntoResponse> {
    info!(
        "Promoting model {} to active (validation_passed: {})",
        model_name, req.validation_passed
    );

    if !req.validation_passed {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Cannot promote model without passing validation"
            })),
        )
            .into_response());
    }

    let mut models = state.models.write().await;
    let shadow_reports = state.shadow_reports.read().await;

    let shadow_key = format!("{}-shadow", model_name);
    let _model = models
        .get_mut(&shadow_key)
        .ok_or(crate::error::ApiError::InvalidRequest(
            format!("Shadow model not found: {}", shadow_key),
        ))?;

    let validation = shadow_reports.get(&shadow_key).ok_or(
        crate::error::ApiError::InvalidRequest("Validation report not found".to_string()),
    )?;

    if !validation.ready_for_promotion {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Model failed safety validation",
                "violations": validation.safety_violations.len()
            })),
        )
            .into_response());
    }

    // Update previous active model to deprecated
    for model_info in models.values_mut() {
        if model_info.state == ModelState::Active {
            model_info.state = ModelState::Deprecated;
            model_info.last_updated = Utc::now();
            break;
        }
    }

    // Now update the shadow model to active
    let model = models
        .get_mut(&shadow_key)
        .expect("model already checked above");
    model.state = ModelState::Active;
    model.last_updated = Utc::now();

    let promoted_model = model.clone();
    drop(models);
    drop(shadow_reports);

    info!(
        "Model {} promoted to active (rollout: {}%)",
        model_name,
        req.rollout_percentage.unwrap_or(100)
    );

    Ok((
        StatusCode::OK,
        Json(json!({
            "model": promoted_model,
            "previous_state": "shadow",
            "new_state": "active",
            "rollout_percentage": req.rollout_percentage.unwrap_or(100),
            "timestamp": Utc::now().to_rfc3339(),
        })),
    )
        .into_response())
}

/// POST /hde/models/{name}/demote - Demote model from active
#[axum::debug_handler]
pub async fn demote_model(
    State(state): State<Arc<HdeState>>,
    Path(model_name): Path<String>,
    Json(req): Json<ModelDemoteRequest>,
) -> ApiResult<impl IntoResponse> {
    info!("Demoting model {} (reason: {})", model_name, req.reason);

    let mut models = state.models.write().await;

    let model = models
        .get_mut(&model_name)
        .ok_or(crate::error::ApiError::InvalidRequest(
            format!("Model not found: {}", model_name),
        ))?;

    if model.state != ModelState::Active {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("Can only demote active models, current state: {:?}", model.state)
            })),
        )
            .into_response());
    }

    let preserve_shadow = req.preserve_shadow.unwrap_or(true);
    let new_state = if preserve_shadow {
        ModelState::Shadow
    } else {
        ModelState::Archived
    };

    model.state = new_state;
    model.last_updated = Utc::now();

    let demoted_model = model.clone();
    drop(models);

    info!(
        "Model {} demoted from active to {:?}",
        model_name, new_state
    );

    Ok((
        StatusCode::OK,
        Json(json!({
            "model": demoted_model,
            "previous_state": "active",
            "new_state": format!("{:?}", new_state).to_lowercase(),
            "reason": req.reason,
            "preserve_shadow": preserve_shadow,
            "timestamp": Utc::now().to_rfc3339(),
        })),
    )
        .into_response())
}

/// GET /hde/shadow-reports - Get validation reports
#[axum::debug_handler]
pub async fn get_shadow_reports(
    State(state): State<Arc<HdeState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<impl IntoResponse> {
    let model_filter = params.get("model");

    info!("Retrieving shadow validation reports");

    let shadow_reports = state.shadow_reports.read().await;

    let reports: Vec<ShadowValidationReport> = if let Some(model_name) = model_filter {
        shadow_reports
            .values()
            .filter(|r| r.model_id.0.contains(model_name))
            .cloned()
            .collect()
    } else {
        shadow_reports.values().cloned().collect()
    };

    let total_violations = reports.iter().map(|r| r.safety_violations.len()).sum::<usize>();
    let critical_violations = reports
        .iter()
        .flat_map(|r| &r.safety_violations)
        .filter(|v| v.severity == ViolationSeverity::Critical)
        .count();

    Ok((
        StatusCode::OK,
        Json(json!({
            "reports": reports,
            "total_reports": reports.len(),
            "total_violations": total_violations,
            "critical_violations": critical_violations,
            "all_ready": reports.iter().all(|r| r.ready_for_promotion),
            "timestamp": Utc::now().to_rfc3339(),
        })),
    )
        .into_response())
}

/// POST /hde/models/{name}/validate - Trigger validation
#[axum::debug_handler]
pub async fn validate_shadow_model(
    State(state): State<Arc<HdeState>>,
    Path(model_name): Path<String>,
) -> ApiResult<impl IntoResponse> {
    info!("Starting safety validation for shadow model: {}", model_name);

    let models = state.models.read().await;
    let shadow_key = format!("{}-shadow", model_name);

    let model = models.get(&shadow_key).ok_or(
        crate::error::ApiError::InvalidRequest(
            format!("Shadow model not found: {}", shadow_key),
        ),
    )?;

    if model.state != ModelState::Shadow {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("Model is not in shadow state: {:?}", model.state)
            })),
        )
            .into_response());
    }

    let model_clone = model.clone();
    let state_clone = state.clone();
    let shadow_key_clone = shadow_key.clone();

    drop(models);

    tokio::spawn(async move {
        execute_shadow_validation(state_clone, shadow_key_clone, model_clone).await;
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "model": shadow_key,
            "validation_status": "running",
            "timestamp": Utc::now().to_rfc3339(),
        })),
    )
        .into_response())
}

async fn execute_shadow_validation(
    state: Arc<HdeState>,
    model_key: String,
    model: ModelInfo,
) {
    info!("Executing safety validation for {}", model_key);

    let start = std::time::Instant::now();
    let mut violations = Vec::new();
    let mut tests_passed = 0;
    let mut tests_failed = 0;

    let test_count = 50;
    for i in 0..test_count {
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        if i % 10 == 0 {
            tests_failed += 1;
            let mut context_map = std::collections::HashMap::new();
            context_map.insert("test_id".to_string(), json!(format!("safety_test_{:03}", i)));
            context_map.insert("type".to_string(), json!("bounds_check_failure"));
            violations.push(SafetyViolation {
                severity: if i % 20 == 0 {
                    ViolationSeverity::Critical
                } else {
                    ViolationSeverity::Warning
                },
                description: format!("Safety violation in test {}", i),
                context: context_map,
            });
        } else {
            tests_passed += 1;
        }
    }

    let performance_deltas = PerformanceDeltas {
        accuracy_delta: 0.005,
        latency_delta_ms: -3.0,
        throughput_delta_rps: 100.0,
        error_rate_delta: -0.0002,
    };

    let ready_for_promotion = violations
        .iter()
        .filter(|v| v.severity == ViolationSeverity::Critical)
        .count()
        == 0;

    let report = ShadowValidationReport {
        model_id: model.id,
        model_version: model.version.clone(),
        validation_timestamp: Utc::now(),
        tests_run: test_count,
        tests_passed,
        tests_failed,
        safety_violations: violations,
        performance_deltas,
        ready_for_promotion,
    };

    {
        let mut reports = state.shadow_reports.write().await;
        reports.insert(model_key.clone(), report.clone());
    }

    info!(
        "Safety validation completed for {} in {}ms: {} passed, {} failed, ready: {}",
        model_key,
        start.elapsed().as_millis(),
        report.tests_passed,
        report.tests_failed,
        report.ready_for_promotion
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hde_state_creation() {
        let state = HdeState::new();
        assert_eq!(state.models.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_hde_state_with_defaults() {
        let state = HdeState::with_defaults().await;
        let models = state.models.read().await;
        assert!(models.len() > 0);
    }
}
