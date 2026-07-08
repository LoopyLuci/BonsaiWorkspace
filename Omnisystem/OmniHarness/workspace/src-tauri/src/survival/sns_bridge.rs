//! Survival System — Sandbox Nervous System bridge.
//!
//! One of the Survival System's tools (see `super` for the others: `kb`,
//! `bug_db`, `bug_hunter`, `crash_ingest`, `daemon`). Before this, `sns`
//! (capability-sandbox violations) and `failure-finder`/F³ (fuzzing-
//! discovered crashes) only ever fed the shell-script KB one-way (via
//! `SurvivalBridge`, and the `sns`-crash-handler fallback wired in
//! `lib.rs`) and had **zero frontend** — `fff_list_failures`/
//! `sns_list_violations` were never once called from the UI (confirmed via
//! grep). This module makes them a second, additional source feeding the
//! Bug Database, so fuzzing/sandbox findings show up in the same catalog as
//! compile/test/lint/runtime-crash bugs, not only in a separate KB path.

use failure_finder::{F3Orchestrator, FailureReport};
use sns::{CapabilityViolation, SandboxSupervisor};

use super::bug_db::DiscoveredBug;

pub struct SnsIngestCursor {
    last_seen_failure_ns: u64,
    last_seen_violation_ns: u64,
}

impl SnsIngestCursor {
    pub fn new() -> Self {
        Self { last_seen_failure_ns: 0, last_seen_violation_ns: 0 }
    }

    pub fn poll_new(&mut self, f3: &F3Orchestrator, sns: &SandboxSupervisor) -> Vec<DiscoveredBug> {
        let mut bugs = Vec::new();

        let mut max_failure_ns = self.last_seen_failure_ns;
        for f in f3.list_failures() {
            if f.timestamp_ns <= self.last_seen_failure_ns {
                continue;
            }
            max_failure_ns = max_failure_ns.max(f.timestamp_ns);
            bugs.push(failure_to_bug(&f));
        }
        self.last_seen_failure_ns = max_failure_ns;

        let mut max_violation_ns = self.last_seen_violation_ns;
        for v in sns.all_violations() {
            if v.timestamp_ns <= self.last_seen_violation_ns {
                continue;
            }
            max_violation_ns = max_violation_ns.max(v.timestamp_ns);
            bugs.push(violation_to_bug(&v));
        }
        self.last_seen_violation_ns = max_violation_ns;

        bugs
    }
}

fn failure_to_bug(f: &FailureReport) -> DiscoveredBug {
    DiscoveredBug {
        source: "runtime".to_string(),
        severity: "error".to_string(),
        title: format!("Fuzz failure ({}): {}", f.target, f.error_pattern).chars().take(200).collect(),
        message: format!(
            "strategy: {}\nerror: {}\nreproduction: {}\nbacktrace:\n{}",
            f.strategy, f.error_pattern, f.reproduction_cmd, f.backtrace
        ),
        file_path: None,
        line_number: None,
    }
}

fn violation_to_bug(v: &CapabilityViolation) -> DiscoveredBug {
    DiscoveredBug {
        source: "runtime".to_string(),
        severity: "error".to_string(),
        title: format!("Sandbox violation ({}): {:?}", v.component, v.violation_type).chars().take(200).collect(),
        message: format!(
            "sandbox: {}\ncomponent: {}\nviolation: {:?}\nattempted action: {}\nblocked: {}",
            v.sandbox_id, v.component, v.violation_type, v.attempted_action, v.blocked
        ),
        file_path: None,
        line_number: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_starts_at_zero() {
        let cursor = SnsIngestCursor::new();
        assert_eq!(cursor.last_seen_failure_ns, 0);
        assert_eq!(cursor.last_seen_violation_ns, 0);
    }

    #[test]
    fn failure_to_bug_includes_target_and_reproduction_command() {
        let f = FailureReport {
            id: "f1".into(),
            campaign_id: "c1".into(),
            target: "tauri_filesystem".into(),
            strategy: "mutate".into(),
            error_pattern: "panic: index out of bounds".into(),
            backtrace: "at foo.rs:10".into(),
            minimal_input: serde_json::json!({}),
            input_hash: "abc".into(),
            reproduction_cmd: "cargo run --bin repro -- f1".into(),
            timestamp_ns: 1,
            auto_fix_script: None,
        };
        let bug = failure_to_bug(&f);
        assert_eq!(bug.source, "runtime");
        assert!(bug.title.contains("tauri_filesystem"));
        assert!(bug.message.contains("cargo run --bin repro -- f1"));
    }
}
