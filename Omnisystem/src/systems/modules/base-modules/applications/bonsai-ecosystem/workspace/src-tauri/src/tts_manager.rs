use serde::Serialize;
/// Piper TTS sidecar manager.
///
/// Synthesizes speech via the Piper binary (--output_file + --json for phoneme timing).
/// Falls back gracefully when the binary or voice model is absent.
/// Audio playback via rodio. Viseme timeline is emitted as a Tauri event.
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct VisemeEvent {
    pub viseme_id: u8,
    pub start_ms: u32,
}

#[derive(Debug, Serialize)]
pub struct TtsVisemePayload {
    pub duration_ms: u32,
    pub events: Vec<VisemeEvent>,
}

pub struct SynthResult {
    pub wav_bytes: Vec<u8>,
    pub duration_ms: u32,
    pub viseme_timeline: Vec<VisemeEvent>,
}

// ── TtsManager ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TtsManager {
    state: Arc<TtsState>,
}

struct TtsState {
    voice: Mutex<String>,
    speed: Mutex<f32>,
    speaking: Arc<AtomicBool>,
    stop_flag: Arc<AtomicBool>,
    piper_exe: Option<PathBuf>,
}

impl TtsManager {
    pub fn new(app: &AppHandle) -> Self {
        let piper_exe = find_piper_exe(app);
        TtsManager {
            state: Arc::new(TtsState {
                voice: Mutex::new("en_US-amy-medium".into()),
                speed: Mutex::new(1.0),
                speaking: Arc::new(AtomicBool::new(false)),
                stop_flag: Arc::new(AtomicBool::new(false)),
                piper_exe,
            }),
        }
    }

    pub fn set_voice(&self, voice: &str) {
        *self.state.voice.lock().unwrap() = voice.to_string();
    }

    pub fn set_speed(&self, speed: f32) {
        *self.state.speed.lock().unwrap() = speed.clamp(0.5, 2.0);
    }

    pub fn stop(&self) {
        self.state.stop_flag.store(true, Ordering::SeqCst);
    }

    pub fn is_available(&self) -> bool {
        self.state.piper_exe.is_some()
    }

    /// Synthesize text and return WAV bytes + viseme timeline.
    pub async fn synthesize(&self, text: &str) -> Result<SynthResult, String> {
        let piper = match &self.state.piper_exe {
            Some(p) => p.clone(),
            None => return Err("Piper TTS binary not found".into()),
        };

        let voice = self.state.voice.lock().unwrap().clone();
        let speed = *self.state.speed.lock().unwrap();

        // Find voice model file
        let model_path = find_voice_model(&piper, &voice)?;

        // Write text to a temp file as Piper reads stdin or file
        let tmp_dir = std::env::temp_dir();
        let wav_out = tmp_dir.join("bonsai_tts_out.wav");

        // Run Piper:
        //   piper --model <model> --output_file <wav> --json_output <json> --length_scale <1/speed>
        let length_scale = 1.0 / speed;
        let mut piper_cmd = Command::new(&piper);
        piper_cmd
            .arg("--model")
            .arg(&model_path)
            .arg("--output_file")
            .arg(&wav_out)
            .arg("--length_scale")
            .arg(length_scale.to_string())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            piper_cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let output = piper_cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn Piper: {e}"))?;

        // Write text to stdin
        let mut child = output;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(text.as_bytes())
                .await
                .map_err(|e| format!("Failed to write to Piper stdin: {e}"))?;
        }

        let out = child
            .wait_with_output()
            .await
            .map_err(|e| format!("Piper wait error: {e}"))?;

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!("Piper error: {stderr}"));
        }

        // Read WAV
        let wav_bytes =
            std::fs::read(&wav_out).map_err(|e| format!("Failed to read WAV output: {e}"))?;

        // Parse phoneme JSON from stdout (Piper writes timing JSON to stdout with --output_file)
        let json_str = String::from_utf8_lossy(&out.stdout);
        let viseme_timeline = parse_piper_phonemes(&json_str);

        // Calculate duration from WAV header
        let duration_ms = wav_duration_ms(&wav_bytes).unwrap_or(0);

        Ok(SynthResult {
            wav_bytes,
            duration_ms,
            viseme_timeline,
        })
    }

    /// Synthesize and play. Emits tts-visemes / tts-started / tts-done / tts-error events.
    pub async fn speak(&self, app: &AppHandle, text: &str) -> Result<(), String> {
        if self.state.speaking.swap(true, Ordering::SeqCst) {
            return Err("Already speaking".into());
        }
        self.state.stop_flag.store(false, Ordering::SeqCst);

        let result = self.synthesize(text).await;

        match result {
            Err(e) => {
                self.state.speaking.store(false, Ordering::SeqCst);
                let _ = app.emit_to(
                    tauri::EventTarget::webview_window("assistant"),
                    "tts-error",
                    &e,
                );
                return Err(e);
            }
            Ok(synth) => {
                // Emit viseme timeline so the avatar can start animating
                let payload = TtsVisemePayload {
                    duration_ms: synth.duration_ms,
                    events: synth.viseme_timeline,
                };
                let _ = app.emit_to(
                    tauri::EventTarget::webview_window("assistant"),
                    "tts-visemes",
                    &payload,
                );
                let _ = app.emit_to(
                    tauri::EventTarget::webview_window("assistant"),
                    "tts-started",
                    &synth.duration_ms,
                );

                // Play via rodio in a blocking thread (rodio is synchronous)
                let wav_bytes = synth.wav_bytes.clone();
                let stop_flag = Arc::clone(&self.state.stop_flag);
                let duration_ms = synth.duration_ms;
                let app2 = app.clone();
                let speaking = Arc::clone(&self.state.speaking);

                tokio::task::spawn_blocking(move || {
                    play_wav_rodio(&wav_bytes, &stop_flag);
                    speaking.store(false, Ordering::SeqCst);
                    let _ = app2.emit_to(
                        tauri::EventTarget::webview_window("assistant"),
                        "tts-done",
                        &duration_ms,
                    );
                });
            }
        }

        Ok(())
    }
}

