use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};
use uuid::Uuid;

use tauri::Manager;

use crate::gpu_controller::GpuController;
use crate::model_orchestrator::ModelOrchestrator;
use crate::AppState;

#[derive(Clone, Deserialize)]
pub struct CommandRequest {
    pub action: String,
    pub parameters: serde_json::Value,
}

#[derive(Clone, Serialize)]
pub struct JobStatus {
    pub id: String,
    pub action: String,
    pub status: String, // pending | running | completed | failed
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub started_at: u64,
    pub completed_at: Option<u64>,
}

pub struct OrchestratorState {
    pub jobs: Arc<Mutex<HashMap<String, JobStatus>>>,
    pub app_handle: tauri::AppHandle,
    pub gpu_controller: Arc<GpuController>,
    pub model_orchestrator: Arc<ModelOrchestrator>,
}

impl OrchestratorState {
    pub fn new(
        app_handle: tauri::AppHandle,
        gpu_controller: Arc<GpuController>,
        model_orchestrator: Arc<ModelOrchestrator>,
    ) -> Arc<Self> {
        Arc::new(Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
            app_handle,
            gpu_controller,
            model_orchestrator,
        })
    }

    pub async fn submit_job(self: &Arc<Self>, action: String, params: serde_json::Value) -> String {
        let id = Uuid::new_v4().to_string();
        let job = JobStatus {
            id: id.clone(),
            action: action.clone(),
            status: "pending".into(),
            result: None,
            error: None,
            started_at: chrono::Utc::now().timestamp() as u64,
            completed_at: None,
        };
        self.jobs.lock().await.insert(id.clone(), job);
        let state = Arc::clone(self);
        let id_for_spawn = id.clone();
        tokio::spawn(async move {
            state
                .execute_job(id_for_spawn, action.clone(), params)
                .await;
        });
        id
    }

    async fn update_job_result(&self, id: &str, res: Result<serde_json::Value, String>) {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.get_mut(id) {
            match res {
                Ok(v) => {
                    job.status = "completed".into();
                    job.result = Some(v);
                }
                Err(e) => {
                    job.status = "failed".into();
                    job.error = Some(e);
                }
            }
            job.completed_at = Some(chrono::Utc::now().timestamp() as u64);
        }
    }

    async fn execute_job(self: Arc<Self>, id: String, action: String, params: serde_json::Value) {
        // mark running
        {
            let mut jobs = self.jobs.lock().await;
            if let Some(job) = jobs.get_mut(&id) {
                job.status = "running".into();
            }
        }

        let res = match action.as_str() {
            "compile_launcher" => self.compile_launcher().await,
            "start_training" => self.start_training(params).await,
            "stop_training" => self.stop_training().await,
            "run_evaluation" => self.run_evaluation(params).await,
            "restart_bonsai" => self.restart_bonsai().await,
            "train_all_models_hours" => self.train_all_models_hours(params).await,
            _ => Err("unknown action".into()),
        };

        self.update_job_result(&id, res).await;
    }

    async fn compile_launcher(&self) -> Result<serde_json::Value, String> {
        // Attempt to run the provided PS1 script in workspace root (best-effort)
        let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../BonsaiExeLauncherBuilder.ps1");
        let script_str = script.to_string_lossy().to_string();
        let cwd = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        info!(script=%script_str, cwd=%cwd.display(), "orchestrator: running launcher builder");

        let output = tokio::process::Command::new("powershell")
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(script_str)
            .current_dir(cwd)
            .output()
            .await
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Ok(serde_json::json!({"status":"ok","stdout": stdout}))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(stderr)
        }
    }

    async fn start_training(
        &self,
        _params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        // Trigger an immediate cycle via AppState training loop
        let state = self.app_handle.state::<AppState>();
        let training = state.training.clone();
        training.loop_engine.trigger_now().await;
        Ok(serde_json::json!({"status":"triggered"}))
    }

    async fn stop_training(&self) -> Result<serde_json::Value, String> {
        let state = self.app_handle.state::<AppState>();
        // Best-effort: stop self-play if available
        let _ = state.self_play.clone().stop();
        Ok(serde_json::json!({"status":"stopped"}))
    }

    async fn run_evaluation(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let full = params
            .get("full")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let harness =
            crate::evaluation_harness::EvaluationHarness::new(self.model_orchestrator.clone());
        if full {
            let res = harness.run_full_harness().await;
            serde_json::to_value(res).map_err(|e| e.to_string())
        } else {
            let (ok, vec) = harness.run_core_check().await;
            Ok(serde_json::json!({"ok": ok, "results": vec}))
        }
    }

    async fn restart_bonsai(&self) -> Result<serde_json::Value, String> {
        // Attempt graceful restart: spawn a new process and exit
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let _ = tokio::task::spawn_blocking(move || {
            let _ = std::process::Command::new(exe).spawn();
            std::process::exit(0);
        })
        .await
        .map_err(|e| e.to_string())?;
        Ok(serde_json::json!({"status":"restarting"}))
    }

    async fn train_all_models_hours(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let duration_hours = params
            .get("duration_hours")
            .and_then(|v| v.as_u64())
            .unwrap_or(6);
        let state = self.app_handle.state::<AppState>();
        let _training = state.training.clone();
        // The eternal loop already runs in background; scheduling/rotation across models
        // is left for higher-level orchestration. For now, acknowledge the request.
        Ok(serde_json::json!({"status":"scheduled","duration_hours": duration_hours}))
    }
}

// --------- HTTP handlers and server start ----------
use axum::extract::State as AxState;

async fn submit_command(
    AxState(state): AxState<Arc<OrchestratorState>>,
    Json(req): Json<CommandRequest>,
) -> impl IntoResponse {
    let id = state.submit_job(req.action, req.parameters).await;
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "job_id": id })),
    )
}

async fn get_job_status(
    AxState(state): AxState<Arc<OrchestratorState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let jobs = state.jobs.lock().await;
    if let Some(job) = jobs.get(&id) {
        let val = serde_json::to_value(job.clone()).unwrap_or(serde_json::json!({}));
        (StatusCode::OK, Json(val))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Job not found" })),
        )
    }
}

pub async fn start_orchestrator(
    app_handle: tauri::AppHandle,
    gpu_controller: Arc<GpuController>,
    model_orchestrator: Arc<ModelOrchestrator>,
) {
    let state = OrchestratorState::new(app_handle.clone(), gpu_controller, model_orchestrator);
    let router = Router::new()
        .route("/control", post(submit_command))
        .route("/jobs/:id", get(get_job_status))
        .with_state(state.clone());

    // Bind to localhost:11380 using a TcpListener and axum::serve
    let listener = match tokio::net::TcpListener::bind(("127.0.0.1", 11380)).await {
        Ok(l) => l,
        Err(e) => {
            error!(error=%e, "orchestrator failed to bind");
            return;
        }
    };

    let server = axum::serve(listener, router).with_graceful_shutdown(async move {
        // Currently no shutdown signal; placeholder for future shutdown handling.
        futures::future::pending::<()>().await;
    });

    if let Err(e) = server.await {
        error!(error=%e, "orchestrator server failed");
    }
}
