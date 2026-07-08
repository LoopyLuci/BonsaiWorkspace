//! Risk classification for self-upgrade proposals — the first real
//! consumer of `capability_registry::trust_score` (the gate math was
//! defined but never wired into anything before this).
//!
//! Two independent questions decide what happens to a proposal:
//!   1. Does it touch anything safety-critical? (`classify_paths`) — if so,
//!      it's `Blocked` outright, before a sandbox build/test is even run.
//!   2. For everything else: did the sandboxed build+test pass?
//!      (`decide_tier`) — only a change that both stays out of
//!      safety-critical territory *and* passes its own tests can
//!      auto-merge; a wide-surface (many-file) change is capped at
//!      `StagedApproval` even with green tests, since breadth alone is a
//!      real risk signal tests don't fully cover.

use capability_registry::trust_score::{TrustScore, GATE_STAGING};

/// Files/paths that gate their own safety mechanism — the self-upgrader can
/// never approve changes here autonomously, only propose them for human
/// review. Checked as a substring match against each touched path.
pub const SAFETY_CRITICAL_PATHS: &[&str] = &[
    "self_upgrade/",
    "agents/self_upgrader.rs",
    "capability-registry/",
    "trust_score.rs",
    "plan_gate.rs",
    "assistant_audit_log",
    "secrets_store.rs",
    "features.rs",
];

/// A many-file change is a real risk signal on its own — capped at
/// `StagedApproval` even if every file individually looks safe and tests
/// pass, since breadth of change is exactly what a per-file safety-path
/// check and a test suite can both miss.
const WIDE_SURFACE_FILE_COUNT: usize = 5;
const WIDE_SURFACE_PENALTY: u8 = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyTier {
    /// Sandboxed, tested, and merged without waiting for a human.
    AutoMerge,
    /// Sandboxed and tested, but a human must approve the merge.
    StagedApproval,
    /// Touches a safety-critical path — never attempted autonomously; a
    /// human must review the raw proposal before any sandbox work happens.
    Blocked,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RiskAssessment {
    pub score: u8,
    pub violations: u8,
    pub touched_safety_critical: Vec<String>,
    pub wide_surface: bool,
}

/// Pass 1: does this proposal touch anything safety-critical? Call this
/// *before* running any sandbox build — a `Blocked` result here means the
/// agent should never even attempt the change unsupervised.
pub fn classify_paths(touched_paths: &[String]) -> RiskAssessment {
    let mut score = TrustScore::baseline();
    let touched_safety_critical: Vec<String> = touched_paths
        .iter()
        .filter(|p| SAFETY_CRITICAL_PATHS.iter().any(|sp| p.contains(sp)))
        .cloned()
        .collect();

    for _ in &touched_safety_critical {
        score.add_capability_penalty(30);
        score.record_violation();
    }

    let wide_surface = touched_paths.len() > WIDE_SURFACE_FILE_COUNT;
    if wide_surface && touched_safety_critical.is_empty() {
        score.add_capability_penalty(WIDE_SURFACE_PENALTY);
    }

    RiskAssessment {
        score: score.score(),
        violations: score.violations,
        touched_safety_critical,
        wide_surface,
    }
}

/// Pass 2: given the pre-sandbox assessment and whether the sandboxed
/// build+test passed, decide the final tier.
pub fn decide_tier(assessment: &RiskAssessment, tests_passed: bool) -> AutonomyTier {
    if !assessment.touched_safety_critical.is_empty() {
        return AutonomyTier::Blocked;
    }
    // Reconstruct enough of the score to reuse `passes_gate` rather than
    // re-deriving the threshold comparison by hand.
    let mut score = TrustScore::baseline();
    if assessment.wide_surface {
        score.add_capability_penalty(WIDE_SURFACE_PENALTY);
    }
    if score.passes_gate(GATE_STAGING) && tests_passed {
        AutonomyTier::AutoMerge
    } else {
        AutonomyTier::StagedApproval
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trivial, low-risk, single-file change with green sandbox tests
    /// must land in the fully-autonomous auto-merge tier — this is the
    /// plan's "deliberately trivial goal" verification case.
    #[test]
    fn low_risk_change_with_passing_tests_auto_merges() {
        let touched = vec!["Omnisystem/OmniHarness/workspace/src-tauri/src/some_module.rs".to_string()];
        let assessment = classify_paths(&touched);
        assert!(assessment.touched_safety_critical.is_empty());
        assert!(!assessment.wide_surface);
        assert_eq!(decide_tier(&assessment, true), AutonomyTier::AutoMerge);
    }

    /// The same low-risk change with a *failing* sandbox must never
    /// auto-merge, regardless of how safe the touched paths looked.
    #[test]
    fn low_risk_change_with_failing_tests_is_staged_not_merged() {
        let touched = vec!["Omnisystem/OmniHarness/workspace/src-tauri/src/some_module.rs".to_string()];
        let assessment = classify_paths(&touched);
        assert_eq!(decide_tier(&assessment, false), AutonomyTier::StagedApproval);
    }

    /// A change touching the trust-score gate itself (or any other
    /// safety-critical path) must be Blocked outright — this is the plan's
    /// "deliberately risky goal" verification case. Note this is decided
    /// entirely by `classify_paths` before `decide_tier`/tests ever run —
    /// matching "blocked before the agent even starts", not just before merge.
    #[test]
    fn safety_critical_path_is_always_blocked() {
        let touched = vec![
            "Omnisystem/OmniHarness/crates/capability-registry/src/trust_score.rs".to_string(),
        ];
        let assessment = classify_paths(&touched);
        assert_eq!(assessment.touched_safety_critical.len(), 1);
        // Even with a hypothetically perfect sandbox run, still blocked.
        assert_eq!(decide_tier(&assessment, true), AutonomyTier::Blocked);
    }

    /// A change touching many files is capped at staged approval even if
    /// every individual file looks safe and tests pass — breadth alone is
    /// a real risk signal.
    #[test]
    fn wide_surface_change_is_capped_at_staged_approval() {
        let touched: Vec<String> = (0..8).map(|i| format!("some/harmless/file_{i}.rs")).collect();
        let assessment = classify_paths(&touched);
        assert!(assessment.touched_safety_critical.is_empty());
        assert!(assessment.wide_surface);
        assert_eq!(decide_tier(&assessment, true), AutonomyTier::StagedApproval);
    }
}
