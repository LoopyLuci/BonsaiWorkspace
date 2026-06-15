//! Sulphur-2-base video generation — `Abiray/Sulphur-2-base-GGUF`.
//!
//! Runs via `video_worker.py` using the LTX pipeline.  Video generation is
//! compute-intensive so requests are serialised with a tokio Mutex to prevent
//! concurrent VRAM exhaustion.
//!
//! ## Model placement
//!   - `~/.bonsai/models/sulphur2/sulphur-2-base.gguf`
//!   - `$SULPHUR2_MODEL_PATH`
//!
//! ## Installation
//!   pip install diffusers transformers accelerate torch

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tracing::info;

use crate::tool_registry::{Tool, ToolResult};

// Serialise video generation — only one at a time.
static GEN_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
fn gen_lock() -> &'static Mutex<()> {
    GEN_LOCK.get_or_init(|| Mutex::new(()))
}

fn find_model() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("SULPHUR2_MODEL_PATH") {
        let pb = PathBuf::from(&p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let base = dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai/models/sulphur2");
    for name in &[
        "sulphur-2-base.gguf",
        "sulphur-2-base-Q4_K_M.gguf",
        "sulphur-2-base-Q5_K_M.gguf",
    ] {
        let p = base.join(name);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn find_worker() -> Option<PathBuf> {
    let candidates = [
        dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/sidecars/video_worker.py"),
        PathBuf::from("sidecars/video_worker.py"),
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
pub struct VideoGenResult {
    pub video_path: String,
    pub frames: u32,
    pub duration_s: f32,
    pub fps: u32,
    pub model: String,
    pub prompt: String,
    pub elapsed_ms: u64,
}

async fn run_generate(
    prompt: &str,
    frames: u32,
    fps: u32,
    save_path: &str,
) -> Result<VideoGenResult, String> {
    let model = find_model().ok_or(
        "Sulphur-2 model not found. Place sulphur-2-base.gguf in ~/.bonsai/models/sulphur2/",
    )?;
    let worker = find_worker().ok_or("video_worker.py not found in ~/.bonsai/sidecars/")?;
    let python = which_python()?;

    info!(%prompt, %frames, "[video_gen] generating video");
    let _guard = gen_lock().lock().await;
    let t0 = std::time::Instant::now();

    let payload = json!({
        "op": "generate_video",
        "prompt": prompt,
        "model_path": model.to_string_lossy().as_ref(),
        "frames": frames,
        "fps": fps,
        "save_path": save_path,
    });
    let encoded = serde_json::to_string(&payload).unwrap();

    let mut child = tokio::process::Command::new(&python)
        .arg(&worker)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to start video_worker: {e}"))?;

    if let Some(mut s) = child.stdin.take() {
        s.write_all(encoded.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
    }
    // Video generation can take a long time — allow up to 20 minutes.
    let out = tokio::time::timeout(Duration::from_secs(1200), child.wait_with_output())
        .await
        .map_err(|_| "video_worker timed out (20 min)".to_string())?
        .map_err(|e| e.to_string())?;

    if !out.status.success() {
        return Err(format!("video_worker exited {:?}", out.status.code()));
    }
    let result: Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("parse error: {e}"))?;

    Ok(VideoGenResult {
        video_path: result["video_path"]
            .as_str()
            .unwrap_or(save_path)
            .to_string(),
        frames: result["frames"].as_u64().unwrap_or(frames as u64) as u32,
        duration_s: result["duration_s"]
            .as_f64()
            .unwrap_or(frames as f64 / fps as f64) as f32,
        fps: result["fps"].as_u64().unwrap_or(fps as u64) as u32,
        model: model
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("sulphur2")
            .to_string(),
        prompt: prompt.to_string(),
        elapsed_ms: t0.elapsed().as_millis() as u64,
    })
}

// ── Tools ─────────────────────────────────────────────────────────────────────

pub struct GenerateVideoTool;
#[async_trait]
impl Tool for GenerateVideoTool {
    fn name(&self) -> &str {
        "generate_video"
    }
    fn description(&self) -> &str {
        "Generate a short video clip from a text prompt using Sulphur-2-base. \
         Args: {prompt: string, frames?: number (default 24), fps?: number (default 8), save_path: string}. \
         Returns: video_path, frames, duration_s, fps, elapsed_ms."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let prompt = args["prompt"].as_str().ok_or("Missing 'prompt'")?;
        let save_path = args["save_path"].as_str().ok_or("Missing 'save_path'")?;
        if !is_available() {
            return Err("Sulphur-2 model not installed. See Settings → Vision Tools.".into());
        }
        let frames = args["frames"].as_u64().unwrap_or(24) as u32;
        let fps = args["fps"].as_u64().unwrap_or(8) as u32;
        let result = run_generate(prompt, frames, fps, save_path).await?;
        Ok(ToolResult::json(
            &serde_json::to_value(&result).unwrap_or_default(),
        ))
    }
}

pub struct EnhanceVideoPromptTool;
#[async_trait]
impl Tool for EnhanceVideoPromptTool {
    fn name(&self) -> &str {
        "enhance_video_prompt"
    }
    fn description(&self) -> &str {
        "Enhance a video generation prompt with cinematic details (motion, lighting, camera). \
         Args: {prompt: string}. Returns: enhanced_prompt (string)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let prompt = args["prompt"].as_str().ok_or("Missing 'prompt'")?;
        let enhanced = format!(
            "{prompt}, cinematic quality, smooth motion, professional lighting, \
             high frame rate, sharp details, film grain, depth of field"
        );
        Ok(ToolResult::json(
            &json!({ "enhanced_prompt": enhanced, "original_prompt": prompt }),
        ))
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn generate_video_cmd(
    prompt: String,
    save_path: String,
    frames: Option<u32>,
    fps: Option<u32>,
) -> Result<VideoGenResult, String> {
    run_generate(&prompt, frames.unwrap_or(24), fps.unwrap_or(8), &save_path).await
}

#[tauri::command]
pub async fn video_gen_available() -> Result<bool, String> {
    Ok(is_available())
}
