use axum::{Json, extract::{State, Path}};
use serde::Deserialize;
use crate::{AppState, TrainingJob};

#[derive(Debug, Deserialize)]
pub struct TrainingRequest {
    pub config_path: String,
    pub stages: Vec<u32>,
    pub gpu_count: Option<u32>,
    pub dataset_id: Option<String>,
}

pub async fn start_training(
    State(state): State<AppState>,
    Json(req): Json<TrainingRequest>,
) -> Json<serde_json::Value> {
    let job_id = uuid::Uuid::new_v4().to_string();

    let job = TrainingJob {
        id: job_id.clone(),
        config: req.config_path.clone(),
        status: "queued".into(),
        progress: 0.0,
        current_stage: req.stages.first().copied().unwrap_or(1),
        started_at: chrono::Utc::now().to_rfc3339(),
        estimated_completion: "Calculating...".into(),
        logs: vec!["🚀 Job queued.".into()],
    };

    state.training_jobs.write().await.push(job);

    Json(serde_json::json!({
        "status": "queued",
        "job_id": job_id,
        "stages": req.stages,
        "message": "Training job started"
    }))
}

pub async fn job_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Json<serde_json::Value> {
    let jobs = state.training_jobs.read().await;
    if let Some(job) = jobs.iter().find(|j| j.id == job_id) {
        Json(serde_json::to_value(job).unwrap())
    } else {
        Json(serde_json::json!({"error": "Job not found"}))
    }
}

pub async fn cancel_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Json<serde_json::Value> {
    let mut jobs = state.training_jobs.write().await;
    if let Some(job) = jobs.iter_mut().find(|j| j.id == job_id) {
        job.status = "cancelled".into();
        job.logs.push("⏹ Job cancelled by user.".into());
    }
    Json(serde_json::json!({
        "status": "cancelled",
        "job_id": job_id
    }))
}
