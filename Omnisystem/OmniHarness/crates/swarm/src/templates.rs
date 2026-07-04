use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplatePriority {
    Critical,
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmMemberTemplate {
    pub persona: String,
    pub count: usize,
    pub required_tools: Vec<String>,
    pub is_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmTemplate {
    pub name: String,
    pub description: String,
    pub leader_persona: String,
    pub members: Vec<SwarmMemberTemplate>,
    pub max_agents: usize,
    pub timeout_secs: u64,
    /// SystemEvent type names that auto-trigger this template.
    pub auto_spawn_on: Vec<String>,
    pub priority: TemplatePriority,
}

pub struct TemplateRegistry {
    templates: Arc<RwLock<HashMap<String, SwarmTemplate>>>,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        let registry = Self {
            templates: Arc::new(RwLock::new(HashMap::new())),
        };
        registry
    }

    pub async fn load_defaults(&self) {
        let mut t = self.templates.write().await;
        for template in default_templates() {
            t.insert(template.name.clone(), template);
        }
    }

    pub async fn register(&self, template: SwarmTemplate) {
        self.templates.write().await.insert(template.name.clone(), template);
    }

    pub async fn get(&self, name: &str) -> Option<SwarmTemplate> {
        self.templates.read().await.get(name).cloned()
    }

    pub async fn list_all(&self) -> Vec<SwarmTemplate> {
        self.templates.read().await.values().cloned().collect()
    }

    pub async fn for_event(&self, event_type: &str) -> Vec<SwarmTemplate> {
        self.templates
            .read().await
            .values()
            .filter(|t| t.auto_spawn_on.iter().any(|e| e == event_type))
            .cloned()
            .collect()
    }
}

fn default_templates() -> Vec<SwarmTemplate> {
    vec![
        SwarmTemplate {
            name: "ci-validation".into(),
            description: "Runs the full CI pipeline on a commit and reports results".into(),
            leader_persona: "pm-agent".into(),
            members: vec![
                SwarmMemberTemplate {
                    persona: "builder".into(), count: 1,
                    required_tools: vec!["run_cargo_check".into(), "run_cargo_build".into()],
                    is_required: true,
                },
                SwarmMemberTemplate {
                    persona: "tester".into(), count: 2,
                    required_tools: vec!["run_cargo_test".into()],
                    is_required: true,
                },
                SwarmMemberTemplate {
                    persona: "frontend-checker".into(), count: 1,
                    required_tools: vec!["run_svelte_check".into()],
                    is_required: false,
                },
            ],
            max_agents: 4,
            timeout_secs: 600,
            auto_spawn_on: vec!["ExternalPush".into(), "PRMerged".into()],
            priority: TemplatePriority::High,
        },
        SwarmTemplate {
            name: "feature-developer".into(),
            description: "Implements a feature from a natural language description with full testing".into(),
            leader_persona: "pm-agent".into(),
            members: vec![
                SwarmMemberTemplate {
                    persona: "feature-developer".into(), count: 2,
                    required_tools: vec!["read_file".into(), "write_file".into(), "run_cargo_check".into()],
                    is_required: true,
                },
                SwarmMemberTemplate {
                    persona: "tester".into(), count: 1,
                    required_tools: vec!["run_cargo_test".into()],
                    is_required: true,
                },
                SwarmMemberTemplate {
                    persona: "code-reviewer".into(), count: 1,
                    required_tools: vec!["read_file".into(), "search_codebase".into()],
                    is_required: false,
                },
            ],
            max_agents: 4,
            timeout_secs: 3600,
            auto_spawn_on: vec!["IssueCreated".into()],
            priority: TemplatePriority::Normal,
        },
        SwarmTemplate {
            name: "bug-fixer".into(),
            description: "Diagnoses and fixes bugs from test failures, crashes, or issue reports".into(),
            leader_persona: "pm-agent".into(),
            members: vec![
                SwarmMemberTemplate {
                    persona: "bug-fixer".into(), count: 2,
                    required_tools: vec!["read_file".into(), "write_file".into(), "run_cargo_test".into()],
                    is_required: true,
                },
                SwarmMemberTemplate {
                    persona: "tester".into(), count: 1,
                    required_tools: vec!["run_cargo_test".into()],
                    is_required: true,
                },
            ],
            max_agents: 3,
            timeout_secs: 1800,
            auto_spawn_on: vec!["TestFailed".into(), "BuildFailed".into(), "CrashDetected".into()],
            priority: TemplatePriority::Critical,
        },
        SwarmTemplate {
            name: "security-audit".into(),
            description: "Reviews code, extensions, or dependencies for security vulnerabilities".into(),
            leader_persona: "security-lead".into(),
            members: vec![
                SwarmMemberTemplate {
                    persona: "security-auditor".into(), count: 2,
                    required_tools: vec!["read_file".into(), "search_codebase".into()],
                    is_required: true,
                },
            ],
            max_agents: 3,
            timeout_secs: 3600,
            auto_spawn_on: vec!["ExtensionInstalled".into(), "SecurityReviewRequested".into()],
            priority: TemplatePriority::High,
        },
        SwarmTemplate {
            name: "deployment".into(),
            description: "Monitors a deployment with health checking and auto-rollback".into(),
            leader_persona: "deployment-lead".into(),
            members: vec![
                SwarmMemberTemplate {
                    persona: "health-checker".into(), count: 1,
                    required_tools: vec!["health_check".into()],
                    is_required: true,
                },
            ],
            max_agents: 2,
            timeout_secs: 300,
            auto_spawn_on: vec!["UpgradeReady".into()],
            priority: TemplatePriority::Critical,
        },
        SwarmTemplate {
            name: "training-pipeline".into(),
            description: "Runs the full model training pipeline with evaluation gates".into(),
            leader_persona: "training-manager".into(),
            members: vec![
                SwarmMemberTemplate {
                    persona: "data-curator".into(), count: 1,
                    required_tools: vec!["export_training_data".into()],
                    is_required: true,
                },
                SwarmMemberTemplate {
                    persona: "trainer".into(), count: 2,
                    required_tools: vec!["run_dpo".into(), "run_sft".into()],
                    is_required: true,
                },
                SwarmMemberTemplate {
                    persona: "evaluator".into(), count: 1,
                    required_tools: vec!["evaluate_model".into()],
                    is_required: true,
                },
            ],
            max_agents: 4,
            timeout_secs: 86400,
            auto_spawn_on: vec!["TrainingScheduled".into()],
            priority: TemplatePriority::Normal,
        },
        SwarmTemplate {
            name: "dream-cycle".into(),
            description: "Nightly memory consolidation via DreamAgent".into(),
            leader_persona: "dream-agent".into(),
            members: vec![
                SwarmMemberTemplate {
                    persona: "memory-consolidator".into(), count: 1,
                    required_tools: vec!["read_memory_nodes".into(), "consolidate_memory".into()],
                    is_required: true,
                },
            ],
            max_agents: 2,
            timeout_secs: 1800,
            auto_spawn_on: vec!["DreamCycleScheduled".into()],
            priority: TemplatePriority::Low,
        },
    ]
}
