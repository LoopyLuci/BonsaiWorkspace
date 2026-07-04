use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBudget {
    pub max_workers:    usize,
    pub cpu_cores:      f64,
    pub ram_mb:         u64,
    pub timeout_secs:   u64,
    pub priority:       Priority,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_workers: 2,
            cpu_cores: 1.0,
            ram_mb: 2048,
            timeout_secs: 3600,
            priority: Priority::Low,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Priority { Low, Normal, High, Critical }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FuzzStrategy {
    /// Generate random valid inputs using type-aware generation.
    InputFuzzing { iterations: u64, mutation_rate: f64 },
    /// Randomly sequence valid operations to find unexpected state combinations.
    StateFuzzing { sequences: u64, state_model: String },
    /// Stress test under resource pressure.
    ResourceExhaustion { scenarios: Vec<String> },
    /// Compare two implementations for divergence.
    Differential { baseline: String, candidate: String },
    /// Property-based testing with invariant definitions.
    PropertyBased { properties: Vec<String>, samples: u64 },
    /// Replay historical failures with variations.
    CrashReplay { failure_ids: Vec<String> },
    /// LLM-driven adversarial testing.
    Adversarial { adversary_model: String, rounds: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    TauriCommand { name: String },
    RustCrate { name: String, test_filter: Option<String> },
    WasmTool { tool_name: String },
    SylvaScript { script_path: String },
    SwarmAgent { template: String },
    TransferDaemon { lane: String },
    CrdtDocument,
    TrainingPipeline { phase: String },
    Extension { extension_id: String },
    Custom { command: String, args: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignSpec {
    pub id:           Uuid,
    pub name:         String,
    pub description:  String,
    pub targets:      Vec<TargetKind>,
    pub strategies:   Vec<FuzzStrategy>,
    pub resources:    ResourceBudget,
    pub auto_report:  bool,
    pub auto_fix:     bool,
    pub create_issue: bool,
}

impl CampaignSpec {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id:           Uuid::new_v4(),
            name:         name.into(),
            description:  String::new(),
            targets:      Vec::new(),
            strategies:   vec![FuzzStrategy::InputFuzzing { iterations: 10_000, mutation_rate: 0.05 }],
            resources:    ResourceBudget::default(),
            auto_report:  true,
            auto_fix:     false,
            create_issue: true,
        }
    }

    /// Pre-built campaign: fuzz all Tauri file system commands.
    pub fn tauri_filesystem_fuzz() -> Self {
        let mut s = Self::new("Tauri Filesystem Fuzz");
        s.description = "Fuzz all file system Tauri commands with random, malformed, and path-traversal inputs.".into();
        s.targets = vec![
            TargetKind::TauriCommand { name: "write_file".into() },
            TargetKind::TauriCommand { name: "read_file".into() },
            TargetKind::TauriCommand { name: "delete_file".into() },
            TargetKind::TauriCommand { name: "list_directory".into() },
        ];
        s.strategies = vec![
            FuzzStrategy::InputFuzzing { iterations: 100_000, mutation_rate: 0.10 },
            FuzzStrategy::PropertyBased {
                properties: vec![
                    "write_then_read_is_identity".into(),
                    "delete_makes_file_absent".into(),
                    "path_traversal_is_rejected".into(),
                ],
                samples: 10_000,
            },
        ];
        s
    }

    /// Pre-built campaign: fuzz the swarm and agent system.
    pub fn swarm_agent_fuzz() -> Self {
        let mut s = Self::new("Swarm Agent Fuzz");
        s.description = "Test swarm creation, agent messaging, and task assignment with malformed inputs.".into();
        s.targets = vec![
            TargetKind::SwarmAgent { template: "bug-fixer".into() },
            TargetKind::SwarmAgent { template: "feature-developer".into() },
            TargetKind::TauriCommand { name: "swarm_create".into() },
            TargetKind::TauriCommand { name: "swarm_send_command".into() },
        ];
        s.strategies = vec![
            FuzzStrategy::InputFuzzing { iterations: 50_000, mutation_rate: 0.15 },
            FuzzStrategy::StateFuzzing { sequences: 10_000, state_model: "swarm_lifecycle".into() },
        ];
        s
    }

    /// Pre-built campaign: CRDT concurrent operations.
    pub fn crdt_concurrency_fuzz() -> Self {
        let mut s = Self::new("CRDT Concurrency Fuzz");
        s.description = "Test CRDT document merge correctness with concurrent conflicting operations.".into();
        s.targets = vec![TargetKind::CrdtDocument];
        s.strategies = vec![
            FuzzStrategy::PropertyBased {
                properties: vec![
                    "concurrent_inserts_converge".into(),
                    "delete_beats_concurrent_insert".into(),
                    "merge_is_associative".into(),
                    "merge_is_commutative".into(),
                ],
                samples: 50_000,
            },
        ];
        s
    }

    /// Pre-built campaign: resource exhaustion stress test.
    pub fn resource_exhaustion() -> Self {
        let mut s = Self::new("Resource Exhaustion Stress");
        s.description = "Test system behaviour under extreme resource pressure.".into();
        s.targets = vec![
            TargetKind::TauriCommand { name: "run_sandboxed_code".into() },
            TargetKind::RustCrate { name: "bonsai-swarm".into(), test_filter: None },
        ];
        s.strategies = vec![
            FuzzStrategy::ResourceExhaustion {
                scenarios: vec![
                    "99_percent_memory".into(),
                    "disk_full".into(),
                    "network_1byte_per_sec".into(),
                    "cpu_starved".into(),
                ],
            },
        ];
        s
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CampaignStatus {
    Queued, Running, Paused, Completed, Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignState {
    pub spec:            CampaignSpec,
    pub status:          CampaignStatus,
    pub iterations_done: u64,
    pub crashes_found:   usize,
    pub coverage_pct:    f64,
    pub started_at:      Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at:    Option<chrono::DateTime<chrono::Utc>>,
}

impl CampaignState {
    pub fn new(spec: CampaignSpec) -> Self {
        Self {
            spec,
            status:          CampaignStatus::Queued,
            iterations_done: 0,
            crashes_found:   0,
            coverage_pct:    0.0,
            started_at:      None,
            completed_at:    None,
        }
    }
}
