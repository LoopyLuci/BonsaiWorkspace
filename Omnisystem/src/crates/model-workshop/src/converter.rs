use axum::Json;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ConvertRequest {
    pub input_path: String,
    pub input_format: String,
    pub output_format: String,
    pub quantization: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct QuantizeRequest {
    pub input_path: String,
    pub quantization: String,
    pub output_path: Option<String>,
}

pub async fn convert_model(Json(req): Json<ConvertRequest>) -> Json<serde_json::Value> {
    let output_path = format!(
        "{}.{}",
        req.input_path.trim_end_matches(&format!(".{}", req.input_format)),
        req.output_format
    );

    Json(serde_json::json!({
        "status": "converting",
        "input": req.input_path,
        "input_format": req.input_format,
        "output_format": req.output_format,
        "output_path": output_path,
        "estimated_time": "2-5 minutes",
        "command": format!(
            "python convert.py --input {} --from {} --to {} {}",
            req.input_path,
            req.input_format,
            req.output_format,
            req.quantization.map(|q| format!("--quantize {}", q)).unwrap_or_default()
        ),
        "message": "Model conversion started"
    }))
}

pub async fn quantize_model(Json(req): Json<QuantizeRequest>) -> Json<serde_json::Value> {
    let size_reduction = match req.quantization.as_str() {
        "q4_k_m" => "75%",
        "q5_k_m" => "67%",
        "q8_0" => "50%",
        _ => "unknown",
    };

    Json(serde_json::json!({
        "status": "quantizing",
        "input": req.input_path,
        "quantization": req.quantization,
        "estimated_size_reduction": size_reduction,
        "estimated_time": "1-2 minutes",
        "command": format!(
            "python quantize.py --input {} --quantization {} {}",
            req.input_path,
            req.quantization,
            req.output_path.map(|o| format!("--output {}", o)).unwrap_or_default()
        ),
        "message": "Model quantization started"
    }))
}
