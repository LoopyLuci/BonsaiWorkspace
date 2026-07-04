//! PixAI image tagger — `deepghs/pixai-tagger-v0.9-onnx`.
//!
//! Multi-label classification returning 13 000+ Danbooru-style tags with
//! confidence scores.  Runs via a Python sidecar using `dghs-imgutils` or
//! `onnxruntime` directly.
//!
//! ## Model placement
//!   - `~/.bonsai/models/pixai/pixai-tagger-v0.9.onnx`
//!   - `$PIXAI_MODEL_PATH`
//!
//! ## Installation
//!   pip install dghs-imgutils onnxruntime

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;
use tracing::info;

use crate::tool_registry::{Tool, ToolResult};

fn find_model() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("PIXAI_MODEL_PATH") {
        let pb = PathBuf::from(&p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let candidates = [
        dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/models/pixai/pixai-tagger-v0.9.onnx"),
        PathBuf::from("sidecars/pixai/pixai-tagger-v0.9.onnx"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

fn find_worker() -> Option<PathBuf> {
    let candidates = [
        dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/sidecars/pixai_worker.py"),
        PathBuf::from("sidecars/pixai_worker.py"),
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
pub struct TagResult {
    pub tags: Vec<TagEntry>,
    pub top_tags: Vec<String>,
    pub dominant_category: String,
    pub tag_count: usize,
    pub model: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagEntry {
    pub tag: String,
    pub confidence: f32,
    pub category: String,
}

async fn run_tagger(
    image_path: &str,
    confidence: f32,
    max_tags: usize,
) -> Result<TagResult, String> {
    let model = find_model()
        .ok_or("PixAI model not found. Place pixai-tagger-v0.9.onnx in ~/.bonsai/models/pixai/")?;
    let worker = find_worker().ok_or("pixai_worker.py not found in ~/.bonsai/sidecars/")?;
    let python = which_python()?;

    let payload = json!({
        "image_path":  image_path,
        "model_path":  model.to_string_lossy().as_ref(),
        "confidence":  confidence,
        "max_tags":    max_tags,
    });
    let encoded = serde_json::to_string(&payload).unwrap();

    let t0 = std::time::Instant::now();
    let mut child = tokio::process::Command::new(&python)
        .arg(&worker)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to start pixai_worker: {e}"))?;

    if let Some(mut s) = child.stdin.take() {
        s.write_all(encoded.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
    }
    let out = tokio::time::timeout(Duration::from_secs(60), child.wait_with_output())
        .await
        .map_err(|_| "pixai_worker timed out".to_string())?
        .map_err(|e| e.to_string())?;

    if !out.status.success() {
        return Err(format!("pixai_worker exited {:?}", out.status.code()));
    }
    let result: Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("parse error: {e}"))?;

    let tags: Vec<TagEntry> = serde_json::from_value(result["tags"].clone()).unwrap_or_default();
    let top_tags: Vec<String> = tags.iter().take(10).map(|t| t.tag.clone()).collect();
    let dominant = tags
        .first()
        .map(|t| t.category.clone())
        .unwrap_or_else(|| "general".into());
    let count = tags.len();

    Ok(TagResult {
        tags,
        top_tags,
        dominant_category: dominant,
        tag_count: count,
        model: model
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("pixai")
            .to_string(),
        elapsed_ms: t0.elapsed().as_millis() as u64,
    })
}

// ── Tools ─────────────────────────────────────────────────────────────────────

pub struct TagImageTool;
#[async_trait]
impl Tool for TagImageTool {
    fn name(&self) -> &str {
        "tag_image"
    }
    fn description(&self) -> &str {
        "Tag an image with Danbooru-style labels using PixAI tagger (13 000+ tags). \
         Args: {image_path: string, confidence?: number (default 0.35), max_tags?: number (default 50)}. \
         Returns: tags[] (tag, confidence, category), top_tags[], dominant_category."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        let confidence = args["confidence"].as_f64().unwrap_or(0.35) as f32;
        let max_tags = args["max_tags"].as_u64().unwrap_or(50) as usize;
        if !std::path::Path::new(path).exists() {
            return Err(format!("Image not found: {path}"));
        }
        if !is_available() {
            return Err("PixAI model not installed. See Settings → Vision Tools.".into());
        }
        info!(image = path, "[pixai] tagging");
        let result = run_tagger(path, confidence, max_tags).await?;
        Ok(ToolResult::json(
            &serde_json::to_value(&result).unwrap_or_default(),
        ))
    }
}

pub struct DescribeImageAnimeTool;
#[async_trait]
impl Tool for DescribeImageAnimeTool {
    fn name(&self) -> &str {
        "describe_image_anime"
    }
    fn description(&self) -> &str {
        "Describe an anime/illustration image using PixAI tags → natural language. \
         Args: {image_path: string}. Returns: description (string), tags[]."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        if !is_available() {
            return Err("PixAI model not installed. See Settings → Vision Tools.".into());
        }
        let result = run_tagger(path, 0.40, 30).await?;
        let description = format!(
            "This image appears to be {} style content. Key elements: {}.",
            result.dominant_category,
            result.top_tags.join(", ")
        );
        Ok(ToolResult::json(
            &json!({ "description": description, "tags": result.tags }),
        ))
    }
}

// ── Public API (used by vision_training.rs) ───────────────────────────────────
pub async fn tag_image_internal(image_path: &str) -> Option<TagResult> {
    if !is_available() {
        return None;
    }
    run_tagger(image_path, 0.35, 50).await.ok()
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn pixai_tag(
    image_path: String,
    confidence: Option<f32>,
    max_tags: Option<usize>,
) -> Result<TagResult, String> {
    if !std::path::Path::new(&image_path).exists() {
        return Err(format!("Image not found: {image_path}"));
    }
    run_tagger(
        &image_path,
        confidence.unwrap_or(0.35),
        max_tags.unwrap_or(50),
    )
    .await
}

#[tauri::command]
pub async fn pixai_available() -> Result<bool, String> {
    Ok(is_available())
}
