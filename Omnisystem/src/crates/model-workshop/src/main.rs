use axum::{
    Router,
    routing::{get, post, put, delete},
};
use model_workshop::{
    AppState, ModuleInfo, DatasetInfo, TrainingJob, ModelInfo,
    library, datasets, designer, builder, editor, converter, monitor,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let state = AppState {
        modules: Arc::new(RwLock::new(HashMap::new())),
        datasets: Arc::new(RwLock::new(HashMap::new())),
        training_jobs: Arc::new(RwLock::new(Vec::new())),
        models: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = Router::new()
        // Health check
        .route("/health", get(|| async { "OK" }))

        // Module Library endpoints
        .route("/api/modules", get(library::list_modules))
        .route("/api/modules", post(library::create_module))
        .route("/api/modules/:id", get(library::get_module))
        .route("/api/modules/:id", put(library::update_module))
        .route("/api/modules/:id", delete(library::delete_module))
        .route("/api/modules/:id/chunks", post(library::add_chunk))
        .route("/api/modules/:id/chunks/:chunk_id", delete(library::remove_chunk))

        // Dataset endpoints
        .route("/api/datasets", get(datasets::list_datasets))
        .route("/api/datasets", post(datasets::create_dataset))
        .route("/api/datasets/:id", delete(datasets::delete_dataset))
        .route("/api/datasets/:id/import", post(datasets::import_data))

        // Model Designer endpoints
        .route("/api/models/design", post(designer::create_config))
        .route("/api/models/design/validate", post(designer::validate_config))

        // Model Builder endpoints
        .route("/api/models/build", post(builder::start_training))
        .route("/api/models/build/:job_id", get(builder::job_status))
        .route("/api/models/build/:job_id", delete(builder::cancel_job))

        // Model Editor endpoints
        .route("/api/models/edit/:id", put(editor::edit_model))
        .route("/api/models/merge-lora", post(editor::merge_lora))

        // Model Converter endpoints
        .route("/api/models/convert", post(converter::convert_model))
        .route("/api/models/quantize", post(converter::quantize_model))

        // Training Monitor endpoints
        .route("/api/training/jobs", get(monitor::list_jobs))
        .route("/api/training/jobs/:id/logs", get(monitor::job_logs))

        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(state);

    println!("🧬 Model Workshop running on http://127.0.0.1:4200");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:4200").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
