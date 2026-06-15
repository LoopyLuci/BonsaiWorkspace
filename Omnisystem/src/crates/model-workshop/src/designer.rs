use axum::Json;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub base_model: String,
    pub architecture: String,
    pub quantization: String,
    pub context_window: u32,
    pub system_prompt: String,
    pub temperature: f32,
    pub kdb_modules: Vec<String>,
    pub tools: Vec<String>,
    pub parameters: ModelParameters,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelParameters {
    pub total_params_billion: f32,
    pub active_params_billion: f32,
    pub moe_experts: u32,
    pub active_experts: u32,
}

pub async fn create_config(Json(config): Json<ModelConfig>) -> Json<serde_json::Value> {
    let validation = validate_model_config(&config);
    Json(serde_json::json!({
        "status": if validation.is_empty() { "valid" } else { "invalid" },
        "config": config,
        "validation_errors": validation
    }))
}

pub async fn validate_config(Json(config): Json<ModelConfig>) -> Json<serde_json::Value> {
    let errors = validate_model_config(&config);
    let estimated_memory = estimate_memory(&config);

    Json(serde_json::json!({
        "valid": errors.is_empty(),
        "errors": errors,
        "warnings": Vec::<String>::new(),
        "estimated_memory_gb": estimated_memory
    }))
}

fn validate_model_config(config: &ModelConfig) -> Vec<String> {
    let mut errors = Vec::new();
    if config.name.is_empty() {
        errors.push("Name is required".into());
    }
    if config.context_window < 512 {
        errors.push("Context window must be at least 512".into());
    }
    if config.temperature < 0.0 || config.temperature > 2.0 {
        errors.push("Temperature must be 0.0–2.0".into());
    }
    if config.parameters.total_params_billion <= 0.0 {
        errors.push("Total parameters must be positive".into());
    }
    errors
}

fn estimate_memory(config: &ModelConfig) -> f32 {
    let base_gb = match config.quantization.as_str() {
        "q4_k_m" => config.parameters.total_params_billion * 0.5,
        "q8_0" => config.parameters.total_params_billion * 1.0,
        "f16" => config.parameters.total_params_billion * 2.0,
        "f32" => config.parameters.total_params_billion * 4.0,
        _ => config.parameters.total_params_billion * 2.0,
    };
    let kv_cache_gb = (config.context_window as f32 * config.parameters.active_params_billion * 0.002) / 1024.0;
    base_gb + kv_cache_gb
}
