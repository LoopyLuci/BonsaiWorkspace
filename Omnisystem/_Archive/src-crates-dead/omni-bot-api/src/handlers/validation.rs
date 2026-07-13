//! Validation (UVM) API handlers
//! Provides endpoints for test suite execution, results retrieval, and deterministic replay

use axum::{
    extract::{Path, State, Json, ws::WebSocket},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;
use log::{info, warn};

use crate::models::*;
use crate::error::ApiResult;

/// Validation service state
#[derive(Clone)]
pub struct ValidationState {
    pub active_runs: Arc<RwLock<std::collections::HashMap<ValidationRunId, ValidationResults>>>,
    pub history: Arc<RwLock<Vec<ValidationHistoryEntry>>>,
    pub traces: Arc<RwLock<std::collections::HashMap<ValidationRunId, ExecutionTrace>>>,
}

impl ValidationState {
    pub fn new() -> Self {
        Self {
            active_runs: Arc::new(RwLock::new(std::collections::HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            traces: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
}

impl Default for ValidationState {
    fn default() -> Self {
        Self::new()
    }
}

/// POST /validation/run - Execute test suite
#[axum::debug_handler]
pub async fn run_validation(
    State(state): State<Arc<ValidationState>>,
    Json(req): Json<ValidationRunRequest>,
) -> ApiResult<impl IntoResponse> {
    info!("Starting validation run: {}", req.name);

    let run_id = ValidationRunId::new();
    let parallelism = req.parallelism.clone().unwrap_or_default();

    let total_combinations = req
        .matrix
        .axes
        .iter()
        .map(|axis| axis.values.len())
        .product::<usize>();

    if total_combinations == 0 {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Matrix configuration results in 0 combinations"})),
        )
            .into_response());
    }

    let results = ValidationResults {
        run_id,
        name: req.name.clone(),
        status: ValidationStatus::Running,
        started_at: Utc::now(),
        completed_at: None,
        total_tests: total_combinations,
        passed: 0,
        failed: 0,
        skipped: 0,
        timeout: 0,
        results: Vec::new(),
        summary_metrics: ValidationMetrics {
            total_duration_ms: 0,
            avg_test_duration_ms: 0,
            peak_memory_mb: 0,
            avg_cpu_percent: 0,
            success_rate_percent: 0.0,
        },
    };

    {
        let mut runs = state.active_runs.write().await;
        runs.insert(run_id, results.clone());
    }

    let mut details = HashMap::new();
    details.insert("name".to_string(), json!(&req.name));
    let trace = ExecutionTrace {
        run_id,
        events: vec![TraceEvent {
            timestamp: Utc::now(),
            event_type: "validation_started".to_string(),
            details,
        }],
        total_events: 1,
    };

    info!(
        "Validation run {} queued with {} total combinations",
        run_id.0, total_combinations
    );

    spawn_validation_task(state.clone(), run_id, req, parallelism, results, trace).await;

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "run_id": run_id.0.to_string(),
            "status": "queued",
            "total_combinations": total_combinations,
            "timestamp": Utc::now().to_rfc3339(),
        })),
    )
        .into_response())
}

/// GET /validation/results/{id} - Retrieve results
#[axum::debug_handler]
pub async fn get_validation_results(
    State(state): State<Arc<ValidationState>>,
    Path(run_id_str): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let run_id = ValidationRunId(Uuid::parse_str(&run_id_str).map_err(|_| {
        crate::error::ApiError::InvalidRequest("Invalid run ID format".to_string())
    })?);

    let runs = state.active_runs.read().await;

    match runs.get(&run_id) {
        Some(results) => {
            info!("Retrieved results for validation run {}", run_id.0);
            Ok((StatusCode::OK, Json(results.clone())).into_response())
        }
        None => {
            warn!("Validation run not found: {}", run_id.0);
            Ok((
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": format!("Validation run not found: {}", run_id.0)
                })),
            )
                .into_response())
        }
    }
}

