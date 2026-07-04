//! Kokoro-82M TTS integration.
//!
//! Kokoro is an 82M-parameter, Apache-2.0 TTS model with 8 languages and 54
//! preset voices.  It is the primary TTS backend when the model is present;
//! the original Piper pipeline in `tts_engine.rs` remains as a fallback.
//!
//! ## Model placement (offline, user-managed)
//! Place the ONNX weights at any of:
//!   - `$KOKORO_MODEL_PATH` (env var)
//!   - `~/.bonsai/models/kokoro/kokoro-v1.9.onnx`
//!   - `<app_data>/sidecars/kokoro/kokoro-v1.9.onnx`
//!
//! ## Worker
//! Kokoro is invoked via a thin Python sidecar (`kokoro_worker.py`) that reads
//! JSON from stdin and writes WAV bytes to stdout.  The worker is placed next
//! to the model file.  If neither model nor worker is found, the tool returns
//! an error that gracefully surfaces in the UI.

use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

use crate::tool_registry::{Tool, ToolResult};

// ── Voice catalogue ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInfo {
    pub name: String,
    pub language: String,
    pub gender: String,
}

/// Built-in Kokoro voice list (subset — all 54 available in the full model).
pub fn builtin_voices() -> Vec<VoiceInfo> {
    vec![
        VoiceInfo {
            name: "af_heart".into(),
            language: "en-us".into(),
            gender: "female".into(),
        },
        VoiceInfo {
            name: "af_bella".into(),
            language: "en-us".into(),
            gender: "female".into(),
        },
        VoiceInfo {
            name: "af_nicole".into(),
            language: "en-us".into(),
            gender: "female".into(),
        },
        VoiceInfo {
            name: "am_adam".into(),
            language: "en-us".into(),
            gender: "male".into(),
        },
        VoiceInfo {
            name: "am_michael".into(),
            language: "en-us".into(),
            gender: "male".into(),
        },
        VoiceInfo {
            name: "bf_emma".into(),
            language: "en-gb".into(),
            gender: "female".into(),
        },
        VoiceInfo {
            name: "bm_george".into(),
            language: "en-gb".into(),
            gender: "male".into(),
        },
        VoiceInfo {
            name: "ef_dora".into(),
            language: "es".into(),
            gender: "female".into(),
        },
        VoiceInfo {
            name: "ff_siwis".into(),
            language: "fr-fr".into(),
            gender: "female".into(),
        },
        VoiceInfo {
            name: "hf_alpha".into(),
            language: "hi".into(),
            gender: "female".into(),
        },
        VoiceInfo {
            name: "if_sara".into(),
            language: "it".into(),
            gender: "female".into(),
        },
        VoiceInfo {
            name: "jf_alpha".into(),
            language: "ja".into(),
            gender: "female".into(),
        },
        VoiceInfo {
            name: "pf_dora".into(),
            language: "pt-br".into(),
            gender: "female".into(),
        },
        VoiceInfo {
            name: "zf_xiaobei".into(),
            language: "zh".into(),
            gender: "female".into(),
        },
    ]
}

// ── Model / worker discovery ──────────────────────────────────────────────────

