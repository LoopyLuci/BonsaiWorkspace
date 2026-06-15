//! Text-to-speech via Piper TTS sidecar.
//! Piper writes raw PCM (16-bit LE, 22050 Hz mono) to stdout.
//! We prepend a WAV header and return the bytes as a base64 string.

use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine as _;
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

#[derive(Debug, Serialize)]
pub struct TtsResult {
    /// WAV file as base64 string — play directly in Web Audio API.
    pub audio_b64: String,
    /// MIME type, always "audio/wav".
    pub mime_type: String,
    pub duration_ms: u64,
    pub sample_rate: u32,
}

#[tauri::command]
pub async fn tts_synthesize(text: String, voice: Option<String>) -> Result<TtsResult, String> {
    synthesize_speech(&text, voice.as_deref()).await
}

pub async fn synthesize_speech(text: &str, voice: Option<&str>) -> Result<TtsResult, String> {
    let piper = find_piper()?;
    let model = find_voice_model(voice)?;

    info!(chars = text.len(), model = %model.display(), "[tts] synthesizing");

    let t0 = std::time::Instant::now();

    let mut child = tokio::process::Command::new(&piper)
        .args(["--model", &model.to_string_lossy(), "--output-raw"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to start piper: {e}"))?;

    // Write text to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .await
            .map_err(|e| format!("Stdin write: {e}"))?;
    }

    let out = tokio::time::timeout(Duration::from_secs(60), child.wait_with_output())
        .await
        .map_err(|_| "Piper timed out (60s)".to_string())?
        .map_err(|e| format!("Piper error: {e}"))?;

    if !out.status.success() {
        return Err(format!("Piper exited {:?}", out.status.code()));
    }

    let pcm = out.stdout;
    let sample_rate = 22050u32;
    let channels = 1u16;
    let bits = 16u16;
    let wav = build_wav(&pcm, sample_rate, channels, bits);

    let elapsed = t0.elapsed().as_millis() as u64;
    // Estimate duration from PCM size: bytes / (sample_rate * channels * bits/8)
    let bytes_per_sec = sample_rate as u64 * channels as u64 * (bits as u64 / 8);
    let duration_ms = if bytes_per_sec > 0 {
        (pcm.len() as u64 * 1000) / bytes_per_sec
    } else {
        0
    };

    info!(
        duration_ms,
        elapsed_ms = elapsed,
        "[tts] synthesis complete"
    );

    Ok(TtsResult {
        audio_b64: base64::engine::general_purpose::STANDARD.encode(&wav),
        mime_type: "audio/wav".into(),
        duration_ms,
        sample_rate,
    })
}

// ── WAV header ────────────────────────────────────────────────────────────────

fn build_wav(pcm: &[u8], sample_rate: u32, channels: u16, bits_per_sample: u16) -> Vec<u8> {
    let data_len = pcm.len() as u32;
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;
    let mut w = Vec::with_capacity(44 + pcm.len());

    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data_len).to_le_bytes());
    w.extend_from_slice(b"WAVE");
    w.extend_from_slice(b"fmt ");
    w.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&channels.to_le_bytes());
    w.extend_from_slice(&sample_rate.to_le_bytes());
    w.extend_from_slice(&byte_rate.to_le_bytes());
    w.extend_from_slice(&block_align.to_le_bytes());
    w.extend_from_slice(&bits_per_sample.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_len.to_le_bytes());
    w.extend_from_slice(pcm);
    w
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn find_piper() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("PIPER_PATH") {
        let p = PathBuf::from(&path);
        if p.exists() {
            return Ok(p);
        }
    }
    let candidates = [
        dirs::data_local_dir()
            .unwrap_or_default()
            .join("com.bonsai.workspace")
            .join("sidecars")
            .join("piper.exe"),
        PathBuf::from("sidecars").join("piper.exe"),
        PathBuf::from("piper.exe"),
        PathBuf::from("piper"),
    ];
    for p in &candidates {
        if p.exists() {
            return Ok(p.clone());
        }
    }
    Err("piper not found. Set PIPER_PATH or place piper.exe in sidecars/.".into())
}

fn find_voice_model(voice: Option<&str>) -> Result<PathBuf, String> {
    let voices_dir = dirs::data_local_dir()
        .unwrap_or_default()
        .join("com.bonsai.workspace")
        .join("voices");

    if let Some(name) = voice {
        let p = voices_dir.join(name);
        if p.exists() {
            return Ok(p);
        }
        // Try with .onnx extension
        let p2 = voices_dir.join(format!("{name}.onnx"));
        if p2.exists() {
            return Ok(p2);
        }
        return Err(format!("Voice model not found: {name}"));
    }

    // Auto-discover first .onnx file
    if let Ok(mut rd) = std::fs::read_dir(&voices_dir) {
        while let Some(Ok(entry)) = rd.next() {
            if entry
                .path()
                .extension()
                .map(|e| e == "onnx")
                .unwrap_or(false)
            {
                return Ok(entry.path());
            }
        }
    }

    Err("No voice model found. Place a .onnx model in AppData/com.bonsai.workspace/voices/.".into())
}