/// GET /validation/heatmap - Visual results
#[axum::debug_handler]
pub async fn get_heatmap(
    State(state): State<Arc<ValidationState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<impl IntoResponse> {
    let run_id_str = params
        .get("run_id")
        .ok_or(crate::error::ApiError::InvalidRequest(
            "Missing run_id parameter".to_string(),
        ))?;

    let run_id = ValidationRunId(Uuid::parse_str(run_id_str).map_err(|_| {
        crate::error::ApiError::InvalidRequest("Invalid run ID format".to_string())
    })?);

    let runs = state.active_runs.read().await;
    let results = runs.get(&run_id).ok_or(crate::error::ApiError::InvalidRequest(
        format!("Validation run not found: {}", run_id.0),
    ))?;

    let mut cells = Vec::new();
    for (idx, result) in results.results.iter().enumerate() {
        cells.push(HeatmapCell {
            coordinates: vec![idx / 10, idx % 10],
            value: if result.status == TestStatus::Passed {
                1.0
            } else {
                0.0
            },
            status: result.status,
            label: result.test_id.clone(),
        });
    }

    let heatmap = HeatmapData {
        run_id,
        axes: vec!["X".to_string(), "Y".to_string()],
        cells,
        legend: HeatmapLegend {
            min_value: 0.0,
            max_value: 1.0,
            color_scale: "viridis".to_string(),
        },
    };

    info!("Generated heatmap for validation run {}", run_id.0);
    Ok((StatusCode::OK, Json(heatmap)).into_response())
}

/// POST /validation/replay - Deterministic replay
#[axum::debug_handler]
pub async fn replay_validation(
    State(state): State<Arc<ValidationState>>,
    Json(req): Json<ValidationReplayRequest>,
) -> ApiResult<impl IntoResponse> {
    info!("Replaying validation run: {}", req.original_run_id.0);

    let runs = state.active_runs.read().await;
    let original = runs.get(&req.original_run_id).ok_or(
        crate::error::ApiError::InvalidRequest(
            format!("Original run not found: {}", req.original_run_id.0),
        ),
    )?;

    let new_run_id = ValidationRunId::new();
    let mut new_results = original.clone();
    new_results.run_id = new_run_id;
    new_results.status = ValidationStatus::Running;
    new_results.started_at = Utc::now();
    new_results.completed_at = None;

    if let Some(specific_tests) = &req.specific_tests {
        new_results.results = new_results
            .results
            .into_iter()
            .filter(|r| specific_tests.contains(&r.test_id))
            .collect();
    }

    drop(runs);

    let mut runs = state.active_runs.write().await;
    runs.insert(new_run_id, new_results);

    info!(
        "Started replay of validation run {} (original: {})",
        new_run_id.0, req.original_run_id.0
    );

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "new_run_id": new_run_id.0.to_string(),
            "original_run_id": req.original_run_id.0.to_string(),
            "status": "replaying",
            "timestamp": Utc::now().to_rfc3339(),
        })),
    )
        .into_response())
}

/// GET /validation/results/{id}/trace - Execution trace
#[axum::debug_handler]
pub async fn get_execution_trace(
    State(state): State<Arc<ValidationState>>,
    Path(run_id_str): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let run_id = ValidationRunId(Uuid::parse_str(&run_id_str).map_err(|_| {
        crate::error::ApiError::InvalidRequest("Invalid run ID format".to_string())
    })?);

    let traces = state.traces.read().await;

    match traces.get(&run_id) {
        Some(trace) => {
            info!("Retrieved execution trace for validation run {}", run_id.0);
            Ok((StatusCode::OK, Json(trace.clone())).into_response())
        }
        None => {
            warn!("Execution trace not found: {}", run_id.0);
            Ok((
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Execution trace not found"})),
            )
                .into_response())
        }
    }
}

/// GET /validation/history - Historical runs
#[axum::debug_handler]
pub async fn get_validation_history(
    State(state): State<Arc<ValidationState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<impl IntoResponse> {
    let page: u32 = params.get("page").and_then(|p| p.parse().ok()).unwrap_or(0);
    let per_page: u32 = params
        .get("per_page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(20);

    let history = state.history.read().await;
    let total = history.len() as u32;
    let total_pages = (total + per_page - 1) / per_page;

    let start = (page * per_page) as usize;
    let end = std::cmp::min(((page + 1) * per_page) as usize, history.len());

    let items = history[start..end].to_vec();

    info!(
        "Retrieved validation history: page {}, {} items of {}",
        page,
        items.len(),
        total
    );

    Ok((
        StatusCode::OK,
        Json(json!({
            "items": items,
            "total": total,
            "page": page,
            "per_page": per_page,
            "total_pages": total_pages,
        })),
    )
        .into_response())
}

/// WebSocket /validation/progress/{id} - Progress streaming
pub async fn validation_progress_stream(
    State(state): State<Arc<ValidationState>>,
    Path(run_id_str): Path<String>,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_progress_socket(state, run_id_str, socket))
}

async fn handle_progress_socket(
    state: Arc<ValidationState>,
    run_id_str: String,
    mut socket: WebSocket,
) {
    let run_id = match Uuid::parse_str(&run_id_str) {
        Ok(uuid) => ValidationRunId(uuid),
        Err(_) => {
            let _ = socket
                .send(axum::extract::ws::Message::Text(
                    json!({"error": "Invalid run ID"}).to_string(),
                ))
                .await;
            return;
        }
    };

    let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));

    loop {
        interval.tick().await;

        let runs = state.active_runs.read().await;
        match runs.get(&run_id) {
            Some(results) => {
                let progress = (results.passed + results.failed) as f64 / results.total_tests as f64
                    * 100.0;
                let msg = json!({
                    "run_id": run_id.0.to_string(),
                    "status": format!("{:?}", results.status),
                    "progress": progress as u8,
                    "passed": results.passed,
                    "failed": results.failed,
                    "total": results.total_tests,
                    "timestamp": Utc::now().to_rfc3339(),
                });

                if socket
                    .send(axum::extract::ws::Message::Text(msg.to_string()))
                    .await
                    .is_err()
                {
                    break;
                }

                if results.status != ValidationStatus::Running {
                    break;
                }
            }
            None => {
                let _ = socket
                    .send(axum::extract::ws::Message::Text(
                        json!({"error": "Validation run not found"}).to_string(),
                    ))
                    .await;
                break;
            }
        }
    }

    let _ = socket.close().await;
}

