use std::fs::{rename, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::sync::mpsc::UnboundedSender;

use crate::kernel_bridge::KernelMirrorEvent;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AuditEvent {
    pub ts: i64,
    pub tool: String,
    pub decision: String,  // "allowed" | "denied" | "confirmed" | "cancelled"
    pub args_hash: String, // SHA-256 of redacted/serialized args — not raw values
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub tool_call_id: Option<String>,
}

// ── AuditLog ──────────────────────────────────────────────────────────────────

const MAX_SIZE_BYTES: u64 = 10 * 1024 * 1024; // 10 MB

pub struct AuditLog {
    log_path: PathBuf,
    rotate_path: PathBuf,
    writer: Mutex<Option<File>>,
    /// Cross-language mirror — set via `attach_kernel_bridge` once the
    /// kernel_bridge is constructed. Every logged event is also forwarded
    /// here so the OmniHarness Rust kernel's hash-chained event store gets
    /// an independently verifiable copy (no-op when the kernel isn't running
    /// or this was never attached, e.g. in tests).
    kernel_tx: Mutex<Option<UnboundedSender<KernelMirrorEvent>>>,
}

impl AuditLog {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let log_path = app_data_dir.join("assistant-audit.log");
        let rotate_path = app_data_dir.join("assistant-audit.log.1");
        let writer = Mutex::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
                .ok(),
        );
        AuditLog {
            log_path,
            rotate_path,
            writer,
            kernel_tx: Mutex::new(None),
        }
    }

    /// Wires this audit log up to the kernel bridge so every future event is
    /// also mirrored into the kernel's cross-language audit chain.
    pub fn attach_kernel_bridge(&self, tx: UnboundedSender<KernelMirrorEvent>) {
        *self.kernel_tx.lock().unwrap() = Some(tx);
    }

    pub fn log(&self, event: AuditEvent) {
        let Ok(line) = serde_json::to_string(&event) else {
            return;
        };
        if let Ok(guard) = self.kernel_tx.lock() {
            if let Some(tx) = guard.as_ref() {
                let _ = tx.send(KernelMirrorEvent {
                    module_source: "workspace-assistant".to_string(),
                    event_type: format!("tool:{}:{}", event.tool, event.decision),
                    payload_json: line.clone(),
                    session_id: event.session_id.clone().unwrap_or_default(),
                });
            }
        }
        let mut guard = self.writer.lock().unwrap();
        self.maybe_rotate(&mut guard);
        if let Some(f) = guard.as_mut() {
            let _ = writeln!(f, "{line}");
        }
    }

    /// Convenience: log a tool decision immediately.
    pub fn log_decision(
        &self,
        tool: &str,
        decision: &str,
        args_json: &str,
        error: Option<String>,
        duration_ms: Option<u64>,
    ) {
        self.log_decision_with_context(
            tool,
            decision,
            args_json,
            error,
            duration_ms,
            None,
            None,
            None,
        );
    }

    /// Context-aware tool decision logging for end-to-end correlation.
    pub fn log_decision_with_context(
        &self,
        tool: &str,
        decision: &str,
        args_json: &str,
        error: Option<String>,
        duration_ms: Option<u64>,
        session_id: Option<&str>,
        turn_id: Option<&str>,
        tool_call_id: Option<&str>,
    ) {
        let args_hash = hash_args(args_json);
        self.log(AuditEvent {
            ts: now_ms(),
            tool: tool.to_string(),
            decision: decision.to_string(),
            args_hash,
            error,
            duration_ms,
            session_id: session_id.map(|s| s.to_string()),
            turn_id: turn_id.map(|s| s.to_string()),
            tool_call_id: tool_call_id.map(|s| s.to_string()),
        });
    }

    pub fn log_path(&self) -> &std::path::Path {
        &self.log_path
    }

    fn maybe_rotate(&self, writer: &mut Option<File>) {
        // Check current file size
        let too_big = self
            .log_path
            .metadata()
            .map(|m| m.len() > MAX_SIZE_BYTES)
            .unwrap_or(false);

        if !too_big {
            return;
        }

        // Close current file, rotate .log → .log.1, open fresh
        *writer = None;
        let _ = rename(&self.log_path, &self.rotate_path);
        *writer = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .ok();
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn hash_args(args_json: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(args_json.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
