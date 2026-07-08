//! The self-building agent — proposes (and, depending on risk, applies)
//! changes to Omnisystem's own source. See `self_upgrade/{risk,sandbox,
//! gate}.rs` for the tiered-autonomy design this drives.
//!
//! Reuses `code_writer`'s model-calling + fenced-code-block convention
//! wholesale (`call_model`/`extract_files`) rather than re-implementing it —
//! a self-upgrade proposal is structurally the same "goal in, files out"
//! shape as an ordinary code-writing request, just routed through a risk
//! gate and a sandbox instead of writing straight to disk.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tauri::AppHandle;

use crate::agent::{Agent, AgentAction, AgentCapability, AgentContext, AgentMessage, AgentMetadata, AgentOutput};
use crate::agents::code_writer::{call_model, extract_files};
use crate::error::AgentError;
use crate::self_upgrade::gate::{FileChange, ProposalStatus, SelfUpgradeGateState, SelfUpgradeProposal};
use crate::self_upgrade::risk::{self, AutonomyTier};
use crate::self_upgrade::sandbox;

pub struct SelfUpgrader {
    pub gate: Arc<SelfUpgradeGateState>,
    pub app_handle: AppHandle,
    pub repo_root: PathBuf,
}

const SYSTEM_PROMPT_PREFIX: &str = "You are Omnisystem's self-upgrade agent — you propose changes to \
    Omnisystem's own source code. Wrap every file in a fenced code block with the file path, \
    RELATIVE TO THE REPOSITORY ROOT (e.g. `Omnisystem/OmniHarness/workspace/src-tauri/src/foo.rs`), \
    as a comment on the first line inside the block, like:\n\
    ```language\n// path: Omnisystem/OmniHarness/workspace/src-tauri/src/foo.rs\n<code>\n```\n\
    Propose the minimal set of files needed. Goal: ";

#[async_trait]
impl Agent for SelfUpgrader {
    fn metadata(&self) -> AgentMetadata {
        AgentMetadata {
            id: "self-upgrader".into(),
            name: "Self-Upgrade Agent".into(),
            description: "Proposes changes to Omnisystem's own source, sandboxed and risk-gated before anything is applied.".into(),
            capabilities: vec![
                AgentCapability::TextGeneration,
                AgentCapability::CodeEditing,
                AgentCapability::FileManipulation,
            ],
        }
    }

    async fn handle_message(&self, ctx: AgentContext, msg: AgentMessage) -> Result<AgentOutput, AgentError> {
        if !crate::features::FeatureFlags::is_enabled("self_upgrade") {
            return Err(AgentError::Internal(
                "Self-upgrade is disabled — enable it in Settings \u{2192} Self-Build first.".into(),
            ));
        }
        let model_url = ctx
            .model_url
            .as_deref()
            .ok_or_else(|| AgentError::Orchestrator("No model slot is ready — load a model first".into()))?;

        let prompt = format!("{SYSTEM_PROMPT_PREFIX}{}", msg.content);
        let raw = call_model(model_url, &prompt).await?;
        let extracted = extract_files(&raw);

        if extracted.is_empty() {
            return Ok(AgentOutput {
                content: format!("No file changes were proposed.\n\n{raw}"),
                actions: vec![],
                metadata: None,
            });
        }

        let touched_paths: Vec<String> = extracted.iter().map(|f| f.path.clone()).collect();
        let assessment = risk::classify_paths(&touched_paths);

        let files: Vec<FileChange> = extracted
            .iter()
            .map(|f| {
                let old = std::fs::read_to_string(self.repo_root.join(&f.path)).unwrap_or_default();
                let unified_diff = diffy::create_patch(&old, &f.content).to_string();
                FileChange { path: f.path.clone(), unified_diff, new_content: f.content.clone() }
            })
            .collect();

        let id = uuid::Uuid::new_v4().to_string();
        let goal = msg.content.clone();

        // Safety-critical: never even attempted autonomously — a human must
        // explicitly approve the raw proposal before any sandbox work runs.
        if !assessment.touched_safety_critical.is_empty() {
            let proposal = SelfUpgradeProposal {
                id: id.clone(),
                goal,
                files,
                risk: assessment,
                tier: AutonomyTier::Blocked,
                sandbox: None,
                status: ProposalStatus::PendingReview,
                created_at_ms: chrono::Utc::now().timestamp_millis(),
            };
            self.gate.push(proposal, &self.app_handle);
            return Ok(AgentOutput {
                content: format!(
                    "Proposal touches safety-critical files — blocked pending human review (id: {id})."
                ),
                actions: vec![AgentAction { kind: "self_upgrade_blocked".into(), payload: json!({ "id": id }) }],
                metadata: None,
            });
        }

        // Not safety-critical: sandbox it (build + test in an isolated worktree).
        let files_for_sandbox: Vec<(String, String)> =
            extracted.iter().map(|f| (f.path.clone(), f.content.clone())).collect();

        let outcome = sandbox::run_in_worktree(&self.repo_root, &files_for_sandbox, None)
            .map_err(|e| AgentError::Internal(format!("Sandbox setup failed: {e}")))?;

        let tier = risk::decide_tier(&assessment, outcome.passed());
        let status = if !outcome.passed() {
            ProposalStatus::Failed
        } else if tier == AutonomyTier::AutoMerge {
            ProposalStatus::AutoMerged
        } else {
            ProposalStatus::PendingApproval
        };

        let proposal = SelfUpgradeProposal {
            id: id.clone(),
            goal,
            files,
            risk: assessment,
            tier,
            sandbox: Some(outcome.clone()),
            status,
            created_at_ms: chrono::Utc::now().timestamp_millis(),
        };
        self.gate.push(proposal, &self.app_handle);

        match status {
            ProposalStatus::AutoMerged => match sandbox::promote(&self.repo_root, &outcome) {
                Ok(()) => Ok(AgentOutput {
                    content: format!(
                        "Auto-merged (id: {id}) — sandbox build+test passed and the change was low-risk."
                    ),
                    actions: vec![AgentAction { kind: "self_upgrade_auto_merged".into(), payload: json!({ "id": id }) }],
                    metadata: None,
                }),
                Err(e) => {
                    self.gate.mark_failed(&id, &self.app_handle);
                    Err(AgentError::Internal(format!("Sandbox passed but promotion failed: {e}")))
                }
            },
            ProposalStatus::Failed => {
                let _ = sandbox::discard(&self.repo_root, &outcome);
                Ok(AgentOutput {
                    content: format!("Sandbox build/test failed (id: {id}) — discarded, nothing applied.\n\n{}\n{}", outcome.build_output, outcome.test_output),
                    actions: vec![AgentAction { kind: "self_upgrade_failed".into(), payload: json!({ "id": id }) }],
                    metadata: None,
                })
            }
            _ => Ok(AgentOutput {
                content: format!("Sandboxed and tested; awaiting human approval (id: {id})."),
                actions: vec![AgentAction { kind: "self_upgrade_pending_approval".into(), payload: json!({ "id": id }) }],
                metadata: None,
            }),
        }
    }

    async fn shutdown(&self) {}
}
