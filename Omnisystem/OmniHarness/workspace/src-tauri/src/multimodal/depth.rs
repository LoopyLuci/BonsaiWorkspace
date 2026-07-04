//! Depth-Anything-V2 monocular depth estimation.
//!
//! The GGUF variant from `Acly/Depth-Anything-V2-GGUF` is loaded via the
//! existing `llama-server` sidecar with its vision pipeline.  The tool sends
//! an image to the running server's `/completion` endpoint and receives a
//! depth-annotated description.  For actual depth maps, the tool can
//! alternatively call a thin Python helper (`depth_worker.py`) that uses
//! `transformers` + PIL to produce a grayscale PNG.
//!
//! ## Model placement (offline)
//!   - `$DEPTH_MODEL_PATH`
//!   - `~/.bonsai/models/depth/Depth-Anything-V2-Small-F16.gguf`
//!   - `<app_data>/sidecars/depth/Depth-Anything-V2-Small-F16.gguf`
//!
//! ## Outputs
//! - `depth_map_b64` — grayscale PNG base64 (when Python worker available)
//! - `description`   — natural-language depth description from the VLM
//! - `near_objects` / `far_objects` — structured depth zones

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

use crate::tool_registry::{Tool, ToolResult};

// ── Model discovery ───────────────────────────────────────────────────────────

