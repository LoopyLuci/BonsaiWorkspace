//! BonsAI music generation engine.
//!
//! `generate_wav(prompt, duration_secs)` returns raw WAV bytes (IEEE float 32-bit, 44100 Hz, mono).
//! It tries the `bonsai-music-worker` sidecar first; if the binary is absent it falls back to the
//! inline Rust synthesizer embedded in the worker (duplicated here so the main app has no hard
//! dependency on the sidecar being present).
//!
//! The sidecar protocol uses TCP, not stdin/stdout, to allow the worker to be kept warm for
//! fast follow-up requests.  The worker prints `BONSAI_MUSIC_PORT=<port>` on stdout line 1 then
//! accepts one long-lived TCP connection.

use std::sync::OnceLock;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::{info, warn};
use std::sync::Arc;

// ── Persistent worker handle ──────────────────────────────────────────────────

struct WorkerHandle {
    conn:  TcpStream,
    req_id: u64,
}

static WORKER: OnceLock<Arc<Mutex<Option<WorkerHandle>>>> = OnceLock::new();

fn worker_cell() -> &'static Arc<Mutex<Option<WorkerHandle>>> {
    WORKER.get_or_init(|| Arc::new(Mutex::new(None)))
}

fn worker_binary_path() -> Option<std::path::PathBuf> {
    // 1. Same dir as this executable (production / release)
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.with_file_name(if cfg!(target_os = "windows") {
            "bonsai-music-worker.exe"
        } else {
            "bonsai-music-worker"
        });
        if sibling.exists() { return Some(sibling); }
    }
    // 2. Workspace target/debug (dev workflow — Cargo places workspace binaries here)
    let bin_name = if cfg!(target_os = "windows") { "bonsai-music-worker.exe" } else { "bonsai-music-worker" };
    // Walk up from CARGO_MANIFEST_DIR until we find a target/ dir (handles workspace layout)
    let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..4 {
        for profile in &["debug", "release"] {
            let candidate = dir.join("target").join(profile).join(bin_name);
            if candidate.exists() { return Some(candidate); }
        }
        if !dir.pop() { break; }
    }
    None
}

async fn ensure_worker() -> Option<()> {
    let mut guard = worker_cell().lock().await;
    if guard.is_some() { return Some(()); }

    let bin = worker_binary_path()?;
    let mut child = Command::new(&bin)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    let stdout = child.stdout.take()?;
    let mut lines = BufReader::new(stdout).lines();

    // Read first line to get the port
    let port: u16 = tokio::time::timeout(Duration::from_secs(5), async {
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(rest) = line.strip_prefix("BONSAI_MUSIC_PORT=") {
                if let Ok(p) = rest.trim().parse::<u16>() {
                    return Some(p);
                }
            }
        }
        None
    }).await.ok()??;

    let conn = tokio::time::timeout(
        Duration::from_secs(3),
        TcpStream::connect(("127.0.0.1", port))
    ).await.ok()?.ok()?;

    info!(port, "[music] worker connected");
    // Detach child (runs independently; will exit when connection closes)
    tokio::spawn(async move { let _ = child.wait().await; });

    *guard = Some(WorkerHandle { conn, req_id: 0 });
    Some(())
}

/// Generate WAV bytes for `prompt`, approximately `duration_secs` long.
/// Returns raw WAV (RIFF/IEEE-float-32/44100 Hz/mono).
pub async fn generate_wav(prompt: &str, duration_secs: f32) -> Vec<u8> {
    // Try the persistent sidecar worker first
    if ensure_worker().await.is_some() {
        let mut guard = worker_cell().lock().await;
        if let Some(wh) = guard.as_mut() {
            wh.req_id += 1;
            let id = wh.req_id;
            let req = format!("{id}|{duration_secs}|{prompt}\n");

            let result = async {
                wh.conn.write_all(req.as_bytes()).await?;
                wh.conn.flush().await?;

                let mut reader = BufReader::new(&mut wh.conn);
                let mut header = String::new();
                reader.read_line(&mut header).await?;

                // `OK <id>|<byte_len>\n`
                let byte_len: usize = header
                    .trim()
                    .split('|')
                    .last()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                let mut wav = vec![0u8; byte_len];
                reader.read_exact(&mut wav).await?;
                Ok::<Vec<u8>, std::io::Error>(wav)
            }.await;

            match result {
                Ok(wav) if !wav.is_empty() => {
                    info!(bytes = wav.len(), "[music] generated via worker");
                    return wav;
                }
                Err(e) => {
                    warn!("[music] worker error: {e}; falling back to inline synth");
                    // Reset so next call re-spawns
                    *guard = None;
                }
                _ => {}
            }
        }
    }

    // Fallback: inline synthesis
    info!("[music] generating inline (no worker binary)");
    let prompt_owned = prompt.to_string();
    tokio::task::spawn_blocking(move || {
        inline_generate_wav(&prompt_owned, duration_secs)
    }).await.unwrap_or_default()
}

// ── Inline synthesizer (mirrors bonsai-music-worker) ─────────────────────────

