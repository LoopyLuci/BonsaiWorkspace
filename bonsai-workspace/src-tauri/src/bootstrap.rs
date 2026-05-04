//! First-run bootstrap.
//!
//! On first launch Bonsai automatically:
//!   1. Downloads the llama-server binary from the latest llama.cpp GitHub release.
//!   2. Downloads the whisper-server binary from the latest whisper.cpp GitHub release.
//!   3. Downloads the Whisper base.en model (~148 MB).
//!   4. Downloads the Bonsai-1.7B GGUF model from HuggingFace (~1.1 GB).
//!
//! Everything is stored in `{app_data}/sidecars/` and `{app_data}/models/`.
//! On subsequent launches the check is instant (just `Path::exists()`).
//!
//! # Cancellation
//! Pass an `Arc<AtomicBool>` as `cancel`; set it to `true` from any thread/task
//! to abort cleanly between download steps.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::{Context, Result};
use futures::StreamExt;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};
use sha2::{Digest, Sha256};

// ── Model sources ─────────────────────────────────────────────────────────────

const BONSAI_HF_REPO: &str = "prism-ml/Bonsai-1.7B-gguf";
/// Quantization preference order (first match wins).
const BONSAI_QUANT_PREF: &[&str] = &["q4_k_m", "q5_k_m", "q4_k", "q4_0", "q8_0"];

const WHISPER_MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin";
const WHISPER_MODEL_FILE: &str = "ggml-base.en.bin";

// ── Sidecar sources ───────────────────────────────────────────────────────────

const LLAMA_API: &str =
    "https://api.github.com/repos/ggerganov/llama.cpp/releases/latest";
const WHISPER_API: &str =
    "https://api.github.com/repos/ggerganov/whisper.cpp/releases/latest";

// ── Checksum verification ─────────────────────────────────────────────────────

/// Compute SHA-256 hash of a file and return as lowercase hex string.
async fn compute_sha256(path: &Path) -> Result<String> {
    let data = tokio::fs::read(path).await?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Load checksums from checksums.json. Returns empty map if file not found.
async fn load_checksums() -> Result<std::collections::HashMap<String, String>> {
    // Try to load from the assets bundled with the binary
    if let Ok(data) = std::fs::read_to_string("checksums.json") {
        if let Ok(map) = serde_json::from_str::<serde_json::Value>(&data) {
            let mut checksums: std::collections::HashMap<String, String> = std::collections::HashMap::new();
            if let Some(obj) = map.as_object() {
                for (key, val) in obj {
                    if let Some(hash) = val.get("latest").and_then(|v| v.as_str()) {
                        checksums.insert(key.clone(), hash.to_string());
                    }
                }
            }
            return Ok(checksums);
        }
    }
    // If checksums.json not found, return empty map (verification skipped)
    Ok(std::collections::HashMap::new())
}

// ── Status ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone, Debug)]
pub struct BootstrapStatus {
    pub llama_ready:   bool,
    pub whisper_ready: bool,
    pub model_ready:   bool,
}

impl BootstrapStatus {
    pub fn all_ready(&self) -> bool {
        self.llama_ready && self.whisper_ready && self.model_ready
    }
}

// ── Canonical paths ───────────────────────────────────────────────────────────

pub fn sidecars_dir(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().expect("app_data_dir").join("sidecars")
}

pub fn models_dir(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().expect("app_data_dir").join("models")
}

pub fn llama_exe(app: &AppHandle) -> PathBuf {
    sidecar_exe(app, "llama-server")
}

pub fn whisper_exe(app: &AppHandle) -> PathBuf {
    sidecar_exe(app, "whisper-server")
}

fn sidecar_exe(app: &AppHandle, name: &str) -> PathBuf {
    #[cfg(windows)]
    return sidecars_dir(app).join(format!("{}.exe", name));
    #[cfg(not(windows))]
    return sidecars_dir(app).join(name);
}

pub fn whisper_model(app: &AppHandle) -> PathBuf {
    models_dir(app).join(WHISPER_MODEL_FILE)
}

/// Returns the path of the first GGUF file found, preferring Bonsai-1.7B.
pub fn find_gguf(app: &AppHandle) -> Option<PathBuf> {
    let dir = models_dir(app);
    if let Ok(rd) = std::fs::read_dir(&dir) {
        let mut fallback: Option<PathBuf> = None;
        for entry in rd.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("gguf") {
                continue;
            }
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            if stem.contains("bonsai") {
                return Some(p);
            }
            fallback.get_or_insert(p);
        }
        return fallback;
    }
    None
}