fn find_depth_model() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("DEPTH_MODEL_PATH") {
        let pb = PathBuf::from(&p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let candidates = [
        dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/models/depth/Depth-Anything-V2-Small-F16.gguf"),
        dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/models/depth/Depth-Anything-V2-Base-F16.gguf"),
        dirs::data_local_dir()
            .unwrap_or_default()
            .join("com.bonsai.workspace/sidecars/depth/Depth-Anything-V2-Small-F16.gguf"),
        PathBuf::from("sidecars/depth/Depth-Anything-V2-Small-F16.gguf"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

fn find_depth_worker() -> Option<PathBuf> {
    let candidates = [
        dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/sidecars/depth_worker.py"),
        PathBuf::from("sidecars/depth_worker.py"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

pub fn is_available() -> bool {
    find_depth_model().is_some()
}

// ── Depth result ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthResult {
    /// Grayscale depth map as base64 PNG (brighter = closer).
    /// `None` when only the VLM description path was used.
    pub depth_map_b64: Option<String>,
    /// Natural-language depth description from the vision model.
    pub description: String,
    /// Objects/regions estimated as nearest to camera.
    pub near_objects: Vec<String>,
    /// Objects/regions estimated as furthest from camera.
    pub far_objects: Vec<String>,
    /// Model variant used.
    pub model: String,
}

// ── Worker-based depth estimation ─────────────────────────────────────────────

#[derive(Serialize)]
struct DepthWorkerRequest<'a> {
    image_path: &'a str,
    output_png: bool,
}

async fn run_depth_worker(image_path: &str, model_path: &PathBuf) -> Result<DepthResult, String> {
    let worker =
        find_depth_worker().ok_or("depth_worker.py not found. Place it in ~/.bonsai/sidecars/")?;

    let python = which_python()?;
    let payload = serde_json::to_string(&DepthWorkerRequest {
        image_path,
        output_png: true,
    })
    .map_err(|e| e.to_string())?;

    let mut child = tokio::process::Command::new(&python)
        .arg(&worker)
        .arg("--model")
        .arg(model_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to start depth_worker: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(payload.as_bytes())
            .await
            .map_err(|e| format!("stdin write: {e}"))?;
    }

    let out = tokio::time::timeout(Duration::from_secs(120), child.wait_with_output())
        .await
        .map_err(|_| "depth_worker timed out".to_string())?
        .map_err(|e| format!("depth_worker error: {e}"))?;

    if !out.status.success() {
        return Err(format!("depth_worker exited {:?}", out.status.code()));
    }

    // Worker returns JSON: { depth_map_b64, near_objects, far_objects, description }
    let json: Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("depth_worker output parse error: {e}"))?;

    Ok(DepthResult {
        depth_map_b64: json["depth_map_b64"].as_str().map(|s| s.to_string()),
        description: json["description"]
            .as_str()
            .unwrap_or("Depth estimation complete")
            .to_string(),
        near_objects: json["near_objects"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        far_objects: json["far_objects"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        model: model_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("depth-anything-v2")
            .to_string(),
    })
}

/// Fallback: ask the running llama-server VLM to describe depth.
/// This doesn't produce a depth map PNG but still gives structural depth info.
async fn run_vlm_depth_description(
    image_path: &str,
    orchestrator_url: &str,
) -> Result<DepthResult, String> {
    let image_bytes = tokio::fs::read(image_path)
        .await
        .map_err(|e| format!("Cannot read image: {e}"))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&image_bytes);
    let ext = std::path::Path::new(image_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpeg");
    let mime = match ext {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    };

    let payload = serde_json::json!({
        "messages": [{
            "role": "user",
            "content": [
                { "type": "image_url", "image_url": { "url": format!("data:{mime};base64,{b64}") } },
                { "type": "text",      "text": "Analyse the depth in this image. List which objects appear nearest to the camera, which appear furthest, and describe the overall depth structure. Be concise and structured." }
            ]
        }],
        "max_tokens": 400
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{orchestrator_url}/v1/chat/completions"))
        .json(&payload)
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| format!("VLM depth request failed: {e}"))?;

    let body: Value = resp.json().await.map_err(|e| e.to_string())?;
    let description = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Ok(DepthResult {
        depth_map_b64: None,
        description,
        near_objects: vec![],
        far_objects: vec![],
        model: "vlm-fallback".into(),
    })
}

fn which_python() -> Result<PathBuf, String> {
    for c in &["python3", "python"] {
        if std::process::Command::new(c)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(PathBuf::from(c));
        }
    }
    Err("python3/python not found in PATH".into())
}

// ── Tool ──────────────────────────────────────────────────────────────────────

pub struct DepthEstimationTool {
    _marker: (),
}

impl DepthEstimationTool {
    pub fn new() -> Self {
        Self { _marker: () }
    }
}

#[async_trait]
impl Tool for DepthEstimationTool {
    fn name(&self) -> &str {
        "estimate_depth"
    }

    fn description(&self) -> &str {
        "Estimate monocular depth from an image using Depth-Anything-V2. \
         Args: {image_path: string}. \
         Returns: depth_map_b64 (grayscale PNG), description, near_objects[], far_objects[]."
    }

    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"]
            .as_str()
            .ok_or("Missing 'image_path' argument")?;

        if !std::path::Path::new(image_path).exists() {
            return Err(format!("Image not found: {image_path}"));
        }

        let model = find_depth_model().ok_or_else(|| {
            "Depth-Anything-V2 model not found. \
             Download from huggingface.co/Acly/Depth-Anything-V2-GGUF \
             and place in ~/.bonsai/models/depth/"
                .to_string()
        })?;

        info!(image = image_path, model = %model.display(), "[depth] estimating");

        let result = if find_depth_worker().is_some() {
            run_depth_worker(image_path, &model).await?
        } else {
            // Graceful fallback: use the running VLM
            warn!("[depth] depth_worker.py not found — falling back to VLM description");
            run_vlm_depth_description(image_path, "http://127.0.0.1:8080")
                .await
                .unwrap_or_else(|e| DepthResult {
                    depth_map_b64: None,
                    description: format!("Depth estimation unavailable: {e}"),
                    near_objects: vec![],
                    far_objects: vec![],
                    model: "unavailable".into(),
                })
        };

        Ok(ToolResult::json(
            &serde_json::to_value(&result).unwrap_or_default(),
        ))
    }
}

// ── Tauri command ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn estimate_depth(image_path: String) -> Result<DepthResult, String> {
    let model = find_depth_model().ok_or(
        "Depth-Anything-V2 model not found. Place Depth-Anything-V2-Small-F16.gguf in ~/.bonsai/models/depth/"
    )?;
    if find_depth_worker().is_some() {
        run_depth_worker(&image_path, &model).await
    } else {
        run_vlm_depth_description(&image_path, "http://127.0.0.1:8080").await
    }
}

#[tauri::command]
pub async fn depth_model_available() -> Result<bool, String> {
    Ok(is_available())
}
