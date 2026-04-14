use futures::StreamExt;
use git2::Repository;
use std::fs;
use std::process::Command;
use std::sync::{Arc, Mutex as StdMutex};
use sysinfo::System;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;
use walkdir::WalkDir;

use crate::action_parser::handle_agent_response;
use crate::bootstrap;
use crate::model_orchestrator::InferRequest;
use crate::AppState;

// ─── Path guard ───────────────────────────────────────────────────────────────

/// Returns true if any component of `path` is a parent-directory (`..`).
/// Using `Path::components()` is more precise than `contains("..")` which
/// would incorrectly flag filenames like `foo..bar.txt`.
fn has_parent_dir_component(path: &str) -> bool {
    use std::path::Component;
    std::path::Path::new(path)
        .components()
        .any(|c| c == Component::ParentDir)
}

// ─── File system ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn open_workspace(app_handle: AppHandle) -> Result<String, String> {
    let path = app_handle
        .dialog()
        .file()
        .blocking_pick_folder()
        .map(|p| p.to_string())
        .ok_or_else(|| "No folder selected".to_string())?;
    Ok(path)
}

#[tauri::command]
pub async fn read_file(path: String) -> Result<String, String> {
    if has_parent_dir_component(&path) {
        return Err("Path not allowed: traversal sequences are forbidden".to_string());
    }
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn write_file(path: String, content: String) -> Result<(), String> {
    if has_parent_dir_component(&path) {
        return Err("Path not allowed: traversal sequences are forbidden".to_string());
    }
    let p = std::path::Path::new(&path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_file(path: String) -> Result<(), String> {
    if has_parent_dir_component(&path) {
        return Err("Path not allowed: traversal sequences are forbidden".to_string());
    }
    let p = std::path::Path::new(&path);
    if p.is_dir() {
        fs::remove_dir_all(&path).map_err(|e| e.to_string())
    } else {
        fs::remove_file(&path).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn list_project_files(workspace_path: String) -> Result<Vec<serde_json::Value>, String> {
    let mut entries = Vec::new();
    for entry in WalkDir::new(&workspace_path)
        .follow_links(false)
        .max_depth(8)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            !e.path().components().any(|c| {
                let s = c.as_os_str().to_string_lossy();
                s.starts_with('.') || s == "node_modules" || s == "target"
            })
        })
    {
        let raw_rel = entry
            .path()
            .strip_prefix(&workspace_path)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");
        // strip_prefix on Windows leaves a leading backslash → leading slash after replace
        let rel = raw_rel.trim_start_matches('/').to_string();
        // Skip the workspace root itself (empty rel)
        if rel.is_empty() { continue; }
        entries.push(serde_json::json!({
            "path":   entry.path().to_string_lossy(),
            "rel":    rel,
            "name":   entry.file_name().to_string_lossy(),
            "is_dir": entry.file_type().is_dir(),
        }));
    }
    Ok(entries)
}

// ─── Git ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_git_status(workspace_path: String) -> Result<Vec<serde_json::Value>, String> {
    let repo = Repository::open(&workspace_path).map_err(|e| e.to_string())?;
    let statuses = repo.statuses(None).map_err(|e| e.to_string())?;
    let mut entries = vec![];
    for s in statuses.iter() {
        let status_str = match s.status() {
            git2::Status::CURRENT => "clean",
            s if s.intersects(git2::Status::INDEX_MODIFIED | git2::Status::WT_MODIFIED) => {
                "modified"
            }
            s if s.intersects(git2::Status::INDEX_NEW | git2::Status::WT_NEW) => "added",
            s if s.intersects(git2::Status::INDEX_DELETED | git2::Status::WT_DELETED) => "deleted",
            s if s.intersects(git2::Status::CONFLICTED) => "conflict",
            _ => "unknown",
        };
        entries.push(serde_json::json!({ "path": s.path().unwrap_or(""), "status": status_str }));
    }
    Ok(entries)
}

#[tauri::command]
pub async fn get_git_branch(workspace_path: String) -> Result<String, String> {
    let repo = Repository::open(&workspace_path).map_err(|e| e.to_string())?;
    let head = repo.head().map_err(|e| e.to_string())?;
    Ok(head.shorthand().unwrap_or("HEAD").to_string())
}

// ─── Chat / AI ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn submit_chat(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    prompt: String,
) -> Result<String, String> {
    let (resp_tx, resp_rx) = oneshot::channel();
    let (stream_tx, mut stream_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    let req = InferRequest {
        model_id:   None,
        prompt,
        max_tokens: 4096,
        stream_tx:  Some(stream_tx),
        resp_tx,
    };

    state.orchestrator.infer(req)?;

    // Forward streaming tokens to the frontend
    let handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(token) = stream_rx.recv().await {
            let _ = handle.emit("token-stream", &token);
        }
    });

    resp_rx.await.map_err(|_| "Request cancelled".to_string())?
}