pub fn check_status(app: &AppHandle) -> BootstrapStatus {
    BootstrapStatus {
        llama_ready:   llama_exe(app).exists(),
        whisper_ready: whisper_exe(app).exists(),
        model_ready:   find_gguf(app).is_some(),
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Run the full bootstrap sequence, honouring a cancellation flag.
///
/// Each step is skipped if its output already exists. Steps emit
/// `bootstrap-progress` events so the frontend can show progress bars.
pub async fn run(app: AppHandle, cancel: Arc<AtomicBool>) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent("bonsai-workspace/0.1.0")
        .timeout(std::time::Duration::from_secs(3600))
        .build()?;

    tokio::fs::create_dir_all(sidecars_dir(&app)).await?;
    tokio::fs::create_dir_all(models_dir(&app)).await?;

    // 1. llama-server
    if !llama_exe(&app).exists() {
        check_cancel(&cancel, "Bootstrap cancelled before llama-server download")?;
        step(&app, "llama", 0, "Locating llama.cpp release…");
        let url = github_zip_url(&client, LLAMA_API)
            .await
            .context("Could not find a llama.cpp release for this platform")?;
        download_and_extract(&client, &url, &sidecars_dir(&app), &app, "llama", &cancel)
            .await
            .context("llama.cpp download failed")?;
        normalise_exe(&sidecars_dir(&app), "llama-server")?;
    }

    // 2. whisper-server
    if !whisper_exe(&app).exists() {
        check_cancel(&cancel, "Bootstrap cancelled before whisper-server download")?;
        step(&app, "whisper", 0, "Locating whisper.cpp release…");
        let url = github_zip_url(&client, WHISPER_API)
            .await
            .context("Could not find a whisper.cpp release for this platform")?;
        download_and_extract(&client, &url, &sidecars_dir(&app), &app, "whisper", &cancel)
            .await
            .context("whisper.cpp download failed")?;
        normalise_exe(&sidecars_dir(&app), "whisper-server")?;
    }

    // 3. Whisper model
    if !whisper_model(&app).exists() {
        check_cancel(&cancel, "Bootstrap cancelled before Whisper model download")?;
        step(&app, "whisper_model", 0, "Downloading Whisper base.en (148 MB)…");
        stream_file(&client, WHISPER_MODEL_URL, &whisper_model(&app), &app, "whisper_model", &cancel)
            .await
            .context("Whisper model download failed")?;
    }

    // 4. Bonsai-1.7B
    if find_gguf(&app).is_none() {
        check_cancel(&cancel, "Bootstrap cancelled before Bonsai model download")?;
        step(&app, "bonsai_model", 0, "Locating Bonsai-1.7B on HuggingFace…");
        let (url, filename) = hf_gguf_url(&client, BONSAI_HF_REPO, BONSAI_QUANT_PREF)
            .await
            .context("Could not locate Bonsai-1.7B GGUF on HuggingFace")?;
        let dest = models_dir(&app).join(&filename);
        step(&app, "bonsai_model", 1, &format!("Downloading {} …", filename));
        stream_file(&client, &url, &dest, &app, "bonsai_model", &cancel)
            .await
            .context("Bonsai model download failed")?;
    }

    let _ = app.emit("bootstrap-complete", ());
    Ok(())
}

// ── Cancellation helper ───────────────────────────────────────────────────────

fn check_cancel(cancel: &AtomicBool, msg: &str) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(anyhow::anyhow!("{}", msg))
    } else {
        Ok(())
    }
}

// ── HuggingFace GGUF discovery ────────────────────────────────────────────────

async fn hf_gguf_url(
    client: &reqwest::Client,
    repo: &str,
    pref: &[&str],
) -> Result<(String, String)> {
    let api = format!("https://huggingface.co/api/models/{}", repo);
    let info: serde_json::Value = client.get(&api).send().await?.json().await?;

    let siblings = info["siblings"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("HuggingFace repo {} has no siblings", repo))?;

    let gguf_files: Vec<&str> = siblings
        .iter()
        .filter_map(|s| s["rfilename"].as_str())
        .filter(|f| f.ends_with(".gguf"))
        .collect();

    if gguf_files.is_empty() {
        return Err(anyhow::anyhow!("No GGUF files found in {}", repo));
    }

    let best = pref
        .iter()
        .find_map(|p| gguf_files.iter().find(|f| f.to_lowercase().contains(p)))
        .or_else(|| gguf_files.first())
        .copied()
        .ok_or_else(|| anyhow::anyhow!("No suitable GGUF in {}", repo))?;

    let url = format!("https://huggingface.co/{}/resolve/main/{}", repo, best);
    Ok((url, best.to_string()))
}

