use axum::{Json, extract::{State, Path}};
use crate::AppState;

pub async fn list_jobs(State(state): State<AppState>) -> Json<serde_json::Value> {
    let jobs = state.training_jobs.read().await;
    let job_list: Vec<serde_json::Value> = jobs
        .iter()
        .map(|j| {
            serde_json::json!({
                "id": j.id,
                "status": j.status,
                "progress": format!("{:.1}%", j.progress * 100.0),
                "current_stage": j.current_stage,
                "started_at": j.started_at,
                "estimated_completion": j.estimated_completion
            })
        })
        .collect();

    Json(serde_json::json!({
        "total_jobs": jobs.len(),
        "jobs": job_list
    }))
}

pub async fn job_logs(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Json<serde_json::Value> {
    let jobs = state.training_jobs.read().await;
    if let Some(job) = jobs.iter().find(|j| j.id == job_id) {
        Json(serde_json::json!({
            "job_id": job_id,
            "status": job.status,
            "logs": job.logs,
            "progress": format!("{:.1}%", job.progress * 100.0)
        }))
    } else {
        Json(serde_json::json!({"error": "Job not found"}))
    }
}