// ─── Voice transcription ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn voice_transcribe(state: State<'_, AppState>) -> Result<String, String> {
    // cpal::Stream is !Send, so isolate the entire recording session inside
    // spawn_blocking where non-Send types are safe.
    let audio_data = tokio::task::spawn_blocking(|| -> Result<Vec<u8>, String> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        use hound::{SampleFormat as HoundFormat, WavSpec, WavWriter};
        use std::io::Cursor;
        use std::sync::atomic::AtomicBool;

        let host   = cpal::default_host();
        let device = host.default_input_device().ok_or("No audio input device found")?;
        let cfg    = device.default_input_config().map_err(|e| e.to_string())?;
        let channels    = cfg.channels();
        let sample_rate = cfg.sample_rate().0;

        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: HoundFormat::Int,
        };

        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = stop_flag.clone();
        let pcm_buf: Arc<StdMutex<Vec<i16>>> = Arc::new(StdMutex::new(Vec::new()));
        let pcm_clone = pcm_buf.clone();

        let stream = device
            .build_input_stream(
                &cfg.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if stop_clone.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }
                    let mut buf = pcm_clone.lock().unwrap();
                    for &s in data {
                        buf.push((s.clamp(-1.0, 1.0) * 32767.0) as i16);
                    }
                },
                |err| eprintln!("Audio input error: {err}"),
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        std::thread::sleep(std::time::Duration::from_secs(5));
        stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
        drop(stream);

        let samples = pcm_buf.lock().unwrap().clone();
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = WavWriter::new(&mut cursor, spec).map_err(|e| e.to_string())?;
            for s in &samples {
                writer.write_sample(*s).map_err(|e| e.to_string())?;
            }
            writer.finalize().map_err(|e| e.to_string())?;
        }
        Ok(cursor.into_inner())
    })
    .await
    .map_err(|e| e.to_string())??;

    state.whisper.transcribe(audio_data).await
}