// ── GitHub release asset discovery ───────────────────────────────────────────

/// Returns (preferred_patterns, fallback_patterns) for the current platform.
///
/// Preferred patterns are arch-specific; fallback patterns match any arch on
/// the same OS. `github_zip_url` tries preferred first, then falls back,
/// so repos that publish only a universal or x64 zip are still found.
///
/// # Platform coverage
///
/// | Platform        | Preferred           | Fallback       |
/// |-----------------|---------------------|----------------|
/// | Windows x64     | `["win", "x64"]`    | `["win"]`      |
/// | macOS arm64     | `["macos", "arm64"]`| `["macos"]`    |
/// | macOS x86_64    | `["macos", "x64"]`  | `["macos"]`    |
/// | Linux x86_64    | `["ubuntu", "x64"]` | `["linux","x64"]` |
/// | Linux arm64     | `["ubuntu", "arm64"]`| `["linux"]`   |
///
/// # Testing note — Linux ARM64
/// The Linux arm64 branch has **not** been validated on physical hardware or
/// QEMU emulation. llama.cpp and whisper.cpp do publish `ubuntu-arm64` zips,
/// but naming conventions may change between releases. Before shipping a Linux
/// ARM64 build, validate the download step against the actual GitHub release
/// asset list (e.g. via `curl https://api.github.com/repos/ggerganov/llama.cpp/releases/latest | jq '[.assets[].name]'`)
/// and adjust the patterns here if needed.
fn platform_patterns() -> (&'static [&'static str], &'static [&'static str]) {
    // Windows: only x86_64 releases are common
    if cfg!(target_os = "windows") {
        return (&["win", "x64"], &["win"]);
    }
    // macOS: prefer arch-specific, fall back to any macos zip
    if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        return (&["macos", "arm64"], &["macos"]);
    }
    if cfg!(target_os = "macos") {
        return (&["macos", "x64"], &["macos"]);
    }
    // Linux arm64 — see testing note above
    if cfg!(target_arch = "aarch64") {
        return (&["ubuntu", "arm64"], &["linux"]);
    }
    // Linux x86_64 (most common; llama.cpp names these "ubuntu-x64")
    (&["ubuntu", "x64"], &["linux", "x64"])
}

async fn github_zip_url(client: &reqwest::Client, api: &str) -> Result<String> {
    let release: serde_json::Value = client.get(api).send().await?.json().await?;
    let assets = release["assets"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("release has no assets"))?;

    let (preferred, fallback) = platform_patterns();

    // First attempt: arch-specific
    if let Some(url) = pick_zip(assets, preferred) {
        return Ok(url);
    }
    // Second attempt: OS-only fallback
    if let Some(url) = pick_zip(assets, fallback) {
        return Ok(url);
    }

    Err(anyhow::anyhow!(
        "No suitable zip found (tried {:?} then {:?})",
        preferred,
        fallback,
    ))
}

