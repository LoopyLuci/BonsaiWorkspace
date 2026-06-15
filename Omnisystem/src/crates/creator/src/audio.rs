//! Audio generation tools.
//!
//! - [`MusicGenTool`]  — text-to-music via MusicGen (Meta AI).
//! - [`BarkTtsTool`]   — text-to-speech with voice cloning via Suno Bark.
//!
//! Production path: run models through candle or a Python sidecar,
//! returning WAV/OGG bytes stored in CAS.

use crate::{GenerateParams, GenerationResult, GenerativeTool};
use async_trait::async_trait;
use cas::CasStore;
use std::sync::Arc;

// ── MusicGen ──────────────────────────────────────────────────────────────────

pub struct MusicGenTool {
    pub cas: Arc<CasStore>,
}

impl MusicGenTool {
    pub fn new(cas: Arc<CasStore>) -> Self {
        Self { cas }
    }
}

#[async_trait]
impl GenerativeTool for MusicGenTool {
    async fn generate(&self, params: GenerateParams) -> anyhow::Result<GenerationResult> {
        let duration_sec = params.extra["duration_sec"].as_u64().unwrap_or(10).min(300);
        let model_size = params.extra["model"].as_str().unwrap_or("musicgen-small");

        // === Production path (stubbed) ===
        // 1. Tokenise prompt with EnCodec text tokeniser
        // 2. Run MusicGen autoregressive transformer
        // 3. Decode with EnCodec audio codec → PCM
        // 4. Write WAV header + PCM → bytes

        // Placeholder: silence WAV header + zeroed samples at 44100 Hz mono
        let num_samples = 44100u32 * duration_sec as u32;
        let mut wav = wav_header(num_samples, 1, 44100, 16);
        wav.extend(vec![0u8; num_samples as usize * 2]); // 16-bit samples

        let key = self.cas.put(&wav, "audio/wav").await?;
        Ok(GenerationResult {
            cas_key: key,
            metadata: serde_json::json!({
                "model":        model_size,
                "duration_sec": duration_sec,
                "sample_rate":  44100,
                "channels":     1,
                "prompt":       params.prompt,
            }),
        })
    }
}

// ── Bark TTS ──────────────────────────────────────────────────────────────────

pub struct BarkTtsTool {
    pub cas: Arc<CasStore>,
}

impl BarkTtsTool {
    pub fn new(cas: Arc<CasStore>) -> Self {
        Self { cas }
    }
}

#[async_trait]
impl GenerativeTool for BarkTtsTool {
    async fn generate(&self, params: GenerateParams) -> anyhow::Result<GenerationResult> {
        let voice_preset = params.extra["voice_preset"]
            .as_str()
            .unwrap_or("v2/en_speaker_6");

        // === Production path (stubbed) ===
        // 1. Text → semantic tokens via Bark text encoder
        // 2. Semantic → coarse tokens via Bark coarse model
        // 3. Coarse → fine tokens via Bark fine model
        // 4. EnCodec decode → 24 kHz PCM → WAV

        let text_len = params.prompt.len();
        let est_duration_sec = (text_len / 15 + 1) as u32; // ~15 chars/sec
        let num_samples = 24000u32 * est_duration_sec;
        let mut wav = wav_header(num_samples, 1, 24000, 16);
        wav.extend(vec![0u8; num_samples as usize * 2]);

        let key = self.cas.put(&wav, "audio/wav").await?;
        Ok(GenerationResult {
            cas_key: key,
            metadata: serde_json::json!({
                "model":        "bark",
                "voice_preset": voice_preset,
                "text":         params.prompt,
                "duration_sec": est_duration_sec,
                "sample_rate":  24000,
            }),
        })
    }
}

// ── WAV helper ────────────────────────────────────────────────────────────────

fn wav_header(num_samples: u32, channels: u16, sample_rate: u32, bits: u16) -> Vec<u8> {
    let byte_rate = sample_rate * channels as u32 * bits as u32 / 8;
    let block_align = channels * bits / 8;
    let data_size = num_samples * channels as u32 * bits as u32 / 8;
    let chunk_size = 36 + data_size;

    let mut h = Vec::with_capacity(44);
    h.extend_from_slice(b"RIFF");
    h.extend_from_slice(&chunk_size.to_le_bytes());
    h.extend_from_slice(b"WAVE");
    h.extend_from_slice(b"fmt ");
    h.extend_from_slice(&16u32.to_le_bytes());
    h.extend_from_slice(&1u16.to_le_bytes()); // PCM
    h.extend_from_slice(&channels.to_le_bytes());
    h.extend_from_slice(&sample_rate.to_le_bytes());
    h.extend_from_slice(&byte_rate.to_le_bytes());
    h.extend_from_slice(&block_align.to_le_bytes());
    h.extend_from_slice(&bits.to_le_bytes());
    h.extend_from_slice(b"data");
    h.extend_from_slice(&data_size.to_le_bytes());
    h
}
