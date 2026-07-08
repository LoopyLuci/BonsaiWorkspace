//! The self-upgrade proposal queue — distinct from `plan_gate.rs`'s
//! `PlanGateMiddleware`, which blocks a single in-flight tool-call thread
//! behind a 5-minute timeout. A self-upgrade proposal is generated
//! asynchronously and should sit in a review queue indefinitely (a human
//! reviewing code shouldn't have their window silently expire), so this is
//! a separate, list-based store rather than a reused oneshot channel.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use super::risk::{AutonomyTier, RiskAssessment};
use super::sandbox::SandboxOutcome;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub unified_diff: String,
    /// The full proposed file content — kept alongside the diff (rather
    /// than re-deriving it via `diffy::apply` later) so approving a
    /// never-sandboxed `PendingReview` proposal can run the sandbox
    /// directly from stored data, with no risk of a patch-apply mismatch.
    pub new_content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    /// Blocked tier: raw proposal awaiting a human decision to even attempt it.
    PendingReview,
    /// Sandboxed + tested, awaiting a human's one-click merge decision.
    PendingApproval,
    /// Auto-merged (staging-tier score + green sandbox tests).
    AutoMerged,
    /// A human approved a `PendingApproval` proposal.
    Approved,
    /// A human rejected it (either tier) — discarded, nothing applied.
    Rejected,
    /// Sandbox build or tests failed — never eligible to merge as-is.
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfUpgradeProposal {
    pub id: String,
    pub goal: String,
    pub files: Vec<FileChange>,
    pub risk: RiskAssessment,
    pub tier: AutonomyTier,
    pub sandbox: Option<SandboxOutcome>,
    pub status: ProposalStatus,
    pub created_at_ms: i64,
}

pub struct SelfUpgradeGateState {
    proposals: Mutex<Vec<SelfUpgradeProposal>>,
}

impl SelfUpgradeGateState {
    pub fn new() -> Self {
        Self { proposals: Mutex::new(Vec::new()) }
    }

    pub fn push(&self, proposal: SelfUpgradeProposal, app_handle: &AppHandle) {
        let _ = app_handle.emit("self-upgrade-proposal", &proposal);
        self.proposals.lock().unwrap().push(proposal);
    }

    pub fn list(&self) -> Vec<SelfUpgradeProposal> {
        self.proposals.lock().unwrap().clone()
    }

    /// Marks a `PendingApproval`/`PendingReview` proposal as `Approved` or
    /// `Rejected`. Returns the updated proposal so the caller can act on it
    /// (e.g. actually promote the sandboxed worktree on approval).
    pub fn resolve(&self, id: &str, approved: bool, app_handle: &AppHandle) -> Result<SelfUpgradeProposal, String> {
        let mut proposals = self.proposals.lock().unwrap();
        let p = proposals
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("No self-upgrade proposal with id '{id}'"))?;
        if !matches!(p.status, ProposalStatus::PendingReview | ProposalStatus::PendingApproval) {
            return Err(format!("Proposal '{id}' is already resolved (status: {:?})", p.status));
        }
        p.status = if approved { ProposalStatus::Approved } else { ProposalStatus::Rejected };
        let updated = p.clone();
        let _ = app_handle.emit("self-upgrade-proposal-resolved", &updated);
        Ok(updated)
    }

    pub fn mark_auto_merged(&self, id: &str, app_handle: &AppHandle) {
        let mut proposals = self.proposals.lock().unwrap();
        if let Some(p) = proposals.iter_mut().find(|p| p.id == id) {
            p.status = ProposalStatus::AutoMerged;
            let _ = app_handle.emit("self-upgrade-proposal-resolved", &p.clone());
        }
    }

    pub fn mark_failed(&self, id: &str, app_handle: &AppHandle) {
        let mut proposals = self.proposals.lock().unwrap();
        if let Some(p) = proposals.iter_mut().find(|p| p.id == id) {
            p.status = ProposalStatus::Failed;
            let _ = app_handle.emit("self-upgrade-proposal-resolved", &p.clone());
        }
    }
}
