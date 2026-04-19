use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Serialize;

// ── Counters ──────────────────────────────────────────────────────────────────

pub struct AssistantMetrics {
    pub turn_count:           AtomicU64,
    pub turn_total_ms:        AtomicU64,  // sum — divide by turn_count for avg
    pub tool_call_count:      AtomicU64,
    pub tool_error_count:     AtomicU64,
    pub tts_synthesis_count:  AtomicU64,
    pub tts_total_ms:         AtomicU64,
    pub tts_error_count:      AtomicU64,
    pub session_restore_count: AtomicU64,
    pub sidecar_restart_count: AtomicU64,
    last_errors:              Mutex<Vec<ErrorEntry>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorEntry {
    pub ts:      i64,
    pub context: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct MetricsSnapshot {
    pub turn_count:           u64,
    pub avg_turn_ms:          u64,
    pub tool_call_count:      u64,
    pub tool_error_rate_pct:  f64,
    pub tts_count:            u64,
    pub avg_tts_ms:           u64,
    pub tts_error_count:      u64,
    pub session_restore_count: u64,
    pub sidecar_restart_count: u64,
    pub last_errors:          Vec<ErrorEntry>,
}

impl AssistantMetrics {
    pub fn new() -> Self {
        AssistantMetrics {
            turn_count:            AtomicU64::new(0),
            turn_total_ms:         AtomicU64::new(0),
            tool_call_count:       AtomicU64::new(0),
            tool_error_count:      AtomicU64::new(0),
            tts_synthesis_count:   AtomicU64::new(0),
            tts_total_ms:          AtomicU64::new(0),
            tts_error_count:       AtomicU64::new(0),
            session_restore_count: AtomicU64::new(0),
            sidecar_restart_count: AtomicU64::new(0),
            last_errors:           Mutex::new(Vec::new()),
        }
    }

    pub fn record_turn(&self, duration_ms: u64) {
        self.turn_count.fetch_add(1, Ordering::Relaxed);
        self.turn_total_ms.fetch_add(duration_ms, Ordering::Relaxed);
    }

    pub fn record_tool(&self, error: bool) {
        self.tool_call_count.fetch_add(1, Ordering::Relaxed);
        if error { self.tool_error_count.fetch_add(1, Ordering::Relaxed); }
    }

    pub fn record_tts(&self, duration_ms: u64, error: bool) {
        self.tts_synthesis_count.fetch_add(1, Ordering::Relaxed);
        self.tts_total_ms.fetch_add(duration_ms, Ordering::Relaxed);
        if error { self.tts_error_count.fetch_add(1, Ordering::Relaxed); }
    }

    pub fn record_error(&self, context: &str, message: &str) {
        let entry = ErrorEntry {
            ts: now_ms(),
            context: context.to_string(),
            message: message.chars().take(256).collect(),
        };
        let mut guard = self.last_errors.lock().unwrap();
        guard.push(entry);
        // Keep last 20 errors only
        if guard.len() > 20 {
            let excess = guard.len() - 20;
            guard.drain(0..excess);
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        let turns     = self.turn_count.load(Ordering::Relaxed);
        let total_ms  = self.turn_total_ms.load(Ordering::Relaxed);
        let tts_count = self.tts_synthesis_count.load(Ordering::Relaxed);
        let tts_ms    = self.tts_total_ms.load(Ordering::Relaxed);
        let tools     = self.tool_call_count.load(Ordering::Relaxed);
        let errs      = self.tool_error_count.load(Ordering::Relaxed);

        MetricsSnapshot {
            turn_count:            turns,
            avg_turn_ms:           if turns > 0 { total_ms / turns } else { 0 },
            tool_call_count:       tools,
            tool_error_rate_pct:   if tools > 0 { errs as f64 / tools as f64 * 100.0 } else { 0.0 },
            tts_count,
            avg_tts_ms:            if tts_count > 0 { tts_ms / tts_count } else { 0 },
            tts_error_count:       self.tts_error_count.load(Ordering::Relaxed),
            session_restore_count: self.session_restore_count.load(Ordering::Relaxed),
            sidecar_restart_count: self.sidecar_restart_count.load(Ordering::Relaxed),
            last_errors:           self.last_errors.lock().unwrap().clone(),
        }
    }
}

// ── Health status ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SidecarHealth {
    pub name:    String,
    pub healthy: bool,
    pub last_checked_ts: i64,
    pub consecutive_failures: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssistantHealth {
    pub sidecars:   Vec<SidecarHealth>,
    pub db_ok:      bool,
    pub last_error: Option<String>,
    pub checked_at: i64,
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64
}
