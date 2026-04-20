/// Tool Health — per-tool reliability scoring and circuit breaker.
///
/// `ToolHealthTracker` maintains a rolling window of success/failure counts per tool.
/// The derived `health_score` (0.0..=1.0) feeds the tool selector's ranking.
/// A circuit breaker (Closed → Open → HalfOpen → Closed) prevents repeatedly
/// invoking a broken tool until it demonstrates recovery.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;

// ── Health window ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct WindowEntry {
    success: bool,
    at:      Instant,
}

#[derive(Debug, Clone, Default)]
struct ToolWindow {
    entries: Vec<WindowEntry>,
}

impl ToolWindow {
    fn record(&mut self, success: bool, window: Duration) {
        let now = Instant::now();
        self.evict(now, window);
        self.entries.push(WindowEntry { success, at: now });
    }

    fn evict(&mut self, now: Instant, window: Duration) {
        self.entries.retain(|e| now.duration_since(e.at) < window);
    }

    fn health_score(&self) -> f32 {
        if self.entries.is_empty() {
            return 1.0; // no data → assume healthy
        }
        let successes = self.entries.iter().filter(|e| e.success).count();
        successes as f32 / self.entries.len() as f32
    }

    fn consecutive_failures(&self) -> u32 {
        let mut count = 0u32;
        for entry in self.entries.iter().rev() {
            if !entry.success {
                count += 1;
            } else {
                break;
            }
        }
        count
    }
}

// ── Circuit breaker ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum BreakerState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
struct BreakerInner {
    state:            BreakerState,
    opened_at:        Option<Instant>,
    probe_successes:  u32,
}

impl Default for BreakerInner {
    fn default() -> Self {
        Self { state: BreakerState::Closed, opened_at: None, probe_successes: 0 }
    }
}

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Rolling window over which success rate is computed.
    pub window:                  Duration,
    /// Consecutive failures before breaker trips to Open.
    pub open_after_failures:     u32,
    /// How long the breaker stays Open before probing.
    pub half_open_after:         Duration,
    /// Probe successes needed to return to Closed.
    pub close_on_successes:      u32,
    /// Minimum calls before health score departs from 1.0.
    pub min_sample_size:         usize,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            window:              Duration::from_secs(300), // 5-min rolling window
            open_after_failures: 5,
            half_open_after:     Duration::from_secs(30),
            close_on_successes:  2,
            min_sample_size:     3,
        }
    }
}

// ── Per-tool entry ────────────────────────────────────────────────────────────

struct ToolEntry {
    window:  ToolWindow,
    breaker: BreakerInner,
}

impl Default for ToolEntry {
    fn default() -> Self {
        Self { window: ToolWindow::default(), breaker: BreakerInner::default() }
    }
}

// ── ToolHealthTracker ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ToolHealthTracker {
    inner:  Arc<Mutex<HashMap<String, ToolEntry>>>,
    config: HealthConfig,
}

impl ToolHealthTracker {
    pub fn new(config: HealthConfig) -> Self {
        Self { inner: Arc::new(Mutex::new(HashMap::new())), config }
    }

    pub fn with_defaults() -> Self {
        Self::new(HealthConfig::default())
    }

    /// Record the outcome of a tool invocation.
    pub fn record(&self, tool: &str, success: bool) {
        let mut map = self.inner.lock().unwrap();
        let entry = map.entry(tool.to_string()).or_default();
        entry.window.record(success, self.config.window);
        self.update_breaker(entry, success);
    }

    fn update_breaker(&self, entry: &mut ToolEntry, success: bool) {
        let cfg = &self.config;
        match entry.breaker.state {
            BreakerState::Closed => {
                if entry.window.consecutive_failures() >= cfg.open_after_failures {
                    entry.breaker.state     = BreakerState::Open;
                    entry.breaker.opened_at = Some(Instant::now());
                    entry.breaker.probe_successes = 0;
                }
            }
            BreakerState::Open => {
                // If enough time has elapsed, transition to HalfOpen and treat
                // the current record as a probe result.
                if let Some(opened) = entry.breaker.opened_at {
                    if opened.elapsed() >= cfg.half_open_after {
                        // Move to HalfOpen and evaluate this probe result below.
                        entry.breaker.state = BreakerState::HalfOpen;
                        if success {
                            entry.breaker.probe_successes += 1;
                            if entry.breaker.probe_successes >= cfg.close_on_successes {
                                entry.breaker.state = BreakerState::Closed;
                                entry.breaker.opened_at = None;
                                entry.breaker.probe_successes = 0;
                            }
                        } else {
                            // Probe failed — reopen the breaker and reset timer.
                            entry.breaker.state     = BreakerState::Open;
                            entry.breaker.opened_at = Some(Instant::now());
                            entry.breaker.probe_successes = 0;
                        }
                    }
                }
            }
            BreakerState::HalfOpen => {
                if success {
                    entry.breaker.probe_successes += 1;
                    if entry.breaker.probe_successes >= cfg.close_on_successes {
                        entry.breaker.state = BreakerState::Closed;
                        entry.breaker.opened_at = None;
                        entry.breaker.probe_successes = 0;
                    }
                } else {
                    // Probe failed — reopen.
                    entry.breaker.state     = BreakerState::Open;
                    entry.breaker.opened_at = Some(Instant::now());
                    entry.breaker.probe_successes = 0;
                }
            }
        }
    }

