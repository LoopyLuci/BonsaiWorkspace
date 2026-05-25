//! Offline image generation via a local Stable Diffusion Python script.
//! Serialised by a semaphore so only one generation runs at a time (prevents GPU OOM).

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use tracing::{info, warn};

static GPU_SLOT: OnceLock<Semaphore> = OnceLock::new();

fn gpu_slot() -> &'static Semaphore {
    GPU_SLOT.get_or_init(|| Semaphore::new(1))
}

#[derive(Debug, Deserialize)]
pub struct ImageGenRequest {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub steps: Option<u32>,
    pub guidance_scale: Option<f32>,
    /// Path to a local Stable Diffusion model directory (safetensors/GGUF).
    pub model_path: Option<String>,
    /// Where to save the output PNG. Defaults to a temp file.
    pub output_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ImageGenResult {
    pub output_path: String,
    pub width: u32,
    pub height: u32,
    pub elapsed_ms: u64,
}

#[tauri::command]
pub async fn generate_image_command(request: ImageGenRequest) -> Result<ImageGenResult, String> {
    generate_image(request).await
}

pub async fn generate_image(request: ImageGenRequest) -> Result<ImageGenResult, String> {
    let _permit = gpu_slot()
        .acquire()
        .await
        .map_err(|_| "GPU semaphore closed".to_string())?;

    let script = find_sd_script()?;
    let model = request
        .model_path
        .clone()
        .unwrap_or_else(|| find_default_sd_model().unwrap_or_default());

    if model.is_empty() {
        return Err("No Stable Diffusion model found. Set model_path or place a model in ~/.bonsai/models/sd/".into());
    }

    let output_path = request.output_path.clone().unwrap_or_else(|| {
        std::env::temp_dir()
            .join(format!("bonsai_img_{}.png", epoch_ms()))
            .to_string_lossy()
            .into_owned()
    });

    let width = request.width.unwrap_or(512);
    let height = request.height.unwrap_or(512);
    let steps = request.steps.unwrap_or(20);
    let guidance = request.guidance_scale.unwrap_or(7.5);

    let mut args = vec![
        script.to_string_lossy().into_owned(),
        "--model".into(), model,
        "--prompt".into(), request.prompt.clone(),
        "--output".into(), output_path.clone(),
        "--width".into(), width.to_string(),
        "--height".into(), height.to_string(),
        "--steps".into(), steps.to_string(),
        "--guidance".into(), guidance.to_string(),
    ];
    if let Some(neg) = &request.negative_prompt {
        args.push("--negative_prompt".into());
        args.push(neg.clone());
    }

    let python = find_sd_python();
    info!(prompt = %request.prompt, output = %output_path, python = %python, "[image_gen] starting generation");
    let t0 = std::time::Instant::now();

    let out = tokio::time::timeout(
        Duration::from_secs(300),
        tokio::process::Command::new(&python)
            .args(&args)
            .output(),
    )
    .await
    .map_err(|_| "Image generation timed out (300s)".to_string())?
    .map_err(|e| format!("Spawn failed: {e}"))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        warn!(error = %err, "[image_gen] generation failed");
        return Err(format!("SD script failed: {err}"));
    }

    Ok(ImageGenResult {
        output_path,
        width,
        height,
        elapsed_ms: t0.elapsed().as_millis() as u64,
    })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn find_sd_script() -> Result<PathBuf, String> {
    let candidates = [
        dirs::data_local_dir()
            .unwrap_or_default()
            .join("com.bonsai.workspace")
            .join("scripts")
            .join("sd_generate.py"),
        PathBuf::from("scripts").join("sd_generate.py"),
        PathBuf::from("sd_generate.py"),
    ];
    for p in &candidates {
        if p.exists() {
            return Ok(p.clone());
        }
    }
    Err("sd_generate.py not found. Place it in AppData/com.bonsai.workspace/scripts/ or set BONSAI_SD_SCRIPT.".into())
}

fn find_sd_python() -> String {
    // Prefer the dedicated SD venv installed by install-sd.ps1
    let venv_py = dirs::data_local_dir()
        .unwrap_or_default()
        .join("com.bonsai.workspace")
        .join("sd_venv")
        .join(if cfg!(windows) { "Scripts" } else { "bin" })
        .join(if cfg!(windows) { "python.exe" } else { "python" });
    if venv_py.exists() {
        return venv_py.to_string_lossy().into_owned();
    }
    // Fall back to system Python candidates
    for candidate in &["py", "python3", "python"] {
        if std::process::Command::new(candidate)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return candidate.to_string();
        }
    }
    "python".to_string()
}

fn find_default_sd_model() -> Option<String> {
    let base = dirs::home_dir()?.join(".bonsai").join("models").join("sd");
    for ext in &["safetensors", "ckpt", "gguf"] {
        if let Ok(mut rd) = std::fs::read_dir(&base) {
            while let Some(Ok(entry)) = rd.next() {
                if entry.path().extension().map(|e| e == *ext).unwrap_or(false) {
                    return Some(entry.path().to_string_lossy().into_owned());
                }
            }
        }
    }
    None
}

fn epoch_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