// ── Rodio playback ────────────────────────────────────────────────────────────

fn play_wav_rodio(wav_bytes: &[u8], stop: &AtomicBool) {
    use rodio::{Decoder, OutputStream, Sink};
    use std::io::Cursor;

    let (_stream, stream_handle) = match OutputStream::try_default() {
        Ok(o) => o,
        Err(e) => {
            tracing::error!(error=%e, "[tts] rodio output error");
            return;
        }
    };
    let sink = match Sink::try_new(&stream_handle) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error=%e, "[tts] rodio sink error");
            return;
        }
    };

    let cursor = Cursor::new(wav_bytes.to_vec());
    let decoder = match Decoder::new_wav(cursor) {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(error=%e, "[tts] WAV decode error");
            return;
        }
    };

    sink.append(decoder);

    // Poll until done or stop requested
    while !sink.empty() {
        if stop.load(Ordering::SeqCst) {
            sink.stop();
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn find_piper_exe(app: &AppHandle) -> Option<PathBuf> {
    // Check next to the app binary first
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    for name in &["piper.exe", "piper"] {
        let p = dir.join(name);
        if p.exists() {
            return Some(p);
        }
    }
    // Check app data dir
    if let Ok(data) = app.path().app_data_dir() {
        for name in &["piper.exe", "piper"] {
            let p = data.join("piper").join(name);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

fn find_voice_model(piper_exe: &PathBuf, voice: &str) -> Result<PathBuf, String> {
    let dir = piper_exe.parent().unwrap_or(piper_exe.as_path());
    // Look for <voice>.onnx in same dir or voices/ subdirectory
    for sub in &["", "voices"] {
        let path = if sub.is_empty() {
            dir.join(format!("{voice}.onnx"))
        } else {
            dir.join(sub).join(format!("{voice}.onnx"))
        };
        if path.exists() {
            return Ok(path);
        }
    }
    Err(format!(
        "Voice model '{voice}.onnx' not found near piper executable"
    ))
}

/// Parse Piper's phoneme timing JSON (written to stdout) into VisemeEvents.
/// Piper outputs one JSON object per line with {"phonemes":[{"phoneme":"..","start_ms":N}]}.
fn parse_piper_phonemes(json_str: &str) -> Vec<VisemeEvent> {
    let mut events: Vec<VisemeEvent> = Vec::new();

    for line in json_str.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(phonemes) = val["phonemes"].as_array() {
                for ph in phonemes {
                    let phoneme = ph["phoneme"].as_str().unwrap_or("");
                    let start_ms = ph["start_ms"].as_u64().unwrap_or(0) as u32;
                    let viseme_id = phoneme_to_viseme(phoneme);
                    events.push(VisemeEvent {
                        viseme_id,
                        start_ms,
                    });
                }
            }
        }
    }

    if events.is_empty() {
        // No phoneme data — generate a simple silence→talking→silence sequence
        events.push(VisemeEvent {
            viseme_id: 0,
            start_ms: 0,
        });
    }

    events
}

/// Map Piper phoneme strings to Preston Blair 14-viseme IDs.
fn phoneme_to_viseme(ph: &str) -> u8 {
    match ph {
        "AE" | "AH" | "AX" => 1,
        "EH" | "ER" | "AXR" => 2,
        "IY" | "IH" | "IX" => 3,
        "AW" | "AO" | "AA" => 4,
        "OW" | "UH" => 5,
        "UW" | "OY" | "AY" => 6,
        "M" | "B" | "P" => 7,
        "F" | "V" => 8,
        "TH" | "DH" => 9,
        "T" | "D" | "S" | "Z" => 10,
        "CH" | "SH" | "ZH" | "JH" => 11,
        "N" | "NG" | "L" => 12,
        "R" => 13,
        _ => 0, // Silence / unknown
    }
}

/// Extract playback duration from a WAV header (bytes 28–31 = data rate, 4–7 = file size, etc.)
/// Simple heuristic: (data_chunk_size / byte_rate) * 1000
fn wav_duration_ms(wav: &[u8]) -> Option<u32> {
    if wav.len() < 44 {
        return None;
    }
    // bytes 28–31: nAvgBytesPerSec (little-endian)
    let byte_rate = u32::from_le_bytes([wav[28], wav[29], wav[30], wav[31]]) as u64;
    if byte_rate == 0 {
        return None;
    }
    // bytes 40–43: data chunk size
    let data_size = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as u64;
    Some(((data_size * 1000) / byte_rate) as u32)
}
