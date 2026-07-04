//! Qwen-Image-Edit rapid image editing + multi-angle LoRA.
//!
//! Two models are supported:
//!   - `Phr00t/Qwen-Image-Edit-Rapid-AIO`        — all-in-one rapid editing
//!   - `fal/Qwen-Image-Edit-2511-Multiple-Angles-LoRA` — camera angle variants
//!
//! Both run via `image_edit_worker.py` using the `diffusers` pipeline.
//!
//! ## Model placement
//!   - `~/.bonsai/models/image_edit/qwen-rapid/`
//!   - `~/.bonsai/models/image_edit/qwen-multiangle/`
//!   - `$IMAGE_EDIT_MODEL_PATH` (rapid) / `$IMAGE_EDIT_MULTIANGLE_PATH` (multi-angle)
//!
//! ## Installation
//!   pip install diffusers transformers accelerate torch pillow

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;
use tracing::info;

use crate::tool_registry::{Tool, ToolResult};

fn find_rapid_model() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("IMAGE_EDIT_MODEL_PATH") {
        let pb = PathBuf::from(&p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let base = dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai/models/image_edit/qwen-rapid");
    if base.exists() {
        return Some(base);
    }
    None
}

fn find_multiangle_model() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("IMAGE_EDIT_MULTIANGLE_PATH") {
        let pb = PathBuf::from(&p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let base = dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai/models/image_edit/qwen-multiangle");
    if base.exists() {
        return Some(base);
    }
    None
}

fn find_worker() -> Option<PathBuf> {
    let candidates = [
        dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/sidecars/image_edit_worker.py"),
        PathBuf::from("sidecars/image_edit_worker.py"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

pub fn is_available() -> bool {
    find_rapid_model().is_some() && find_worker().is_some()
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
    Err("python3/python not found".into())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageEditResult {
    pub image_b64: String,
    pub saved_path: Option<String>,
    pub model: String,
    pub prompt: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiviewResult {
    pub images_b64: Vec<String>,
    pub angles: Vec<String>,
    pub saved_paths: Vec<Option<String>>,
    pub model: String,
    pub elapsed_ms: u64,
}

async fn call_worker(payload: &Value) -> Result<Value, String> {
    let worker = find_worker().ok_or("image_edit_worker.py not found in ~/.bonsai/sidecars/")?;
    let python = which_python()?;
    let encoded = serde_json::to_string(payload).unwrap();

    let mut child = tokio::process::Command::new(&python)
        .arg(&worker)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to start image_edit_worker: {e}"))?;

    if let Some(mut s) = child.stdin.take() {
        s.write_all(encoded.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
    }
    let out = tokio::time::timeout(Duration::from_secs(300), child.wait_with_output())
        .await
        .map_err(|_| "image_edit_worker timed out (300s)".to_string())?
        .map_err(|e| e.to_string())?;

    if !out.status.success() {
        return Err(format!("image_edit_worker exited {:?}", out.status.code()));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("parse error: {e}"))
}

async fn run_edit(
    image_path: &str,
    prompt: &str,
    save_path: Option<&str>,
) -> Result<ImageEditResult, String> {
    let model = find_rapid_model().ok_or(
        "Qwen-Image-Edit model not found. Place model in ~/.bonsai/models/image_edit/qwen-rapid/",
    )?;
    info!(image = image_path, %prompt, "[image_edit] editing image");
    let t0 = std::time::Instant::now();
    let payload = json!({
        "op": "edit_image",
        "image_path": image_path,
        "prompt": prompt,
        "model_path": model.to_string_lossy().as_ref(),
        "save_path": save_path,
    });
    let result = call_worker(&payload).await?;
    Ok(ImageEditResult {
        image_b64: result["image_b64"].as_str().unwrap_or("").to_string(),
        saved_path: result["saved_path"].as_str().map(|s| s.to_string()),
        model: model
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("qwen-rapid")
            .to_string(),
        prompt: prompt.to_string(),
        elapsed_ms: t0.elapsed().as_millis() as u64,
    })
}

async fn run_generate_rapid(
    prompt: &str,
    save_path: Option<&str>,
) -> Result<ImageEditResult, String> {
    let model = find_rapid_model().ok_or("Qwen-Image-Edit model not found.")?;
    info!(%prompt, "[image_edit] generating image (rapid)");
    let t0 = std::time::Instant::now();
    let payload = json!({
        "op": "generate_image",
        "prompt": prompt,
        "model_path": model.to_string_lossy().as_ref(),
        "save_path": save_path,
    });
    let result = call_worker(&payload).await?;
    Ok(ImageEditResult {
        image_b64: result["image_b64"].as_str().unwrap_or("").to_string(),
        saved_path: result["saved_path"].as_str().map(|s| s.to_string()),
        model: model
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("qwen-rapid")
            .to_string(),
        prompt: prompt.to_string(),
        elapsed_ms: t0.elapsed().as_millis() as u64,
    })
}

async fn run_multiview(image_path: &str, angles: &[String]) -> Result<MultiviewResult, String> {
    let model = find_multiangle_model()
        .or_else(find_rapid_model)
        .ok_or("Qwen multi-angle model not found. Place model in ~/.bonsai/models/image_edit/qwen-multiangle/")?;
    info!(image = image_path, "[image_edit] generating multiview");
    let t0 = std::time::Instant::now();
    let default_angles = [
        "front",
        "side_left",
        "side_right",
        "back",
        "top",
        "isometric",
    ];
    let angle_list: Vec<&str> = if angles.is_empty() {
        default_angles.iter().map(|s| *s).collect()
    } else {
        angles.iter().map(|s| s.as_str()).collect()
    };
    let payload = json!({
        "op": "generate_multiview",
        "image_path": image_path,
        "angles": angle_list,
        "model_path": model.to_string_lossy().as_ref(),
    });
    let result = call_worker(&payload).await?;
    let images_b64: Vec<String> = result["images_b64"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let saved_paths: Vec<Option<String>> = result["saved_paths"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    Ok(MultiviewResult {
        images_b64,
        angles: angle_list.iter().map(|s| s.to_string()).collect(),
        saved_paths,
        model: model
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("qwen-multiangle")
            .to_string(),
        elapsed_ms: t0.elapsed().as_millis() as u64,
    })
}

// ── Tools ─────────────────────────────────────────────────────────────────────

pub struct EditImageTool;
#[async_trait]
impl Tool for EditImageTool {
    fn name(&self) -> &str {
        "edit_image"
    }
    fn description(&self) -> &str {
        "Edit an image using a text prompt with Qwen-Image-Edit-Rapid-AIO. \
         Args: {image_path: string, prompt: string, save_path?: string}. \
         Returns: image_b64, saved_path, model, elapsed_ms."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        let prompt = args["prompt"].as_str().ok_or("Missing 'prompt'")?;
        if !std::path::Path::new(image_path).exists() {
            return Err(format!("Image not found: {image_path}"));
        }
        if !is_available() {
            return Err("Qwen-Image-Edit model not installed. See Settings → Vision Tools.".into());
        }
        let save_path = args["save_path"].as_str();
        let result = run_edit(image_path, prompt, save_path).await?;
        Ok(ToolResult::json(
            &serde_json::to_value(&result).unwrap_or_default(),
        ))
    }
}

pub struct GenerateImageRapidTool;
#[async_trait]
impl Tool for GenerateImageRapidTool {
    fn name(&self) -> &str {
        "generate_image_rapid"
    }
    fn description(&self) -> &str {
        "Generate an image from a text prompt using Qwen-Image-Edit-Rapid-AIO. \
         Args: {prompt: string, save_path?: string}. \
         Returns: image_b64, saved_path."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let prompt = args["prompt"].as_str().ok_or("Missing 'prompt'")?;
        if !is_available() {
            return Err("Qwen-Image-Edit model not installed.".into());
        }
        let save_path = args["save_path"].as_str();
        let result = run_generate_rapid(prompt, save_path).await?;
        Ok(ToolResult::json(
            &serde_json::to_value(&result).unwrap_or_default(),
        ))
    }
}

pub struct GenerateMultiviewTool;
#[async_trait]
impl Tool for GenerateMultiviewTool {
    fn name(&self) -> &str {
        "generate_multiview"
    }
    fn description(&self) -> &str {
        "Generate multiple camera-angle views of an object image using Qwen multi-angle LoRA. \
         Args: {image_path: string, angles?: string[] (default: front/side/back/top/isometric)}. \
         Returns: images_b64[], angles[], saved_paths[]."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        if !std::path::Path::new(image_path).exists() {
            return Err(format!("Image not found: {image_path}"));
        }
        let angles: Vec<String> = args["angles"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let result = run_multiview(image_path, &angles).await?;
        Ok(ToolResult::json(
            &serde_json::to_value(&result).unwrap_or_default(),
        ))
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn edit_image_cmd(
    image_path: String,
    prompt: String,
    save_path: Option<String>,
) -> Result<ImageEditResult, String> {
    if !std::path::Path::new(&image_path).exists() {
        return Err(format!("Image not found: {image_path}"));
    }
    run_edit(&image_path, &prompt, save_path.as_deref()).await
}

#[tauri::command]
pub async fn generate_image_rapid_cmd(
    prompt: String,
    save_path: Option<String>,
) -> Result<ImageEditResult, String> {
    run_generate_rapid(&prompt, save_path.as_deref()).await
}

#[tauri::command]
pub async fn generate_multiview_cmd(
    image_path: String,
    angles: Option<Vec<String>>,
) -> Result<MultiviewResult, String> {
    if !std::path::Path::new(&image_path).exists() {
        return Err(format!("Image not found: {image_path}"));
    }
    run_multiview(&image_path, &angles.unwrap_or_default()).await
}

#[tauri::command]
pub async fn image_edit_available() -> Result<bool, String> {
    Ok(is_available())
}