    /// Returns true if the tool is allowed to run (breaker Closed or HalfOpen probe).
    /// Side effect: may transition Open → HalfOpen if enough time has elapsed.
    pub fn is_allowed(&self, tool: &str) -> bool {
        let mut map = self.inner.lock().unwrap();
        let entry = map.entry(tool.to_string()).or_default();
        let cfg = &self.config;

        if entry.breaker.state == BreakerState::Open {
            if let Some(opened) = entry.breaker.opened_at {
                if opened.elapsed() >= cfg.half_open_after {
                    entry.breaker.state = BreakerState::HalfOpen;
                    return true; // allow probe
                }
            }
            return false;
        }

        true
    }

    /// Health score for a tool (0.0 = all failures, 1.0 = all successes / no data).
    pub fn health_score(&self, tool: &str) -> f32 {
        let mut map = self.inner.lock().unwrap();
        let entry = map.entry(tool.to_string()).or_default();
        entry.window.evict(Instant::now(), self.config.window);
        if entry.window.entries.len() < self.config.min_sample_size {
            return 1.0;
        }
        entry.window.health_score()
    }

    /// Snapshot of all tracked tools for diagnostics.
    pub fn snapshot(&self) -> Vec<ToolHealthSnapshot> {
        let mut map = self.inner.lock().unwrap();
        let now = Instant::now();
        map.iter_mut().map(|(name, entry)| {
            entry.window.evict(now, self.config.window);
            let total    = entry.window.entries.len();
            let successes = entry.window.entries.iter().filter(|e| e.success).count();
            ToolHealthSnapshot {
                tool:     name.clone(),
                score:    entry.window.health_score(),
                total,
                successes,
                failures: total - successes,
                breaker:  entry.breaker.state.clone(),
            }
        })
        .collect()
    }

    /// Reset all state for a tool (useful after a deploy or manual recovery).
    pub fn reset(&self, tool: &str) {
        let mut map = self.inner.lock().unwrap();
        map.remove(tool);
    }
}

#[derive(Debug, Serialize)]
pub struct ToolHealthSnapshot {
    pub tool:      String,
    pub score:     f32,
    pub total:     usize,
    pub successes: usize,
    pub failures:  usize,
    pub breaker:   BreakerState,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn tracker() -> ToolHealthTracker {
        ToolHealthTracker::new(HealthConfig {
            window:              Duration::from_secs(600),
            open_after_failures: 3,
            half_open_after:     Duration::from_millis(50), // very short for tests
            close_on_successes:  2,
            min_sample_size:     1,
        })
    }

    #[test]
    fn healthy_tool_is_allowed() {
        let t = tracker();
        assert!(t.is_allowed("my_tool"));
    }

    #[test]
    fn health_score_starts_at_one() {
        let t = tracker();
        assert!((t.health_score("no_data") - 1.0).abs() < 0.01);
    }

    #[test]
    fn all_successes_score_is_one() {
        let t = tracker();
        for _ in 0..5 { t.record("tool", true); }
        assert!((t.health_score("tool") - 1.0).abs() < 0.01);
    }

    #[test]
    fn mixed_score_is_correct() {
        let t = tracker();
        t.record("tool", true);
        t.record("tool", false);
        // 1 success out of 2 → 0.5
        assert!((t.health_score("tool") - 0.5).abs() < 0.01);
    }

    #[test]
    fn breaker_opens_after_failures() {
        let t = tracker();
        for _ in 0..3 { t.record("tool", false); }
        assert!(!t.is_allowed("tool"), "breaker should be Open");
    }

    #[test]
    fn breaker_transitions_to_half_open() {
        let t = tracker();
        for _ in 0..3 { t.record("tool", false); }
        // Wait for half_open_after (50ms in test config)
        std::thread::sleep(Duration::from_millis(60));
        assert!(t.is_allowed("tool"), "breaker should allow probe after wait");
    }

    #[test]
    fn breaker_closes_after_probe_successes() {
        let t = tracker();
        for _ in 0..3 { t.record("tool", false); }
        std::thread::sleep(Duration::from_millis(60));
        // Two probe successes should close the breaker
        t.record("tool", true);
        t.record("tool", true);
        assert!(t.is_allowed("tool"), "breaker should be Closed again");
    }

    #[test]
    fn breaker_reopens_on_probe_failure() {
        let t = tracker();
        for _ in 0..3 { t.record("tool", false); }
        std::thread::sleep(Duration::from_millis(60));
        t.record("tool", false); // probe fails → reopen
        assert!(!t.is_allowed("tool"), "should reopen after failed probe");
    }

    #[test]
    fn reset_clears_all_state() {
        let t = tracker();
        for _ in 0..5 { t.record("tool", false); }
        t.reset("tool");
        assert!((t.health_score("tool") - 1.0).abs() < 0.01);
        assert!(t.is_allowed("tool"));
    }

    #[test]
    fn snapshot_covers_all_tools() {
        let t = tracker();
        t.record("alpha", true);
        t.record("beta", false);
        let snap = t.snapshot();
        assert_eq!(snap.len(), 2);
    }
}