fn inline_generate_wav(prompt: &str, duration: f32) -> Vec<u8> {
    use std::f32::consts::PI;

    let sr = 44100u32;
    let n = (duration * sr as f32) as usize;
    let mut mix = vec![0.0f32; n];

    let lower = prompt.to_lowercase();
    let bpm = if lower.contains("slow") || lower.contains("ambient") { 70.0f32 }
              else if lower.contains("fast") || lower.contains("energetic") { 140.0 }
              else { 100.0 };
    let minor = lower.contains("minor") || lower.contains("sad") || lower.contains("dark");
    let scale: &[f32] = if minor {
        &[0.0, 2.0, 3.0, 5.0, 7.0, 8.0, 10.0]
    } else {
        &[0.0, 2.0, 4.0, 5.0, 7.0, 9.0, 11.0]
    };
    let root_hz = 220.0f32;
    let beat_dur = 60.0 / bpm;
    let has_drums = !lower.contains("ambient");

    let semitone_ratio = |s: f32| 2.0f32.powf(s / 12.0);
    let adsr = |t: f32, dur: f32, a: f32, d: f32, s: f32, r: f32| {
        let rs = dur - r;
        if t < a { t / a }
        else if t < a + d { 1.0 - (1.0 - s) * ((t - a) / d) }
        else if t < rs { s }
        else if t < dur { s * (1.0 - (t - rs) / r) }
        else { 0.0 }
    };

    // Bass
    {
        let pattern = [0usize, 0, 4, 4, 2, 2, 3, 3];
        let mut ph = 0.0f32;
        for i in 0..n {
            let t = i as f32 / sr as f32;
            let bi = ((t / beat_dur) as usize) % pattern.len();
            let freq = root_hz * 0.5 * semitone_ratio(scale[pattern[bi] % scale.len()]);
            let env = adsr(t % beat_dur, beat_dur * 0.9, 0.01, 0.1, 0.6, 0.15);
            mix[i] += ((2.0 * PI * ph).sin()) * env * 0.3;
            ph = (ph + freq / sr as f32).fract();
        }
    }

    // Chords
    {
        let chord_roots = [0usize, 3, 4, 2];
        let cd = beat_dur * 4.0;
        let mut phs = [0.0f32; 3];
        for i in 0..n {
            let t = i as f32 / sr as f32;
            let ci = ((t / cd) as usize) % chord_roots.len();
            let r = chord_roots[ci];
            let degs = [r % scale.len(), (r + 2) % scale.len(), (r + 4) % scale.len()];
            let env = adsr(t % cd, cd * 0.95, 0.08, 0.2, 0.7, 0.3);
            let mut s = 0.0f32;
            for (k, &d) in degs.iter().enumerate() {
                let freq = root_hz * semitone_ratio(scale[d]);
                s += (2.0 * PI * phs[k]).sin();
                phs[k] = (phs[k] + freq / sr as f32).fract();
            }
            mix[i] += s * env * 0.1;
        }
    }

    // Melody
    {
        let mel = [0usize, 2, 4, 7, 4, 2, 5, 3, 1, 4, 6, 4, 2, 1, 3, 0];
        let nd = beat_dur * 0.5;
        let mut ph = 0.0f32;
        for i in 0..n {
            let t = i as f32 / sr as f32;
            let ni = ((t / nd) as usize) % mel.len();
            let freq = root_hz * 2.0 * semitone_ratio(scale[mel[ni] % scale.len()]);
            let env = adsr(t % nd, nd * 0.85, 0.005, 0.05, 0.6, 0.1);
            mix[i] += (2.0 * PI * ph).sin() * env * 0.15;
            ph = (ph + freq / sr as f32).fract();
        }
    }

    // Drums
    if has_drums {
        for i in 0..n {
            let t = i as f32 / sr as f32;
            let beat_t = t % beat_dur;
            let beat_pos = (t / beat_dur) % 4.0;
            // Kick on 1 & 3
            if beat_pos < 0.05 || (beat_pos > 1.95 && beat_pos < 2.05) {
                mix[i] += (2.0 * PI * beat_t * 60.0 * (-beat_t * 20.0).exp()).sin() * (-beat_t * 8.0).exp() * 0.4;
            }
            // Snare on 2 & 4
            if (beat_pos > 0.95 && beat_pos < 1.05) || beat_pos > 2.95 {
                let noise = fastrand::f32() * 2.0 - 1.0;
                mix[i] += noise * (-beat_t * 18.0).exp() * 0.25;
            }
            // Hi-hat
            let ht = t % (beat_dur / 4.0);
            if ht < beat_dur / 32.0 {
                mix[i] += (fastrand::f32() * 2.0 - 1.0) * (-ht * 80.0 / beat_dur).exp() * 0.07;
            }
        }
    }

    // Limiter + fade
    let fade = (sr as f32 * 0.05) as usize;
    for i in 0..n {
        mix[i] = mix[i].clamp(-0.95, 0.95);
        if i < fade { mix[i] *= i as f32 / fade as f32; }
        if i > n.saturating_sub(fade) { mix[i] *= (n - i) as f32 / fade as f32; }
    }

    encode_wav_f32_inline(&mix, sr)
}

fn encode_wav_f32_inline(samples: &[f32], sr: u32) -> Vec<u8> {
    let data_len = (samples.len() * 4) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&3u16.to_le_bytes());  // IEEE float
    out.extend_from_slice(&1u16.to_le_bytes());  // mono
    out.extend_from_slice(&sr.to_le_bytes());
    out.extend_from_slice(&(sr * 4).to_le_bytes());
    out.extend_from_slice(&4u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for &s in samples { out.extend_from_slice(&s.to_le_bytes()); }
    out
}
