//! TRELLIS.2-4B 3D asset generation — `microsoft/TRELLIS.2-4B`.
//!
//! Converts an image or text prompt to a 3D mesh (GLB/OBJ) via `threed_worker.py`.
//!
//! ## Model placement
//!   - `~/.bonsai/models/trellis/`   (weights directory)
//!   - `$TRELLIS_MODEL_PATH`
//!
//! ## Installation
//!   pip install torch torchvision transformers accelerate trimesh

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;
use tracing::info;

use crate::tool_registry::{Tool, ToolResult};

fn find_model() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("TRELLIS_MODEL_PATH") {
        let pb = PathBuf::from(&p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let base = dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai/models/trellis");
    if base.exists() {
        return Some(base);
    }
    None
}

fn find_worker() -> Option<PathBuf> {
    let candidates = [
        dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/sidecars/threed_worker.py"),
        PathBuf::from("sidecars/threed_worker.py"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

pub fn is_available() -> bool {
    find_model().is_some() && find_worker().is_some()
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
pub struct ThreeDResult {
    pub mesh_path: String,
    pub format: String,
    pub vertex_count: u64,
    pub face_count: u64,
    pub model: String,
    pub source: String,
    pub elapsed_ms: u64,
}

async fn call_worker(payload: &Value) -> Result<Value, String> {
    let worker = find_worker().ok_or("threed_worker.py not found in ~/.bonsai/sidecars/")?;
    let python = which_python()?;
    let encoded = serde_json::to_string(payload).unwrap();

    let mut child = tokio::process::Command::new(&python)
        .arg(&worker)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to start threed_worker: {e}"))?;

    if let Some(mut s) = child.stdin.take() {
        s.write_all(encoded.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
    }
    // 3D generation is slow — allow up to 10 minutes
    let out = tokio::time::timeout(Duration::from_secs(600), child.wait_with_output())
        .await
        .map_err(|_| "threed_worker timed out (10 min)".to_string())?
        .map_err(|e| e.to_string())?;

    if !out.status.success() {
        return Err(format!("threed_worker exited {:?}", out.status.code()));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("parse error: {e}"))
}

async fn run_from_image(
    image_path: &str,
    save_path: &str,
    format: &str,
) -> Result<ThreeDResult, String> {
    let model =
        find_model().ok_or("TRELLIS model not found. Place model in ~/.bonsai/models/trellis/")?;
    info!(image = image_path, "[threed_gen] generating 3D from image");
    let t0 = std::time::Instant::now();
    let payload = json!({
        "op": "image_to_3d",
        "image_path": image_path,
        "model_path": model.to_string_lossy().as_ref(),
        "save_path": save_path,
        "format": format,
    });
    let result = call_worker(&payload).await?;
    Ok(ThreeDResult {
        mesh_path: result["mesh_path"]
            .as_str()
            .unwrap_or(save_path)
            .to_string(),
        format: result["format"].as_str().unwrap_or(format).to_string(),
        vertex_count: result["vertex_count"].as_u64().unwrap_or(0),
        face_count: result["face_count"].as_u64().unwrap_or(0),
        model: "TRELLIS.2-4B".to_string(),
        source: format!("image:{image_path}"),
        elapsed_ms: t0.elapsed().as_millis() as u64,
    })
}

async fn run_from_text(
    prompt: &str,
    save_path: &str,
    format: &str,
) -> Result<ThreeDResult, String> {
    let model =
        find_model().ok_or("TRELLIS model not found. Place model in ~/.bonsai/models/trellis/")?;
    info!(%prompt, "[threed_gen] generating 3D from text");
    let t0 = std::time::Instant::now();
    let payload = json!({
        "op": "text_to_3d",
        "prompt": prompt,
        "model_path": model.to_string_lossy().as_ref(),
        "save_path": save_path,
        "format": format,
    });
    let result = call_worker(&payload).await?;
    Ok(ThreeDResult {
        mesh_path: result["mesh_path"]
            .as_str()
            .unwrap_or(save_path)
            .to_string(),
        format: result["format"].as_str().unwrap_or(format).to_string(),
        vertex_count: result["vertex_count"].as_u64().unwrap_or(0),
        face_count: result["face_count"].as_u64().unwrap_or(0),
        model: "TRELLIS.2-4B".to_string(),
        source: format!("text:{prompt}"),
        elapsed_ms: t0.elapsed().as_millis() as u64,
    })
}

// ── Tools ─────────────────────────────────────────────────────────────────────

pub struct Generate3dModelTool;
#[async_trait]
impl Tool for Generate3dModelTool {
    fn name(&self) -> &str {
        "generate_3d_model"
    }
    fn description(&self) -> &str {
        "Generate a 3D mesh from an image using TRELLIS.2-4B. \
         Args: {image_path: string, save_path: string, format?: 'glb'|'obj' (default 'glb')}. \
         Returns: mesh_path, format, vertex_count, face_count, elapsed_ms."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        let save_path = args["save_path"].as_str().ok_or("Missing 'save_path'")?;
        if !std::path::Path::new(image_path).exists() {
            return Err(format!("Image not found: {image_path}"));
        }
        if !is_available() {
            return Err("TRELLIS model not installed. See Settings → Vision Tools.".into());
        }
        let format = args["format"].as_str().unwrap_or("glb");
        let result = run_from_image(image_path, save_path, format).await?;
        Ok(ToolResult::json(
            &serde_json::to_value(&result).unwrap_or_default(),
        ))
    }
}

pub struct Generate3dFromTextTool;
#[async_trait]
impl Tool for Generate3dFromTextTool {
    fn name(&self) -> &str {
        "generate_3d_from_text"
    }
    fn description(&self) -> &str {
        "Generate a 3D mesh from a text description using TRELLIS.2-4B. \
         Args: {prompt: string, save_path: string, format?: 'glb'|'obj' (default 'glb')}. \
         Returns: mesh_path, format, vertex_count, face_count."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let prompt = args["prompt"].as_str().ok_or("Missing 'prompt'")?;
        let save_path = args["save_path"].as_str().ok_or("Missing 'save_path'")?;
        if !is_available() {
            return Err("TRELLIS model not installed. See Settings → Vision Tools.".into());
        }
        let format = args["format"].as_str().unwrap_or("glb");
        let result = run_from_text(prompt, save_path, format).await?;
        Ok(ToolResult::json(
            &serde_json::to_value(&result).unwrap_or_default(),
        ))
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn generate_3d_model_cmd(
    image_path: String,
    save_path: String,
    format: Option<String>,
) -> Result<ThreeDResult, String> {
    if !std::path::Path::new(&image_path).exists() {
        return Err(format!("Image not found: {image_path}"));
    }
    run_from_image(&image_path, &save_path, format.as_deref().unwrap_or("glb")).await
}

#[tauri::command]
pub async fn generate_3d_from_text_cmd(
    prompt: String,
    save_path: String,
    format: Option<String>,
) -> Result<ThreeDResult, String> {
    run_from_text(&prompt, &save_path, format.as_deref().unwrap_or("glb")).await
}

#[tauri::command]
pub async fn threed_gen_available() -> Result<bool, String> {
    Ok(is_available())
}