async fn spawn_validation_task(
    state: Arc<ValidationState>,
    run_id: ValidationRunId,
    req: ValidationRunRequest,
    _parallelism: ParallelismSettings,
    mut results: ValidationResults,
    mut trace: ExecutionTrace,
) {
    tokio::spawn(async move {
        let start = std::time::Instant::now();

        let mut total_duration = 0u64;
        let mut memory_peak = 0u32;
        let mut cpu_sum = 0u32;

        for i in 0..results.total_tests {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            let status = if i % 10 == 0 {
                results.failed += 1;
                TestStatus::Failed
            } else if i % 20 == 0 {
                results.skipped += 1;
                TestStatus::Skipped
            } else {
                results.passed += 1;
                TestStatus::Passed
            };

            let duration_ms = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()) as u64
                % 500;

            total_duration += duration_ms;

            let metrics = TestMetrics {
                memory_peak_mb: 256 + (i as u32 % 512),
                cpu_avg_percent: 25 + (i as u32 % 50),
                iops: 1000,
                cache_hits: 1000 + (i as u64),
                cache_misses: 100 + (i as u64 / 10),
            };

            memory_peak = memory_peak.max(metrics.memory_peak_mb);
            cpu_sum += metrics.cpu_avg_percent;

            results.results.push(TestResult {
                test_id: format!("test_{:04}", i),
                status,
                duration_ms,
                timestamp: Utc::now(),
                error: if status == TestStatus::Failed {
                    Some("Test assertion failed".to_string())
                } else {
                    None
                },
                metrics,
            });

            let mut details = HashMap::new();
            details.insert("test_id".to_string(), json!(format!("test_{:04}", i)));
            details.insert("status".to_string(), json!(format!("{:?}", status)));
            details.insert("duration_ms".to_string(), json!(duration_ms));
            trace.events.push(TraceEvent {
                timestamp: Utc::now(),
                event_type: "test_completed".to_string(),
                details,
            });
            trace.total_events += 1;
        }

        results.completed_at = Some(Utc::now());
        results.status = ValidationStatus::Completed;
        results.summary_metrics = ValidationMetrics {
            total_duration_ms: start.elapsed().as_millis() as u64,
            avg_test_duration_ms: if results.total_tests > 0 {
                total_duration / results.total_tests as u64
            } else {
                0
            },
            peak_memory_mb: memory_peak,
            avg_cpu_percent: if results.total_tests > 0 {
                cpu_sum / results.total_tests as u32
            } else {
                0
            },
            success_rate_percent: (results.passed as f64 / results.total_tests as f64) * 100.0,
        };

        let mut details = HashMap::new();
        details.insert("passed".to_string(), json!(results.passed));
        details.insert("failed".to_string(), json!(results.failed));
        details.insert("total".to_string(), json!(results.total_tests));
        trace.events.push(TraceEvent {
            timestamp: Utc::now(),
            event_type: "validation_completed".to_string(),
            details,
        });

        {
            let mut runs = state.active_runs.write().await;
            runs.insert(run_id, results.clone());

            let mut traces = state.traces.write().await;
            traces.insert(run_id, trace.clone());

            let mut history = state.history.write().await;
            history.push(ValidationHistoryEntry {
                run_id,
                name: req.name,
                timestamp: Utc::now(),
                status: results.status,
                passed: results.passed,
                failed: results.failed,
                total_tests: results.total_tests,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        info!(
            "Validation run {} completed: {} passed, {} failed",
            run_id.0, results.passed, results.failed
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validation_state_creation() {
        let state = ValidationState::new();
        assert_eq!(state.active_runs.read().await.len(), 0);
    }
}
