use serde::{Deserialize, Serialize};
use std::sync::{atomic::AtomicU64, Arc};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrashSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityVerdict {
    Safe,
    Caution,
    Risky,
    Malicious,
    NotReviewed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    BuildStarted { crate_name: String, triggered_by: String },
    BuildFailed { crate_name: String, error: String, commit_sha: Option<String> },
    BuildPassed { crate_name: String, artifact_hash: String, duration_ms: u64 },
    TestStarted { suite: String },
    TestPassed { suite: String, count: usize, duration_ms: u64 },
    TestFailed { suite: String, failures: Vec<String>, duration_ms: u64 },
    CrashDetected { component: String, backtrace: String, severity: CrashSeverity },
    CrashResolved { component: String, fix_strategy: String, automated: bool },
    TrainingStarted { phase: String, dataset: String },
    TrainingProgress { phase: String, epoch: u32, total_epochs: u32, loss: f64 },
    TrainingComplete { phase: String, adapter_path: String, metrics: std::collections::HashMap<String, f64> },
    TrainingFailed { phase: String, error: String, epoch: u32 },
    UpgradeReady { component: String, version: String, cas_hash: String, source: String },
    UpgradeDeploying { component: String, version: String },
    UpgradeDeployed { component: String, version: String, duration_ms: u64 },
    UpgradeRolledBack { component: String, reason: String, previous_version: String },
    SecurityReviewRequested { extension_id: String, source: String },
    SecurityReviewComplete { extension_id: String, verdict: SecurityVerdict, risk_score: u8 },
    PRMerged { repo: String, pr_number: u64, commit_sha: String, author: String },
    ExternalPush { repo: String, branch: String, commit_sha: String, files_changed: Vec<String> },
    IssueCreated { issue_id: String, title: String, labels: Vec<String> },
    IssueAssigned { issue_id: String, assignee: String, swarm_template: String },
    IssueResolved { issue_id: String, resolution: String, commit_sha: Option<String> },
    ResourceThresholdAlert { device_id: String, resource: String, usage_pct: f64, threshold_pct: f64 },
    CreditEarned { amount: f64, project_id: String, contributor_id: String },
    CreditSpent { amount: f64, project_id: String, recipient_id: String },
    DreamCycleStarted { memory_nodes_count: usize },
    DreamCycleComplete { nodes_consolidated: usize, bonsai_md_updated: bool },
    ModelHotReloaded { model_path: String, previous_model: String },
    SwarmSpawned { swarm_id: String, template: String, agent_count: usize },
    SwarmCompleted { swarm_id: String, tasks_completed: usize, duration_ms: u64 },
    SwarmFailed { swarm_id: String, failed_task: String, error: String },
    ComponentHealthDegraded { component: String, health_score: f64, symptoms: Vec<String> },
    ComponentHealthRestored { component: String, health_score: f64 },
    ConfigChanged { key: String, old_value: String, new_value: String },
    ExtensionInstalled { extension_id: String, version: String, source: String },
    ExtensionUpdated { extension_id: String, old_version: String, new_version: String },
    ExtensionUninstalled { extension_id: String, reason: String },
    UserFeedback { context: String, rating: i8, comment: Option<String> },
    FeatureRequest { description: String, requester: String },
    CheckpointRequested { label: String, trigger: String },
    UiPanelGenerated { panel_id: String, description: String, cas_hash: String },
    UiPanelReloadRequested { panel_id: String, cas_hash: String },
    ProofVerificationFailed { tool: String, reason: String },
    // Linting & ETL Events
    RuleConfidenceUpdated { rule_id: String, old_confidence: f64, new_confidence: f64, action: String },
    RuleMutationProposed { rule_id: String, expected_improvement: f64, proposal_id: String },
    EtlCycleStarted { cycle_id: String },
    EtlCycleCompleted { cycle_id: String, feedback_events: usize, rules_updated: usize, duration_ms: u64 },
    EtlCycleFailed { cycle_id: String, error: String },
    DiagnosticFeedbackReceived { rule_id: String, file: String, line: u32, feedback_type: String },
}

