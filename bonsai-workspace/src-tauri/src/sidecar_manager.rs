//! Whisper server manager.
//!
//! Owns the `whisper-server` child process (auto-downloaded by `bootstrap`),
//! exposes an async `transcribe` interface backed by the HTTP `/inference`
//! endpoint, and cleans up the process on drop.

use reqwest::Client;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::bootstrap;

// ── WhisperManager ────────────────────────────────────────────────────────────

/// Thread-safe handle to a running `whisper-server` process.
/// Wrap in `Arc` so it can be shared across Tauri command handlers.
pub struct WhisperManager {
    url:     String,
    client:  Client,
    // Mutex so Drop can kill the child without requiring &mut self
    process: Mutex<Option<std::process::Child>>,
}

impl WhisperManager {
    /// Spawn `whisper-server` (if the binary exists) and return a manager.
    /// If the binary is absent the manager still returns, but `transcribe` will
    /// fail gracefully until bootstrap completes and the app reloads.
    pub fn new(app: &AppHandle) -> Self {
        let port: u16 = {
            use rand::Rng;
            rand::thread_rng().gen_range(40_000u16..50_000u16)
        };
        let url = format!("http://127.0.0.1:{}", port);

        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_default();

        let process = try_spawn(app, port);

        // Background readiness probe — emits "whisper-ready" once healthy
        {
            let probe_url = url.clone();
            let handle    = app.clone();
            tauri::async_runtime::spawn(async move {
                let probe = Client::new();
                for _ in 0..120 {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    if probe
                        .get(format!("{}/health", probe_url))
                        .send()
                        .await
                        .is_ok_and(|r| r.status().is_success())
                    {
                        let _ = handle.emit("whisper-ready", ());
                        return;
                    }
                }
                eprintln!("[whisper] Readiness timeout — transcription unavailable until restart");
            });
        }

        Self {
            url,
            client,
            process: Mutex::new(process),
        }
    }

    /// Post raw WAV bytes to `/inference` as `multipart/form-data`.
    /// Retries up to 5 times with exponential back-off.
    pub async fn transcribe(&self, audio_data: Vec<u8>) -> Result<String, String> {
        for attempt in 0..5u32 {
            let form = reqwest::multipart::Form::new().part(
                "file",
                reqwest::multipart::Part::bytes(audio_data.clone())
                    .file_name("audio.wav")
                    .mime_str("audio/wav")
                    .unwrap(),
            );
            match self
                .client
                .post(format!("{}/inference", self.url))
                .multipart(form)
                .send()
                .await
            {
                Ok(r) if r.status().is_success() => {
                    let text = r.text().await.map_err(|e| e.to_string())?;
                    return Ok(text.trim().to_string());
                }
                Ok(r) => eprintln!("[whisper] attempt {attempt}: HTTP {}", r.status()),
                Err(e) => eprintln!("[whisper] attempt {attempt}: {e}"),
            }
            tokio::time::sleep(Duration::from_millis(600 * u64::from(attempt + 1))).await;
        }
        Err("Whisper transcription failed after 5 attempts".into())
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

impl Drop for WhisperManager {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.process.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
            }
        }
    }
}

// ── Process spawn ─────────────────────────────────────────────────────────────

fn try_spawn(app: &AppHandle, port: u16) -> Option<std::process::Child> {
    let exe   = bootstrap::whisper_exe(app);
    let model = bootstrap::whisper_model(app);

    if !exe.exists() {
        eprintln!("[whisper] binary not found (bootstrap pending): {:?}", exe);
        return None;
    }

    let dir = exe.parent().unwrap_or(&exe).to_path_buf();

    let mut cmd = std::process::Command::new(&exe);
    cmd.args([
        "--port",  &port.to_string(),
        "--host",  "127.0.0.1",
        "--model", &model.to_string_lossy(),
        "--log-disable",
    ])
    .current_dir(&dir)
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    match cmd.spawn() {
        Ok(child) => Some(child),
        Err(e) => {
            eprintln!("[whisper] spawn failed: {e}");
            None
        }
    }
}
