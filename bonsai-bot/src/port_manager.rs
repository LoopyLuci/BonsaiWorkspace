use std::process::Command;
use std::time::Duration;
use std::path::PathBuf;
use std::fs::{self, OpenOptions};
use std::io::Write;
use fs2::FileExt;
use tempfile::NamedTempFile;
use tokio::net::TcpListener;
use tokio::time::sleep;
use serde_json::json;
use sha2::Digest;

pub async fn is_api_healthy(host: &str, port: u16) -> bool {
    let url = format!("http://{host}:{port}/health");
    match reqwest::Client::builder()
        .timeout(Duration::from_millis(1200))
        .build()
    {
        Ok(client) => client
            .get(url)
            .send()
            .await
            .is_ok_and(|r| r.status().is_success()),
        Err(_) => false,
    }
}

/// Attempt to allocate and bind a TcpListener near the preferred port.
/// Tries preferred_port .. preferred_port+max_delta. On Windows it will
/// attempt a best-effort reclaim of stale listeners that appear to belong
/// to a known bonsai-bot process image.
pub async fn allocate_listener(preferred_port: u16, max_delta: u16) -> Result<(u16, TcpListener), String> {
    let mut bound = None;
    for delta in 0u16..=max_delta {
        let p = preferred_port.saturating_add(delta);
            match TcpListener::bind(format!("127.0.0.1:{p}")).await {
            Ok(l) => {
                // If we asked for port 0 (ephemeral) the OS assigns a port; read it back.
                let actual_port = if p == 0 {
                    l.local_addr().map(|addr| addr.port()).unwrap_or(p)
                } else { p };
                bound = Some((actual_port, l));
                break;
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::AddrInUse {
                    // If a healthy API is already bound here, skip this port.
                    if is_api_healthy("127.0.0.1", p).await {
                        tracing::info!("[port-manager] Port {p} in use by healthy API; skipping");
                        continue;
                    }

                    // Try to reclaim stale listeners on Windows by killing matching processes.
                    if try_reclaim_stale_listener(p) {
                        // give the OS a moment to release the socket
                        sleep(Duration::from_millis(300)).await;
                        if let Ok(l2) = TcpListener::bind(format!("127.0.0.1:{p}")).await {
                            bound = Some((p, l2));
                            break;
                        }
                    }
                    // otherwise continue to next candidate
                    continue;
                }
            }
        }
    }

    let (port, listener) = bound.ok_or_else(|| format!("no admin port available near {}", preferred_port))?;
    Ok((port, listener))
}

/// Persist chosen port and metadata atomically with an exclusive lock.
/// Writes `bonsai-bot-port.json` in config dir `{config_dir}/bonsai` if available,
/// otherwise writes to the current working directory.
pub fn persist_port(port: u16, admin_token: &str) -> Result<(), String> {
    // Prepare JSON payload
    let pid = std::process::id();
    let started_at = chrono::Utc::now().to_rfc3339();
    let digest = sha2::Sha256::digest(admin_token.as_bytes());
    let token_hash = format!("sha256:{}", hex::encode(digest));
    let payload = json!({
        "port": port,
        "pid": pid,
        "started_at": started_at,
        "token_hash": token_hash,
    });
    let s = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;

    // Determine target path
    let target_dir: PathBuf = match crate::config::config_dir() {
        Some(d) => d.join("bonsai"),
        None => PathBuf::from("."),
    };
    if let Err(e) = fs::create_dir_all(&target_dir) {
        return Err(format!("failed to create config dir: {e}"));
    }
    let target_path = target_dir.join("bonsai-bot-port.json");
    let lock_path = target_dir.join("bonsai-bot-port.lock");

    // Create/open lock file and acquire exclusive lock
    let lock_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&lock_path)
        .map_err(|e| format!("failed to open lock file: {e}"))?;
    lock_file.lock_exclusive().map_err(|e| format!("failed to lock: {e}"))?;

    // Write to a temp file in same dir and then persist atomically
    let mut tmp = NamedTempFile::new_in(&target_dir).map_err(|e| format!("tmpfile failed: {e}"))?;
    std::io::Write::write_all(&mut tmp, s.as_bytes()).map_err(|e| format!("write tmp failed: {e}"))?;
    tmp.flush().map_err(|e| format!("flush tmp failed: {e}"))?;
    tmp.persist(&target_path).map_err(|e| format!("persist failed: {e}"))?;

    // Drop lock by closing file (will unlock on drop)
    drop(lock_file);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn try_reclaim_stale_listener(_port: u16) -> bool { false }

#[cfg(target_os = "windows")]
fn try_reclaim_stale_listener(port: u16) -> bool {
    let pids = listening_pids_on_port(port);
    if pids.is_empty() {
        return false;
    }

    let mut killed_any = false;
    for pid in pids {
        let image = process_image_name(pid);
        let img = image.to_ascii_lowercase();
        if img != "bonsai-bot.exe" && img != "bonsai-bot" {
            continue;
        }
        if let Ok(out) = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output()
        {
            if out.status.success() {
                killed_any = true;
            }
        }
    }
    killed_any
}

#[cfg(target_os = "windows")]
fn listening_pids_on_port(port: u16) -> Vec<u32> {
    let out = match Command::new("netstat").args(["-ano"]).output() {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    let dump = String::from_utf8_lossy(&out.stdout);
    let mut pids = std::collections::BTreeSet::new();
    let needle = format!(":{port}");

    for line in dump.lines() {
        let l = line.trim();
        if l.is_empty() || !l.contains(&needle) || !l.to_ascii_uppercase().contains("LISTEN") {
            continue;
        }
        let parts: Vec<&str> = l.split_whitespace().collect();
        if let Some(last) = parts.last() {
            if let Ok(pid) = last.parse::<u32>() {
                if pid > 0 {
                    pids.insert(pid);
                }
            }
        }
    }

    pids.into_iter().collect()
}

#[cfg(target_os = "windows")]
fn process_image_name(pid: u32) -> String {
    let out = match Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return String::new(),
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.trim();
    if line.is_empty() || line.contains("No tasks are running") {
        return String::new();
    }
    line.split(',')
        .next()
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn allocate_ephemeral_port() {
        let (port, _listener) = allocate_listener(0, 0)
            .await
            .expect("allocate_listener should bind ephemeral port");
        assert!(port > 0, "allocated port should be non-zero");
    }
}