fn find_kokoro_model() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("KOKORO_MODEL_PATH") {
        let pb = PathBuf::from(&p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let candidates = [
        dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/models/kokoro/kokoro-v1.9.onnx"),
        dirs::data_local_dir()
            .unwrap_or_default()
            .join("com.bonsai.workspace/sidecars/kokoro/kokoro-v1.9.onnx"),
        PathBuf::from("sidecars/kokoro/kokoro-v1.9.onnx"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

fn find_kokoro_worker() -> Option<PathBuf> {
    if let Some(model) = find_kokoro_model() {
        let worker = model.parent()?.join("kokoro_worker.py");
        if worker.exists() {
            return Some(worker);
        }
    }
    // Fallback locations
    let candidates = [
        dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/sidecars/kokoro_worker.py"),
        PathBuf::from("sidecars/kokoro_worker.py"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

/// Check whether the Kokoro backend is available on this machine.
pub fn is_available() -> bool {
    find_kokoro_model().is_some() && find_kokoro_worker().is_some()
}

// ── Synthesis ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct KokoroRequest<'a> {
    text: &'a str,
    voice: &'a str,
    speed: f32,
}

pub async fn synthesize(
    text: &str,
    voice: Option<&str>,
    speed: Option<f32>,
) -> Result<crate::tts_engine::TtsResult, String> {
    let model = find_kokoro_model()
        .ok_or("Kokoro model not found. Place kokoro-v1.9.onnx in ~/.bonsai/models/kokoro/")?;
    let worker = find_kokoro_worker().ok_or("kokoro_worker.py not found alongside the model")?;

    let voice_name = voice.unwrap_or("af_heart");
    let speed_val = speed.unwrap_or(1.0f32).clamp(0.5, 2.0);

    let payload = serde_json::to_string(&KokoroRequest {
        text,
        voice: voice_name,
        speed: speed_val,
    })
    .map_err(|e| e.to_string())?;

    info!(
        chars = text.len(),
        voice = voice_name,
        "[kokoro] synthesizing"
    );
    let t0 = std::time::Instant::now();

    // Find python executable
    let python = which_python()?;

    let mut child = tokio::process::Command::new(&python)
        .arg(&worker)
        .arg("--model")
        .arg(&model)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to start kokoro_worker: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(payload.as_bytes())
            .await
            .map_err(|e| format!("stdin write: {e}"))?;
    }

    let out = tokio::time::timeout(Duration::from_secs(120), child.wait_with_output())
        .await
        .map_err(|_| "kokoro_worker timed out (120s)".to_string())?
        .map_err(|e| format!("kokoro_worker error: {e}"))?;

    if !out.status.success() {
        return Err(format!("kokoro_worker exited {:?}", out.status.code()));
    }

    // Worker writes raw WAV bytes to stdout.
    let wav = out.stdout;
    let sample_rate = 24000u32; // Kokoro outputs 24 kHz

    let bytes_per_sec = sample_rate as u64 * 2; // 16-bit mono
    let pcm_len = wav.len().saturating_sub(44) as u64; // strip WAV header from estimate
    let duration_ms = if bytes_per_sec > 0 {
        pcm_len * 1000 / bytes_per_sec
    } else {
        0
    };

    info!(
        duration_ms,
        elapsed_ms = t0.elapsed().as_millis(),
        "[kokoro] done"
    );

    Ok(crate::tts_engine::TtsResult {
        audio_b64: base64::engine::general_purpose::STANDARD.encode(&wav),
        mime_type: "audio/wav".into(),
        duration_ms,
        sample_rate,
    })
}

fn which_python() -> Result<PathBuf, String> {
    for candidate in &["python3", "python"] {
        if let Ok(out) = std::process::Command::new(candidate)
            .arg("--version")
            .output()
        {
            if out.status.success() {
                return Ok(PathBuf::from(candidate));
            }
        }
    }
    Err("python3/python not found in PATH".into())
}

// ── Tool: speak_text (Kokoro primary) ─────────────────────────────────────────

pub struct KokoroTtsTool {
    _marker: (),
}

impl KokoroTtsTool {
    pub fn new() -> Self {
        Self { _marker: () }
    }
}

#[async_trait]
impl Tool for KokoroTtsTool {
    fn name(&self) -> &str {
        "speak_text_kokoro"
    }

    fn description(&self) -> &str {
        "Convert text to speech using Kokoro-82M (high quality, 8 languages, 54 voices). \
         Args: {text: string, voice?: string, speed?: number}"
    }

    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let text = args["text"].as_str().ok_or("Missing 'text' argument")?;
        let voice = args["voice"].as_str();
        let speed = args["speed"].as_f64().map(|f| f as f32);

        if !is_available() {
            return Err(
                "Kokoro model not installed. Download kokoro-v1.9.onnx from \
                 huggingface.co/hexgrad/Kokoro-82M and place it in ~/.bonsai/models/kokoro/"
                    .into(),
            );
        }

        let result = synthesize(text, voice, speed).await?;
        Ok(ToolResult::json(&serde_json::json!({
            "audio_b64":   result.audio_b64,
            "mime_type":   result.mime_type,
            "duration_ms": result.duration_ms,
            "sample_rate": result.sample_rate,
            "voice":       voice.unwrap_or("af_heart"),
            "backend":     "kokoro-82m",
        })))
    }
}

// ── Tool: list_voices ─────────────────────────────────────────────────────────

pub struct ListVoicesTool;

#[async_trait]
impl Tool for ListVoicesTool {
    fn name(&self) -> &str {
        "list_tts_voices"
    }
    fn description(&self) -> &str {
        "List available TTS voices. Args: {language?: string, backend?: \"kokoro\"|\"piper\"}"
    }

    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let lang_filter = args["language"].as_str();
        let mut voices = builtin_voices();
        if let Some(lang) = lang_filter {
            voices.retain(|v| v.language.starts_with(lang));
        }
        Ok(ToolResult::json(&serde_json::json!({
            "voices": voices,
            "kokoro_available": is_available(),
        })))
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct KokoroSynthRequest {
    pub text: String,
    pub voice: Option<String>,
    pub speed: Option<f32>,
}

#[tauri::command]
pub async fn kokoro_synthesize(
    req: KokoroSynthRequest,
) -> Result<crate::tts_engine::TtsResult, String> {
    synthesize(&req.text, req.voice.as_deref(), req.speed).await
}

#[tauri::command]
pub async fn list_kokoro_voices(language: Option<String>) -> Result<Vec<VoiceInfo>, String> {
    let mut voices = builtin_voices();
    if let Some(lang) = language {
        voices.retain(|v| v.language.starts_with(&lang));
    }
    Ok(voices)
}

#[tauri::command]
pub async fn kokoro_available() -> Result<bool, String> {
    Ok(is_available())
}
