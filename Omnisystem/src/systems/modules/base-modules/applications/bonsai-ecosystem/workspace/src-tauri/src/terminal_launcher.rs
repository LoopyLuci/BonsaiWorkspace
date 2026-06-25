//! Launch the Bonsai Terminal Interface (bti) from the GUI.
//!
//! Finds the `bti` binary next to the current executable (bundled release)
//! or falls back to `cargo run -p bonsai-tui` in the workspace root (dev).

use std::path::PathBuf;
use std::process::Stdio;
use tracing::{info, warn};

// ── Binary resolution ──────────────────────────────────────────────────────────

/// Find the `bti` binary to launch, in order of preference:
/// 1. `~/.bonsai/bin/bti[.exe]`
/// 2. Sibling of the current executable (bundled)
/// 3. `cargo run -p bonsai-tui` via workspace root (dev only)
fn find_bti_binary() -> Option<BtiLaunchMode> {
    // 1. ~/.bonsai/bin/bti
    if let Some(home) = dirs::home_dir() {
        let name = if cfg!(windows) { "bti.exe" } else { "bti" };
        let candidate = home.join(".bonsai/bin").join(name);
        if candidate.exists() {
            return Some(BtiLaunchMode::Binary(candidate));
        }
    }

    // 2. Sibling of the running executable
    if let Ok(exe) = std::env::current_exe() {
        let name = if cfg!(windows) { "bti.exe" } else { "bti" };
        let candidate = exe.parent().map(|p| p.join(name)).unwrap_or_default();
        if candidate.exists() {
            return Some(BtiLaunchMode::Binary(candidate));
        }
    }

    // 3. Dev mode: workspace root cargo run
    // Walk up from current dir looking for Cargo.toml with bonsai-tui member
    let mut dir = std::env::current_dir().ok()?;
    for _ in 0..6 {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("bonsai-tui") {
                    return Some(BtiLaunchMode::Cargo { workspace_root: dir });
                }
            }
        }
        dir = dir.parent()?.to_path_buf();
    }

    None
}

enum BtiLaunchMode {
    Binary(PathBuf),
    Cargo { workspace_root: PathBuf },
}

// ── Tauri commands ─────────────────────────────────────────────────────────────

/// Read the IPC port from `~/.bonsai/vscode_port` (set by the running daemon).
fn read_daemon_port() -> u16 {
    dirs::home_dir()
        .map(|h| h.join(".bonsai/vscode_port"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(11370)
}

/// Launch the Bonsai Terminal Interface in a new terminal window.
///
/// On Windows: opens a new `conhost` / Windows Terminal window.
/// On macOS: opens a new Terminal.app or iTerm window.
/// On Linux: tries `gnome-terminal`, `konsole`, `xterm` in order.
#[tauri::command]
pub async fn launch_bonsai_terminal(
    panel: Option<String>,
    workspace: Option<String>,
) -> Result<(), String> {
    let mode = find_bti_binary().ok_or_else(|| {
        "bti binary not found. Install with: cargo install --path crates/bonsai-tui".to_string()
    })?;

    let port = read_daemon_port();
    let connect_arg = format!("--connect 127.0.0.1:{port}");
    let panel_arg = panel.map(|p| format!("--panel {p}")).unwrap_or_default();

    // Build the bti command string
    let bti_invocation = match &mode {
        BtiLaunchMode::Binary(path) => {
            let bti = path.to_string_lossy();
            if panel_arg.is_empty() {
                format!("{bti} {connect_arg}")
            } else {
                format!("{bti} {connect_arg} {panel_arg}")
            }
        }
        BtiLaunchMode::Cargo { workspace_root } => {
            let root = workspace_root.to_string_lossy();
            let args = if panel_arg.is_empty() {
                connect_arg.clone()
            } else {
                format!("{connect_arg} {panel_arg}")
            };
            format!("cd {root} && cargo run -p bonsai-tui --bin bti -- {args}")
        }
    };

    info!(bti_invocation, "[terminal_launcher] launching BTI");

    // Spawn inside a terminal emulator
    spawn_in_terminal(&bti_invocation, workspace.as_deref())
        .map_err(|e| format!("Failed to launch terminal: {e}"))?;

    Ok(())
}

fn spawn_in_terminal(cmd: &str, _workspace: Option<&str>) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        // Try Windows Terminal (wt) first, fall back to cmd.exe
        let wt = std::process::Command::new("wt")
            .args(["new-tab", "--", "cmd", "/k", cmd])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        if wt.is_ok() {
            return Ok(());
        }

        // Fall back to a plain cmd window
        std::process::Command::new("cmd")
            .args(["/c", "start", "cmd", "/k", cmd])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"tell application "Terminal" to do script "{cmd}""#
        );
        std::process::Command::new("osascript")
            .args(["-e", &script])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        // Try common terminal emulators
        let emulators = [
            ("gnome-terminal", vec!["--", "bash", "-c", &format!("{cmd}; exec bash")]),
            ("konsole", vec!["-e", "bash", "-c", &format!("{cmd}; exec bash")]),
            ("xfce4-terminal", vec!["-e", &format!("bash -c '{cmd}; exec bash'")]),
            ("xterm", vec!["-e", &format!("bash -c '{cmd}; exec bash'")]),
        ];
        for (term, args) in &emulators {
            let result = std::process::Command::new(term)
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            if result.is_ok() {
                return Ok(());
            }
        }
        warn!("[terminal_launcher] no terminal emulator found; falling back to detached process");
        // Last resort: just run bti detached (no new window, but at least it starts)
        std::process::Command::new("sh")
            .args(["-c", cmd])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
    }

    Ok(())
}

/// Quick-check whether `bti` is available.
#[tauri::command]
pub fn bti_available() -> bool {
    find_bti_binary().is_some()
}
