//! Vision oracle self-play training loop.
//!
//! Picks workspace images → runs YOLO + PixAI as ground-truth oracles →
//! asks BonsAI to describe the image → compares with oracle output via
//! Critic → generates DPO triples → feeds to adapter_manager.
//!
//! The loop runs in the background and surfaces progress through the
//! `vision_training_progress` Tauri event.

use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tracing::{info, warn};

use crate::adapter_manager::{self, DpoTriple};
use crate::critic::Critic;
use crate::model_orchestrator::ModelOrchestrator;
use crate::multimodal::pixai_tagger;
use crate::multimodal::yolo;

// ── Config ────────────────────────────────────────────────────────────────────

const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp"];
const MAX_IMAGES_PER_RUN: usize = 200;
const SCORE_THRESHOLD: f32 = 0.65; // below this → generate DPO triple
const DPO_SOURCE: &str = "vision_oracle_self_play";

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionTrainingConfig {
    pub workspace_dirs: Vec<PathBuf>,
    pub max_images: usize,
    pub score_threshold: f32,
}

impl Default for VisionTrainingConfig {
    fn default() -> Self {
        Self {
            workspace_dirs: vec![],
            max_images: MAX_IMAGES_PER_RUN,
            score_threshold: SCORE_THRESHOLD,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionTrainingProgress {
    pub total: usize,
    pub processed: usize,
    pub triples: usize,
    pub skipped: usize,
    pub current: String,
    pub done: bool,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn collect_images(dirs: &[PathBuf], max: usize) -> Vec<PathBuf> {
    let mut images = Vec::new();
    for dir in dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if IMAGE_EXTS.contains(&ext.to_lowercase().as_str()) {
                            images.push(path);
                            if images.len() >= max {
                                return images;
                            }
                        }
                    }
                }
            }
        }
    }
    images
}

/// Build a ground-truth description from YOLO detections + PixAI tags.
async fn build_oracle_description(image_path: &str) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();

    // YOLO object detections
    if yolo::is_available() {
        if let Ok(result) = yolo::detect_objects_cmd(image_path.to_string(), None, None).await {
            if !result.objects.is_empty() {
                let labels: Vec<String> = result
                    .objects
                    .iter()
                    .map(|d| format!("{} ({:.0}%)", d.class_name, d.confidence * 100.0))
                    .take(10)
                    .collect();
                parts.push(format!("Objects detected: {}.", labels.join(", ")));
            }
        }
    }

    // PixAI tags
    if let Some(tags) = pixai_tagger::tag_image_internal(image_path).await {
        if !tags.top_tags.is_empty() {
            parts.push(format!("Image tags: {}.", tags.top_tags.join(", ")));
            parts.push(format!("Style category: {}.", tags.dominant_category));
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

/// Ask the running VLM slot to describe an image.
async fn ask_bonsai_description(image_path: &str, slot_url: &str) -> Option<String> {
    use base64::Engine as _;
    let image_bytes = tokio::fs::read(image_path).await.ok()?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&image_bytes);
    let ext = Path::new(image_path)
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
                { "type": "text", "text": "Describe this image in detail. List all visible objects, their positions, colours, and any notable style or artistic elements." }
            ]
        }],
        "max_tokens": 512,
        "temperature": 0.3
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{slot_url}/v1/chat/completions"))
        .json(&payload)
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .ok()?;

    let body: serde_json::Value = resp.json().await.ok()?;
    body["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
}

fn dpo_output_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai/training/vision_dpo.jsonl")
}

// ── Main loop ─────────────────────────────────────────────────────────────────

pub async fn run_vision_training_loop<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    orchestrator: Arc<ModelOrchestrator>,
    config: VisionTrainingConfig,
    cancel: Arc<AtomicBool>,
    slot_url: String,
) {
    let images = collect_images(&config.workspace_dirs, config.max_images);
    let total = images.len();
    info!("[vision_training] starting loop: {} images", total);

    let critic = Critic::new(orchestrator, config.score_threshold, 0);
    let dpo_path = dpo_output_path();
    let mut processed = 0usize;
    let mut triples = 0usize;
    let mut skipped = 0usize;

    for image_path in &images {
        if cancel.load(Ordering::Relaxed) {
            info!("[vision_training] cancelled");
            break;
        }

        let path_str = image_path.to_string_lossy().to_string();
        let _ = app_handle.emit(
            "vision_training_progress",
            VisionTrainingProgress {
                total,
                processed,
                triples,
                skipped,
                current: path_str.clone(),
                done: false,
            },
        );

        // 1. Oracle ground truth
        let Some(oracle) = build_oracle_description(&path_str).await else {
            skipped += 1;
            processed += 1;
            continue;
        };

        // 2. BonsAI response via running VLM
        let Some(bonsai_response) = ask_bonsai_description(&path_str, &slot_url).await else {
            warn!("[vision_training] VLM slot unavailable for {}", path_str);
            skipped += 1;
            processed += 1;
            continue;
        };

        // 3. Score
        let prompt = format!("Describe this image: {path_str}");
        let score = critic.score(&prompt, &bonsai_response).await;

        if score < config.score_threshold {
            let triple = DpoTriple {
                prompt: prompt.clone(),
                chosen: oracle.clone(),
                rejected: bonsai_response.clone(),
                source: DPO_SOURCE,
            };
            if let Err(e) = adapter_manager::write_dpo_triples(&dpo_path, &[triple]) {
                warn!("[vision_training] failed to write triple: {e}");
            } else {
                triples += 1;
                info!("[vision_training] triple #{triples} written (score={score:.2})");
            }
        }

        processed += 1;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let _ = app_handle.emit(
        "vision_training_progress",
        VisionTrainingProgress {
            total,
            processed,
            triples,
            skipped,
            current: String::new(),
            done: true,
        },
    );
    info!("[vision_training] done — {processed}/{total} processed, {triples} triples generated");
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn start_vision_training(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
    workspace_dirs: Vec<String>,
    slot_url: Option<String>,
) -> Result<String, String> {
    let dirs: Vec<PathBuf> = workspace_dirs
        .iter()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect();

    if dirs.is_empty() {
        return Err("No valid workspace directories provided.".into());
    }

    let config = VisionTrainingConfig {
        workspace_dirs: dirs,
        ..Default::default()
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let url = slot_url.unwrap_or_else(|| "http://127.0.0.1:8080".into());
    let orch = state.orchestrator.clone();

    tokio::spawn(run_vision_training_loop(
        app_handle, orch, config, cancel, url,
    ));
    Ok("Vision training loop started.".into())
}

#[tauri::command]
pub async fn vision_training_oracles_available() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "yolo":  yolo::is_available(),
        "pixai": pixai_tagger::is_available(),
        "both":  yolo::is_available() && pixai_tagger::is_available(),
    }))
}
