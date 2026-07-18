use axum::{Json, extract::Path};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct EditModelRequest {
    pub name: Option<String>,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub context_window: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct MergeLoraRequest {
    pub base_model_id: String,
    pub lora_model_id: String,
    pub output_model_id: String,
    pub scaling_factor: Option<f32>,
}

pub async fn edit_model(
    Path(id): Path<String>,
    Json(req): Json<EditModelRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "updated",
        "model_id": id,
        "changes": req,
        "message": "Model configuration updated"
    }))
}

pub async fn merge_lora(
    Json(req): Json<MergeLoraRequest>,
) -> Json<serde_json::Value> {
    let scaling = req.scaling_factor.unwrap_or(1.0);

    Json(serde_json::json!({
        "status": "merging",
        "base_model": req.base_model_id,
        "lora_model": req.lora_model_id,
        "output_model": req.output_model_id,
        "scaling_factor": scaling,
        "estimated_time": "2-5 minutes",
        "message": "LoRA merge started"
    }))
}
