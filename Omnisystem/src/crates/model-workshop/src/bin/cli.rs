//! Model Workshop CLI: exercises the real in-process handlers (module
//! library, dataset registry, model designer validation, and training job
//! queue) without needing to stand up the HTTP server.

use axum::extract::State;
use axum::Json;
use model_workshop::designer::{self, ModelConfig, ModelParameters};
use model_workshop::{builder, library, monitor, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState::new();

    let create = library::create_module(
        State(state.clone()),
        Json(library::CreateModuleRequest {
            name: "Demo Module".into(),
            description: "A module created by the CLI smoke test".into(),
            domains: vec!["general".into()],
            chunks: vec![library::ChunkInput {
                text: "hello world".into(),
                domain: Some("general".into()),
                tags: vec![],
            }],
        }),
    )
    .await;
    println!("created module: {}", create.0);

    let config = ModelConfig {
        name: "demo-model".into(),
        base_model: "base-7b".into(),
        architecture: "transformer".into(),
        quantization: "q4_k_m".into(),
        context_window: 4096,
        system_prompt: "You are a helpful assistant.".into(),
        temperature: 0.7,
        kdb_modules: vec![],
        tools: vec![],
        parameters: ModelParameters {
            total_params_billion: 7.0,
            active_params_billion: 7.0,
            moe_experts: 1,
            active_experts: 1,
        },
    };
    let validation = designer::validate_config(Json(config)).await;
    println!("validation: {}", validation.0);

    let start = builder::start_training(
        State(state.clone()),
        Json(builder::TrainingRequest {
            config_path: "configs/demo.toml".into(),
            stages: vec![1, 2, 3],
            gpu_count: Some(1),
            dataset_id: None,
        }),
    )
    .await;
    println!("training job: {}", start.0);

    let jobs = monitor::list_jobs(State(state.clone())).await;
    println!("jobs: {}", jobs.0);

    Ok(())
}
