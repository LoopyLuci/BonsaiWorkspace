/// Repair engine — deterministic rule-based fixes + AI fallback.

use std::time::Duration;
use anyhow::Result;
use tracing::{debug, info, warn};

use crate::kb::KnowledgeBase;

// ── Deterministic launch-time repairs ────────────────────────────────────────

/// Try hard-coded fixes for the most common pre-launch failures.
/// Returns true if a fix was applied (caller should retry the launch).
pub fn attempt_launch_repair(error: &str) -> bool {
    info!("[repair] attempting launch repair for: {error}");

    // Port already in use (Tauri default 1420, daemon 11369, etc.)
    if error.contains("EADDRINUSE")
        || error.contains("address already in use")
        || error.contains("Failed to bind")
    {
        for port in [1420u16, 11369, 11420, 11421, 11380] {
            let freed = free_port(port);
            if freed {
                info!("[repair] freed port {port}");
                return true;
            }
        }
    }

    // Missing npm/node_modules
    if error.contains("Cannot find module") || error.contains("MODULE_NOT_FOUND") {
        info!("[repair] running npm install");
        return run_script("npm install --prefix bonsai-workspace");
    }

    // Corrupted bonsai config
    if error.contains("toml parse error") || error.contains("TOML parse error") {
        info!("[repair] resetting bonsai config");
        let cfg = dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/bonsai-config.json");
        if cfg.exists() {
            let backup = cfg.with_extension("json.bak");
            let _ = std::fs::copy(&cfg, &backup);
            let _ = std::fs::remove_file(&cfg);
            return true;
        }
    }

    // Missing shared library (Linux)
    if error.contains("error while loading shared libraries") {
        warn!("[repair] shared library missing — manual intervention may be needed");
    }

    false
}

/// Kill any process listening on `port`. Returns true if something was killed.
fn free_port(port: u16) -> bool {
    let script = if cfg!(target_os = "windows") {
        format!(
            "for /f \"tokens=5\" %a in ('netstat -ano ^| findstr :{port}') do taskkill /F /PID %a 2>nul"
        )
    } else {
        format!("lsof -ti:{port} 2>/dev/null | xargs -r kill -9")
    };
    run_script(&script)
}

/// Execute a shell command. Returns true if it exits with code 0.
pub fn run_script(script: &str) -> bool {
    debug!("[repair] executing: {script}");
    let result = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", script])
            .output()
    } else {
        std::process::Command::new("sh")
            .args(["-c", script])
            .output()
    };
    match result {
        Ok(out) => {
            debug!(
                "[repair] exit={} stdout={}",
                out.status,
                String::from_utf8_lossy(&out.stdout).trim()
            );
            out.status.success()
        }
        Err(e) => {
            warn!("[repair] script failed to run: {e}");
            false
        }
    }
}

// ── Rule-based + AI repair ────────────────────────────────────────────────────

/// Try KB rules then AI. Returns the fix entry id if one succeeded.
pub async fn attempt_repair(kb: &KnowledgeBase, logs: &str) -> Option<i64> {
    // 1. Rule-based
    let fixes = kb.find_matching(logs);
    for fix in &fixes {
        info!("[repair] trying rule #{} pattern='{}...'", fix.id, &fix.error_pattern[..fix.error_pattern.len().min(40)]);
        if run_script(&fix.solution_script) {
            info!("[repair] rule #{} succeeded", fix.id);
            return Some(fix.id);
        }
        warn!("[repair] rule #{} did not succeed", fix.id);
    }

    // 2. AI fallback
    info!("[repair] no rule matched — querying AI");
    match ai_diagnose_and_fix(logs).await {
        Some((script, _explanation)) => {
            info!("[repair] AI suggested: {script}");
            // Validate: refuse obviously dangerous commands
            if is_dangerous(&script) {
                warn!("[repair] AI script rejected (dangerous)");
                return None;
            }
            if run_script(&script) {
                let id = kb
                    .insert_fix(logs, "ai", &script, 0.7, "bonsai")
                    .unwrap_or(-1);
                return Some(id);
            }
        }
        None => warn!("[repair] AI produced no fix"),
    }
    None
}

/// Very basic safety filter for AI-generated scripts.
fn is_dangerous(script: &str) -> bool {
    let forbidden = ["rm -rf /", "mkfs", "dd if=", ":(){ :|:& };:", "format c:"];
    forbidden.iter().any(|f| script.contains(f))
}

// ── AI integration ────────────────────────────────────────────────────────────

const AI_SYSTEM_PROMPT: &str = "\
You are an expert system administrator specializing in the Bonsai AI application. \
Given the following error log, determine the root cause and output a single shell \
command (no markdown, no explanation) that will fix the problem. \
If you cannot determine a safe fix, output NOT_FIXABLE.";

/// Call the local BonsAI model server for a repair suggestion.
/// Returns (script, explanation) if the model produced an actionable fix.
pub async fn ai_diagnose_and_fix(logs: &str) -> Option<(String, String)> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .ok()?;

    let payload = serde_json::json!({
        "model": "bonsai",
        "messages": [
            { "role": "system", "content": AI_SYSTEM_PROMPT },
            { "role": "user",   "content": &logs[..logs.len().min(4000)] }
        ],
        "max_tokens": 200,
        "temperature": 0.05
    });

    // Try the Buddy API first (port 11420), then llama-server (8080)
    for port in [11420u16, 8080] {
        let url = format!("http://127.0.0.1:{port}/v1/chat/completions");
        if let Ok(resp) = client.post(&url).json(&payload).send().await {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let content = json["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !content.is_empty() && content != "NOT_FIXABLE" {
                    return Some((content.clone(), content));
                }
            }
        }
    }
    None
}
