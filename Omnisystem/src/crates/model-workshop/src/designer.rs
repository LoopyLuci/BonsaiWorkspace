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

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config() -> ModelConfig {
        ModelConfig {
            name: "demo".into(),
            base_model: "base-7b".into(),
            architecture: "transformer".into(),
            quantization: "q4_k_m".into(),
            context_window: 4096,
            system_prompt: "You are helpful.".into(),
            temperature: 0.7,
            kdb_modules: vec![],
            tools: vec![],
            parameters: ModelParameters {
                total_params_billion: 7.0,
                active_params_billion: 7.0,
                moe_experts: 1,
                active_experts: 1,
            },
        }
    }

    #[test]
    fn valid_config_has_no_errors() {
        assert!(validate_model_config(&valid_config()).is_empty());
    }

    #[test]
    fn empty_name_is_rejected() {
        let mut config = valid_config();
        config.name = "".into();
        let errors = validate_model_config(&config);
        assert!(errors.iter().any(|e| e.contains("Name")));
    }

    #[test]
    fn small_context_window_is_rejected() {
        let mut config = valid_config();
        config.context_window = 128;
        let errors = validate_model_config(&config);
        assert!(errors.iter().any(|e| e.contains("Context window")));
    }

    #[test]
    fn out_of_range_temperature_is_rejected() {
        let mut config = valid_config();
        config.temperature = 3.5;
        let errors = validate_model_config(&config);
        assert!(errors.iter().any(|e| e.contains("Temperature")));
    }

    #[test]
    fn non_positive_params_is_rejected() {
        let mut config = valid_config();
        config.parameters.total_params_billion = 0.0;
        let errors = validate_model_config(&config);
        assert!(errors.iter().any(|e| e.contains("Total parameters")));
    }

    #[test]
    fn q4_quantization_is_roughly_half_size() {
        let config = valid_config();
        let mem = estimate_memory(&config);
        // 7B params at q4_k_m -> ~3.5GB base + small kv cache overhead.
        assert!(mem > 3.5 && mem < 4.0, "unexpected estimate: {mem}");
    }

    #[test]
    fn f32_quantization_is_larger_than_q4() {
        let mut q4 = valid_config();
        q4.quantization = "q4_k_m".into();
        let mut f32_cfg = valid_config();
        f32_cfg.quantization = "f32".into();

        assert!(estimate_memory(&f32_cfg) > estimate_memory(&q4));
    }
}