/// Detect whether a Vulkan-capable GPU is available on this machine.
/// Returns true if wmic/lspci finds AMD, NVIDIA, or Intel Arc.
fn has_vulkan_gpu() -> bool {
    #[cfg(target_os = "windows")]
    {
        if let Ok(out) = {
            let mut c = std::process::Command::new("wmic");
            c.args(["path", "win32_VideoController", "get", "name"]);
            #[cfg(windows)] { use std::os::windows::process::CommandExt; c.creation_flags(0x0800_0000); }
            c.output()
        } {
            let s = String::from_utf8_lossy(&out.stdout).to_lowercase();
            return s.contains("nvidia") || s.contains("amd") || s.contains("radeon")
                || s.contains("intel arc") || s.contains("intel xe");
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(out) = std::process::Command::new("lspci").output() {
            let s = String::from_utf8_lossy(&out.stdout).to_lowercase();
            return s.contains("nvidia") || s.contains("amd") || s.contains("radeon");
        }
    }
    false
}

/// Pick the best zip from `assets` whose filename contains ALL `must` tokens.
///
/// Strategy:
///   - If a Vulkan-capable GPU is detected on Windows, prefer the Vulkan build first.
///   - Otherwise prefer: noavx → avx2 → avx → cpu (pure CPU).
///   - CUDA and Metal are excluded (require separate driver installs).
pub(crate) fn pick_zip(assets: &[serde_json::Value], must: &[&str]) -> Option<String> {
    if must.is_empty() { return None; }

    let use_vulkan = cfg!(target_os = "windows") && has_vulkan_gpu();

    // On Windows with a GPU: try vulkan build first
    if use_vulkan {
        for asset in assets {
            let name = asset["name"].as_str().unwrap_or("").to_lowercase();
            if !name.ends_with(".zip") { continue; }
            if !must.iter().all(|p| name.contains(p)) { continue; }
            if name.contains("vulkan") {
                if let Some(url) = asset["browser_download_url"].as_str() {
                tracing::info!(name=%name, "[bootstrap] Using Vulkan llama.cpp build");
                    return Some(url.to_string());
                }
            }
        }
    }

    // CPU fallback: noavx → avx2 → avx → cpu
    let cpu_pref = ["noavx", "avx2", "avx", "cpu"];
    let mut best: Option<(&serde_json::Value, usize)> = None;

    for asset in assets {
        let name = asset["name"].as_str().unwrap_or("").to_lowercase();
        if !name.ends_with(".zip") { continue; }
        if !must.iter().all(|p| name.contains(p)) { continue; }
        // Skip GPU-specific non-Vulkan builds
        if name.contains("cuda") || name.contains("metal") || name.contains("hip") { continue; }
        if name.contains("vulkan") { continue; } // already tried above
        let score = cpu_pref.iter().position(|p| name.contains(p)).unwrap_or(cpu_pref.len());
        if best.is_none() || score < best.unwrap().1 {
            best = Some((asset, score));
        }
    }

    best.and_then(|(a, _)| a["browser_download_url"].as_str())
        .map(|s| s.to_string())
}

// ── Zip download + extraction ─────────────────────────────────────────────────

async fn download_and_extract(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    app: &AppHandle,
    tag: &str,
    cancel: &AtomicBool,
) -> Result<()> {
    let resp = client.get(url).send().await?;
    let total = resp.content_length().unwrap_or(1);
    let mut buf: Vec<u8> = Vec::with_capacity(total.min(128 * 1024 * 1024) as usize);
    let mut stream = resp.bytes_stream();
    let mut done = 0u64;

    while let Some(chunk) = stream.next().await {
        check_cancel(cancel, "Download cancelled")?;
        let chunk = chunk?;
        done += chunk.len() as u64;
        buf.extend_from_slice(&chunk);
        step(app, tag, (done * 85 / total.max(1)) as u8, "Downloading…");
    }

    // Verify SHA-256 if checksums are available
    {
        let checksums = load_checksums().await.unwrap_or_default();
        if let Some(expected_hash) = checksums.get(tag) {
            step(app, tag, 86, "Verifying SHA-256…");
            let mut hasher = Sha256::new();
            hasher.update(&buf);
            let actual_hash = format!("{:x}", hasher.finalize());
            
            if actual_hash != *expected_hash {
                return Err(anyhow::anyhow!(
                    "SHA-256 mismatch for {}: expected {}, got {}",
                    tag, expected_hash, actual_hash
                ));
            }
        }
    }

    step(app, tag, 87, "Extracting…");
    let dest = dest.to_path_buf();
    tokio::task::spawn_blocking(move || extract(&buf, &dest)).await??;
    step(app, tag, 100, "Ready");
    Ok(())
}

/// Returns `Some(true)` if `bytes` is a 64-bit PE (x86-64), `Some(false)` if it
/// is a PE of a different architecture, or `None` if the bytes are not a PE file.
fn pe_is_x64(bytes: &[u8]) -> Option<bool> {
    if bytes.len() < 64 { return None; }
    if bytes[0] != 0x4D || bytes[1] != 0x5A { return None; } // "MZ"
    let pe_off = u32::from_le_bytes(bytes[60..64].try_into().ok()?) as usize;
    if bytes.len() < pe_off + 6 { return None; }
    if &bytes[pe_off..pe_off + 4] != b"PE\0\0" { return None; }
    let machine = u16::from_le_bytes(bytes[pe_off + 4..pe_off + 6].try_into().ok()?);
    Some(machine == 0x8664) // AMD64 / x86-64
}

fn extract(data: &[u8], dest: &Path) -> Result<()> {
    use std::io::{Cursor, Read};
    let mut archive = zip::ZipArchive::new(Cursor::new(data))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() { continue; }

        let raw = entry.name().to_string();

        // ── Path traversal guard ───────────────────────────────────────────────
        // `Path::file_name()` strips all directory components, making traversal
        // via "../" or absolute paths impossible. We additionally reject any raw
        // entry name that contains separators after the strip as a belt-and-braces
        // defence against exotic implementations.
        let fname = match Path::new(&raw).file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None    => { tracing::warn!(entry=%raw, "[zip] Skipping entry with no filename"); continue; }
        };
        if fname.is_empty() || fname.contains("..") || fname.contains('/') || fname.contains('\\') {
            tracing::warn!(entry=%raw, "[zip] Skipping suspicious entry");
            continue;
        }

        let ext = Path::new(&fname)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Keep executables, shared libraries, and files named *server*/*llama*/*ggml*/*whisper*
        let keep = matches!(ext.as_str(), "exe" | "dll" | "so" | "dylib")
            || fname.contains("server")
            || fname.contains("llama")
            || fname.contains("ggml")
            || fname.contains("whisper");
        if !keep { continue; }

        // Read all entry bytes up-front so we can inspect the PE header before
        // writing. This lets us reject 32-bit binaries from mixed-arch zips.
        let mut content = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut content)?;

        // On 64-bit Windows, skip any 32-bit PE (machine type 0x014C). Some
        // llama.cpp release zips bundle both x86 and x64 variants with the same
        // filename; without this guard the 32-bit file can overwrite the 64-bit
        // one and cause 0xC000007B "Bad Image" crashes at runtime.
        #[cfg(all(windows, target_arch = "x86_64"))]
        if matches!(ext.as_str(), "exe" | "dll") {
            if let Some(false) = pe_is_x64(&content) {
                tracing::warn!(fname=%fname, "[zip] Skipping 32-bit binary (need x64)");
                continue;
            }
        }

        let out = dest.join(&fname);
        std::fs::write(&out, &content)?;

        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            if ext.is_empty() || ext == "so" || ext == "dylib" {
                std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o755))?;
            }
        }
    }
    Ok(())
}

/// Ensure the binary has the canonical name after extraction.
/// e.g. whisper.cpp releases the server as `server.exe`; rename it.
fn normalise_exe(dir: &Path, canonical: &str) -> Result<()> {
    #[cfg(windows)]
    let target = dir.join(format!("{}.exe", canonical));
    #[cfg(not(windows))]
    let target = dir.join(canonical);

    if target.exists() { return Ok(()); }

    #[cfg(windows)]
    let alts = ["server.exe", "main.exe"];
    #[cfg(not(windows))]
    let alts = ["server", "main"];

    for alt in alts {
        let src = dir.join(alt);
        if src.exists() {
            std::fs::rename(&src, &target)?;
            return Ok(());
        }
    }
    // Not found — not fatal, the orchestrator handles absent binaries gracefully.
    Ok(())
}

// ── Plain file streaming ──────────────────────────────────────────────────────

async fn stream_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    app: &AppHandle,
    tag: &str,
    cancel: &AtomicBool,
) -> Result<()> {
    // TODO(future): support download resumption via the HTTP `Range` header.
    // If `dest` already exists and is partially written (e.g. from a previous
    // interrupted run), send `Range: bytes=<existing_size>-` and append rather
    // than overwrite. This avoids re-downloading gigabyte models after a crash
    // or cancellation. Requires the server to advertise `Accept-Ranges: bytes`.
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("HTTP {} for {}", resp.status(), url));
    }
    let total = resp.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(dest).await?;
    let mut stream = resp.bytes_stream();
    let mut done = 0u64;
    let mut hasher = Sha256::new();
    use tokio::io::AsyncWriteExt;
    
    while let Some(chunk) = stream.next().await {
        check_cancel(cancel, "Download cancelled")?;
        let chunk = chunk?;
        hasher.update(&chunk);
        done += chunk.len() as u64;
        file.write_all(&chunk).await?;
        if total > 0 {
            step(app, tag, (done * 90 / total) as u8, "Downloading…");
        }
    }
    file.flush().await?;

    // Verify SHA-256 if checksums are available
    {
        let checksums = load_checksums().await.unwrap_or_default();
        if let Some(expected_hash) = checksums.get(tag) {
            step(app, tag, 95, "Verifying SHA-256…");
            let actual_hash = format!("{:x}", hasher.finalize());
            
            if actual_hash != *expected_hash {
                // Delete the file on mismatch
                let _ = tokio::fs::remove_file(dest).await;
                return Err(anyhow::anyhow!(
                    "SHA-256 mismatch for {}: expected {}, got {}",
                    tag, expected_hash, actual_hash
                ));
            }
        }
    }

    Ok(())
}

// ── Progress helper ───────────────────────────────────────────────────────────

fn step(app: &AppHandle, tag: &str, pct: u8, msg: &str) {
    let _ = app.emit(
        "bootstrap-progress",
        serde_json::json!({ "step": tag, "pct": pct, "msg": msg }),
    );
}