impl SystemEvent {
    pub fn type_name(&self) -> &'static str {
        match self {
            SystemEvent::BuildStarted { .. } => "BuildStarted",
            SystemEvent::BuildFailed { .. } => "BuildFailed",
            SystemEvent::BuildPassed { .. } => "BuildPassed",
            SystemEvent::TestStarted { .. } => "TestStarted",
            SystemEvent::TestPassed { .. } => "TestPassed",
            SystemEvent::TestFailed { .. } => "TestFailed",
            SystemEvent::CrashDetected { .. } => "CrashDetected",
            SystemEvent::CrashResolved { .. } => "CrashResolved",
            SystemEvent::TrainingStarted { .. } => "TrainingStarted",
            SystemEvent::TrainingProgress { .. } => "TrainingProgress",
            SystemEvent::TrainingComplete { .. } => "TrainingComplete",
            SystemEvent::TrainingFailed { .. } => "TrainingFailed",
            SystemEvent::UpgradeReady { .. } => "UpgradeReady",
            SystemEvent::UpgradeDeploying { .. } => "UpgradeDeploying",
            SystemEvent::UpgradeDeployed { .. } => "UpgradeDeployed",
            SystemEvent::UpgradeRolledBack { .. } => "UpgradeRolledBack",
            SystemEvent::SecurityReviewRequested { .. } => "SecurityReviewRequested",
            SystemEvent::SecurityReviewComplete { .. } => "SecurityReviewComplete",
            SystemEvent::PRMerged { .. } => "PRMerged",
            SystemEvent::ExternalPush { .. } => "ExternalPush",
            SystemEvent::IssueCreated { .. } => "IssueCreated",
            SystemEvent::IssueAssigned { .. } => "IssueAssigned",
            SystemEvent::IssueResolved { .. } => "IssueResolved",
            SystemEvent::ResourceThresholdAlert { .. } => "ResourceThresholdAlert",
            SystemEvent::CreditEarned { .. } => "CreditEarned",
            SystemEvent::CreditSpent { .. } => "CreditSpent",
            SystemEvent::DreamCycleStarted { .. } => "DreamCycleStarted",
            SystemEvent::DreamCycleComplete { .. } => "DreamCycleComplete",
            SystemEvent::ModelHotReloaded { .. } => "ModelHotReloaded",
            SystemEvent::SwarmSpawned { .. } => "SwarmSpawned",
            SystemEvent::SwarmCompleted { .. } => "SwarmCompleted",
            SystemEvent::SwarmFailed { .. } => "SwarmFailed",
            SystemEvent::ComponentHealthDegraded { .. } => "ComponentHealthDegraded",
            SystemEvent::ComponentHealthRestored { .. } => "ComponentHealthRestored",
            SystemEvent::ConfigChanged { .. } => "ConfigChanged",
            SystemEvent::ExtensionInstalled { .. } => "ExtensionInstalled",
            SystemEvent::ExtensionUpdated { .. } => "ExtensionUpdated",
            SystemEvent::ExtensionUninstalled { .. } => "ExtensionUninstalled",
            SystemEvent::UserFeedback { .. } => "UserFeedback",
            SystemEvent::FeatureRequest { .. } => "FeatureRequest",
            SystemEvent::CheckpointRequested { .. } => "CheckpointRequested",
            SystemEvent::UiPanelGenerated { .. } => "UiPanelGenerated",
            SystemEvent::UiPanelReloadRequested { .. } => "UiPanelReloadRequested",
            SystemEvent::ProofVerificationFailed { .. } => "ProofVerificationFailed",
            SystemEvent::RuleConfidenceUpdated { .. } => "RuleConfidenceUpdated",
            SystemEvent::RuleMutationProposed { .. } => "RuleMutationProposed",
            SystemEvent::EtlCycleStarted { .. } => "EtlCycleStarted",
            SystemEvent::EtlCycleCompleted { .. } => "EtlCycleCompleted",
            SystemEvent::EtlCycleFailed { .. } => "EtlCycleFailed",
            SystemEvent::DiagnosticFeedbackReceived { .. } => "DiagnosticFeedbackReceived",
        }
    }
}

pub struct SystemEventBus {
    tx: broadcast::Sender<SystemEvent>,
    event_count: AtomicU64,
}

impl SystemEventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx, event_count: AtomicU64::new(0) }
    }

    pub fn publish(&self, event: SystemEvent) {
        self.event_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let name = event.type_name();
        match self.tx.send(event) {
            Ok(n) => tracing::debug!("SystemEvent: {} ({} receivers)", name, n),
            Err(_) => tracing::trace!("SystemEvent dropped (no receivers): {}", name),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SystemEvent> {
        self.tx.subscribe()
    }

    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }

    pub fn total_events(&self) -> u64 {
        self.event_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

pub type SharedEventBus = Arc<SystemEventBus>;
