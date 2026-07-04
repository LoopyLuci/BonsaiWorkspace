//! Extension manifest — the `bonsai-extension.yaml` / `extension.json` schema.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use semver::Version;

/// Top-level extension manifest. Serialised as `bonsai-extension.yaml` inside the repo root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    /// Reverse-domain identifier e.g. `com.example.my-tool`.
    pub extension_id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: AuthorInfo,
    pub license: String,
    pub repository: String,
    pub category: ExtensionCategory,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub screenshots: Vec<String>,

    #[serde(default)]
    pub dependencies: ExtensionDependencies,
    pub entry_points: EntryPoints,
    pub permissions: ExtensionPermissions,

    /// User-configurable parameters (generates the Settings form automatically).
    #[serde(default)]
    pub config_schema: HashMap<String, ConfigField>,

    /// Minimum Bonsai version required.
    #[serde(default)]
    pub min_bonsai_version: Option<String>,

    /// Filled after security review. None until reviewed.
    #[serde(default)]
    pub security_review: Option<SecurityReviewStub>,
}

impl ExtensionManifest {
    /// Parse a semver `version` string or return an error.
    pub fn parsed_version(&self) -> Result<Version, semver::Error> {
        self.version.parse()
    }

    /// Validate required fields.
    pub fn validate(&self) -> Result<(), String> {
        if self.extension_id.is_empty() {
            return Err("extension_id is required".into());
        }
        if self.name.is_empty() {
            return Err("name is required".into());
        }
        self.parsed_version().map_err(|e| format!("invalid version: {e}"))?;
        if self.repository.is_empty() {
            return Err("repository is required".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthorInfo {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub peer_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionCategory {
    Agent,
    Tool,
    UiPanel,
    Theme,
    TrainingRecipe,
    SurvivalRule,
    ComputeTask,
    TuiPlugin,
    Workflow,
    KnowledgeModule,
    Other,
}

impl std::fmt::Display for ExtensionCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtensionCategory::Agent => write!(f, "Agent"),
            ExtensionCategory::Tool => write!(f, "Tool"),
            ExtensionCategory::UiPanel => write!(f, "UI Panel"),
            ExtensionCategory::Theme => write!(f, "Theme"),
            ExtensionCategory::TrainingRecipe => write!(f, "Training Recipe"),
            ExtensionCategory::SurvivalRule => write!(f, "Survival Rule"),
            ExtensionCategory::ComputeTask => write!(f, "Compute Task"),
            ExtensionCategory::TuiPlugin => write!(f, "TUI Plugin"),
            ExtensionCategory::Workflow => write!(f, "Workflow"),
            ExtensionCategory::KnowledgeModule => write!(f, "Knowledge Module"),
            ExtensionCategory::Other => write!(f, "Other"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtensionDependencies {
    /// Minimum Bonsai version (semver range).
    #[serde(default)]
    pub bonsai: Option<String>,
    /// Other extensions required: `{extension_id: semver_req}`.
    #[serde(default)]
    pub extensions: HashMap<String, String>,
}

/// Paths (relative to repo root) for each asset type.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EntryPoints {
    #[serde(default)]
    pub agent_personas: Vec<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub ui_panels: Vec<String>,
    #[serde(default)]
    pub training_recipes: Vec<String>,
    #[serde(default)]
    pub survival_rules: Vec<String>,
    #[serde(default)]
    pub compute_tasks: Vec<String>,
    #[serde(default)]
    pub themes: Vec<String>,
    #[serde(default)]
    pub tui_plugins: Vec<String>,
    #[serde(default)]
    pub workflows: Vec<String>,
    #[serde(default)]
    pub knowledge_modules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtensionPermissions {
    /// "read_only" | "read_write" | "none"
    #[serde(default)]
    pub file_access: FileAccessLevel,
    /// "whitelist" | "none" | "internet"
    #[serde(default)]
    pub network: NetworkAccessLevel,
    /// Allowed hostnames when `network = "whitelist"`.
    #[serde(default)]
    pub network_domains: Vec<String>,
    /// Tool names the extension is allowed to invoke.
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub max_resources: ResourceLimits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FileAccessLevel {
    #[default]
    None,
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NetworkAccessLevel {
    #[default]
    None,
    Whitelist,
    Internet,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceLimits {
    #[serde(default = "default_cpu")]
    pub cpu_cores: f32,
    #[serde(default = "default_ram")]
    pub memory_mb: u32,
    #[serde(default)]
    pub gpu_required: bool,
}

fn default_cpu() -> f32 { 1.0 }
fn default_ram() -> u32 { 256 }

/// A single user-configurable parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    pub label: String,
    pub description: String,
    #[serde(rename = "type")]
    pub field_type: ConfigFieldType,
    pub default: serde_json::Value,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub validator: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigFieldType {
    Boolean,
    Slider,
    Enum,
    String,
    Number,
    Color,
}

/// Minimal security review stub embedded in the manifest after review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReviewStub {
    pub reviewed_by: String,
    pub review_date: chrono::DateTime<chrono::Utc>,
    pub content_hash: String,
    pub verdict: SecurityVerdict,
    pub risk_score: u8,
    /// CID/hash pointing to the full report.
    #[serde(default)]
    pub report_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityVerdict {
    Safe,
    Caution,
    Risky,
    Blocked,
    Unreviewed,
}

impl std::fmt::Display for SecurityVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityVerdict::Safe => write!(f, "🟢 Safe"),
            SecurityVerdict::Caution => write!(f, "🟡 Caution"),
            SecurityVerdict::Risky => write!(f, "🔴 Risky"),
            SecurityVerdict::Blocked => write!(f, "⛔ Blocked"),
            SecurityVerdict::Unreviewed => write!(f, "⚪ Not Reviewed"),
        }
    }
}