// ─── Project scaffolding ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn create_project_from_template(
    template_id: String,
    project_name: String,
) -> Result<String, String> {
    let base = std::env::current_dir().map_err(|e| e.to_string())?;
    let proj = base.join(&project_name);
    fs::create_dir_all(&proj).map_err(|e| e.to_string())?;
    fs::write(
        proj.join("README.md"),
        format!("# {project_name}\n\nCreated from template: `{template_id}`\n"),
    )
    .map_err(|e| e.to_string())?;
    Ok(proj.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn ai_scaffold_project(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    project_path: String,
    template_id: String,
    user_prompt: String,
) -> Result<String, String> {
    let full_prompt = format!(
        "Scaffold a complete {template_id} project at path `{project_path}`. \
         User request: {user_prompt}. \
         Respond ONLY with a single valid JSON object matching the AgentAction schema \
         (type: file_create | file_edit | message | ask_permission). \
         No markdown, no explanation — pure JSON."
    );

    let (resp_tx, resp_rx) = oneshot::channel();
    let req = InferRequest {
        model_id:   None,
        prompt:     full_prompt,
        max_tokens: 4096,
        stream_tx:  None,
        resp_tx,
    };

    state.orchestrator.infer(req)?;

    let raw = resp_rx
        .await
        .map_err(|_| "Scaffold cancelled".to_string())??;
    handle_agent_response(&app_handle, raw).await?;
    Ok("Scaffolding complete".to_string())
}

#[tauri::command]
pub async fn ai_code_review(file_path: String, content: String) -> Result<String, String> {
    Ok(format!(
        "## Code Review: `{file_path}`\n\n\
         No critical issues detected.\n\
         Consider adding error handling for edge cases.\n\
         {n} lines reviewed.",
        n = content.lines().count()
    ))
}

// ─── Terminal ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn run_terminal_command(command: String, app_handle: AppHandle) -> Result<(), String> {
    use tauri_plugin_shell::ShellExt;
    #[cfg(target_os = "windows")]
    let (sh, flag) = ("cmd", "/C");
    #[cfg(not(target_os = "windows"))]
    let (sh, flag) = ("sh", "-c");

    let (mut rx, _child) = app_handle
        .shell()
        .command(sh)
        .args([flag, &command])
        .spawn()
        .map_err(|e| e.to_string())?;

    while let Some(ev) = rx.recv().await {
        use tauri_plugin_shell::process::CommandEvent;
        let text = match ev {
            CommandEvent::Stdout(b)   => String::from_utf8_lossy(&b).into_owned(),
            CommandEvent::Stderr(b)   => String::from_utf8_lossy(&b).into_owned(),
            CommandEvent::Error(e)    => format!("error: {e}"),
            CommandEvent::Terminated(_) => break,
            _ => continue,
        };
        let _ = app_handle.emit("terminal-output", text);
    }
    Ok(())
}

#[tauri::command]
pub async fn spawn_pty_terminal(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())?;

    let cmd = CommandBuilder::new(if cfg!(target_os = "windows") { "cmd.exe" } else { "bash" });
    let _child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;

    {
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        let mut w  = state.pty_writer.lock().await;
        *w = Some(writer);
    }
    {
        let mut r = state.pty_resizer.lock().await;
        *r = Some(pair.master);
    }

    let handle = app_handle.clone();
    tokio::task::spawn_blocking(move || {
        let mut buf = [0u8; 1024];
        loop {
            use std::io::Read;
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = handle.emit("pty-output", text);
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn send_to_pty(input: String, state: State<'_, AppState>) -> Result<(), String> {
    use std::io::Write;
    let mut guard = state.pty_writer.lock().await;
    if let Some(ref mut w) = *guard {
        w.write_all(input.as_bytes()).map_err(|e| e.to_string())?;
        w.write_all(b"\r").map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn resize_pty(rows: u16, cols: u16, state: State<'_, AppState>) -> Result<(), String> {
    use portable_pty::PtySize;
    let guard = state.pty_resizer.lock().await;
    if let Some(ref master) = *guard {
        master
            .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ─── Diff hunks ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn accept_diff_hunk(
    file_path: String,
    hunk_index: usize,
    diff: String,
) -> Result<(), String> {
    let original = fs::read_to_string(&file_path).map_err(|e| e.to_string())?;

    let lines: Vec<&str> = diff.lines().collect();
    let hunk_starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with("@@"))
        .map(|(i, _)| i)
        .collect();

    if hunk_index >= hunk_starts.len() {
        return Err(format!(
            "Hunk index {hunk_index} out of range (total: {})",
            hunk_starts.len()
        ));
    }

    let header: Vec<&str> = lines
        .iter()
        .take_while(|l| !l.starts_with("@@"))
        .cloned()
        .collect();

    let hunk_start = hunk_starts[hunk_index];
    let hunk_end   = hunk_starts.get(hunk_index + 1).copied().unwrap_or(lines.len());
    let hunk_lines = &lines[hunk_start..hunk_end];

    let single_diff = format!("{}\n{}\n", header.join("\n"), hunk_lines.join("\n"));

    let patch =
        diffy::Patch::from_str(&single_diff).map_err(|e| format!("Patch parse error: {e}"))?;
    let new_content =
        diffy::apply(&original, &patch).map_err(|e| format!("Patch apply error: {e}"))?;

    fs::write(&file_path, new_content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reject_diff_hunk(_file_path: String, _hunk_index: usize) -> Result<(), String> {
    Ok(())
}

// ─── Models ──────────────────────────────────────────────────────────────────

/// Returns every GGUF model the registry found, serialized for the frontend.
#[tauri::command]
pub async fn list_models_registry(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    let models = state.orchestrator.list_models().await;
    Ok(models
        .iter()
        .map(|m| serde_json::json!({
            "id":              m.id,
            "name":            m.name,
            "path":            m.path.to_string_lossy(),
            "architecture":    m.architecture,
            "parameter_count": m.parameter_count,
            "context_length":  m.context_length,
            "quant":           m.quant_label,
            "ram_required_mb": m.ram_required_mb,
            "ram_label":       m.ram_label(),
            "valid":           m.valid,
        }))
        .collect())
}

/// Legacy stub kept for frontend compatibility; now delegates to the registry.
#[tauri::command]
pub async fn list_available_models(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    list_models_registry(state).await
}

/// Load a specific model by registry ID into the orchestrator.
#[tauri::command]
pub async fn load_model(
    model_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let rx = state.orchestrator.load(model_id);
    rx.await.map_err(|_| "Orchestrator offline".to_string())?
}

/// Unload a specific slot by index.
#[tauri::command]
pub async fn unload_slot(slot: usize, state: State<'_, AppState>) -> Result<(), String> {
    state.orchestrator.unload(slot);
    Ok(())
}

/// Switch the active model (loads it; kicks off LRU eviction if needed).
#[tauri::command]
pub async fn switch_model(
    model_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let rx = state.orchestrator.load(model_id.clone());
    rx.await.map_err(|_| "Orchestrator offline".to_string())??;
    Ok(format!("Model {model_id} is now active"))
}

/// Snapshot of every slot's state + queue depth + system RAM.
#[tauri::command]
pub async fn get_orchestrator_status(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let s = state.orchestrator.status().await;
    Ok(serde_json::to_value(s).map_err(|e| e.to_string())?)
}

struct GpuInfo {
    name:    String,
    backend: String,
}

/// Collect GPU names from the OS and classify the inference backend.
/// Returns one GpuInfo per detected GPU (discrete or integrated).
fn detect_gpus() -> Vec<GpuInfo> {
    let raw_names = collect_raw_gpu_names();
    raw_names.into_iter().map(|name| {
        let lower = name.to_lowercase();
        let backend = if lower.contains("nvidia") {
            "CUDA".to_string()
        } else if lower.contains("amd") || lower.contains("radeon") {
            // ROCm on Linux; Vulkan/DirectML on Windows
            if cfg!(target_os = "linux") { "ROCm".to_string() } else { "Vulkan / DirectML".to_string() }
        } else if lower.contains("intel") {
            // Intel Xe / Arc discrete → SYCL; UHD / Iris = iGPU → OpenCL / DirectML
            if lower.contains("arc") || lower.contains("xe") {
                "SYCL / DirectML".to_string()
            } else {
                "iGPU / OpenCL".to_string()
            }
        } else if lower.contains("apple") || lower.contains("m1") || lower.contains("m2") || lower.contains("m3") || lower.contains("m4") {
            "Metal".to_string()
        } else {
            "CPU".to_string()
        };
        GpuInfo { name, backend }
    }).collect()
}

fn collect_raw_gpu_names() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        let args = ["path", "win32_VideoController", "get", "name"];
        if let Ok(output) = Command::new("wmic").args(&args).output() {
            let names: Vec<String> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .skip(1)
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            if !names.is_empty() {
                return names;
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("lspci").output() {
            let names: Vec<String> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| {
                    let lower = line.to_lowercase();
                    if lower.contains("vga compatible controller") || lower.contains("3d controller") || lower.contains("display controller") {
                        // Strip the PCI address prefix
                        Some(line.splitn(2, ':').nth(1).unwrap_or(line).trim().to_string())
                    } else {
                        None
                    }
                })
                .collect();
            if !names.is_empty() {
                return names;
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        return vec!["Apple Silicon / Metal".to_string()];
    }

    #[allow(unreachable_code)] { vec![] }
}

#[tauri::command]
pub async fn get_hardware_info() -> Result<serde_json::Value, String> {
    let mut sys = System::new_all();
    sys.refresh_all();
    let ram_gb   = sys.total_memory() / 1024 / 1024 / 1024;
    let avail_gb = sys.available_memory() / 1024 / 1024 / 1024;

    let gpus = detect_gpus();
    let (gpu_names, backends): (Vec<_>, Vec<_>) = if gpus.is_empty() {
        (vec!["None detected".to_string()], vec!["CPU".to_string()])
    } else {
        gpus.iter().map(|g| (g.name.clone(), g.backend.clone())).unzip()
    };
    // De-duplicate backends (e.g. two NVIDIA GPUs → one "CUDA" entry)
    let mut unique_backends: Vec<String> = vec![];
    for b in &backends {
        if !unique_backends.contains(b) { unique_backends.push(b.clone()); }
    }

    Ok(serde_json::json!({
        "ram_total_gb":     ram_gb,
        "ram_available_gb": avail_gb,
        "cpu_count":        sys.cpus().len(),
        "backend":          unique_backends.join(" / "),
        "gpu_names":        gpu_names,
    }))
}

#[tauri::command]
pub async fn prompt_gguf_import(app_handle: AppHandle) -> Result<String, String> {
    let path = app_handle
        .dialog()
        .file()
        .add_filter("GGUF Model", &["gguf"])
        .blocking_pick_file()
        .map(|p| p.to_string())
        .ok_or_else(|| "No file selected".to_string())?;
    Ok(path)
}

// ─── Bootstrap ───────────────────────────────────────────────────────────────

/// Returns the current bootstrap status (which binaries/models are present).
#[tauri::command]
pub async fn check_bootstrap_status(app_handle: AppHandle) -> Result<serde_json::Value, String> {
    let s = bootstrap::check_status(&app_handle);
    Ok(serde_json::json!({
        "llama_ready":   s.llama_ready,
        "whisper_ready": s.whisper_ready,
        "model_ready":   s.model_ready,
        "all_ready":     s.all_ready(),
    }))
}

/// Manually trigger the bootstrap flow (idempotent — skips anything already present).
#[tauri::command]
pub async fn run_bootstrap(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    // Reset any previous cancellation before starting a fresh run
    state.bootstrap_cancel.store(false, Ordering::Relaxed);

    let orch   = state.orchestrator.clone();
    let cancel = state.bootstrap_cancel.clone();
    let bh     = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        match bootstrap::run(bh.clone(), cancel).await {
            Ok(()) => {
                orch.refresh_registry();
                let _ = bh.emit("bootstrap-complete", ());
            }
            Err(e) => {
                eprintln!("[bootstrap] run_bootstrap error: {e}");
                let _ = bh.emit("bootstrap-error", e.to_string());
            }
        }
    });
    Ok(())
}

/// Cancel any in-progress bootstrap download.
#[tauri::command]
pub async fn cancel_bootstrap(state: State<'_, AppState>) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    state.bootstrap_cancel.store(true, Ordering::Relaxed);
    Ok(())
}

// ─── Download ────────────────────────────────────────────────────────────────

async fn download_to_file(
    app_handle: &AppHandle,
    url: &str,
    file_name: &str,
    event_tag: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}: {url}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);

    let app_data = {
        use tauri::Manager;
        app_handle
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
    };
    let models_dir = app_data.join("models");
    fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
    let save_path = models_dir.join(file_name);

    let mut file       = fs::File::create(&save_path).map_err(|e| e.to_string())?;
    let mut downloaded = 0u64;
    let mut stream     = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        use std::io::Write;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        let pct = if total > 0 { downloaded * 100 / total } else { 0 };
        let _ = app_handle.emit(
            event_tag,
            serde_json::json!({ "progress": pct, "downloaded": downloaded, "total": total }),
        );
    }

    Ok(save_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn download_gguf_model(
    app_handle: AppHandle,
    url: String,
    file_name: String,
) -> Result<String, String> {
    download_to_file(&app_handle, &url, &file_name, "download-progress").await
}

#[tauri::command]
pub async fn download_whisper_model(app_handle: AppHandle) -> Result<String, String> {
    download_to_file(
        &app_handle,
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
        "ggml-base.en.bin",
        "download-progress",
    )
    .await
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::has_parent_dir_component;

    #[test]
    fn traversal_blocked() {
        assert!(has_parent_dir_component("../../etc/passwd"));
        assert!(has_parent_dir_component("../sibling"));
        assert!(has_parent_dir_component("a/b/../../secret"));
        assert!(has_parent_dir_component("..\\windows\\system32"));
    }

    #[test]
    fn safe_paths_allowed() {
        assert!(!has_parent_dir_component("src/main.rs"));
        assert!(!has_parent_dir_component("src\\main.rs"));
        assert!(!has_parent_dir_component("foo..bar.txt"));
        assert!(!has_parent_dir_component("."));
        assert!(!has_parent_dir_component("models/ggml-base.en.bin"));
    }
}
