//! Model Data — rich, persistent metadata for every model in the Bonsai ecosystem.
//!
//! `ModelInfo` (from `model_registry`) is auto-scanned and ephemeral.
//! `ModelData` is persistent, user-editable, and provider-agnostic — it covers
//! local GGUF models and cloud/API endpoints alike.
//!
//! Relationship:
//!   Local GGUF  →  ModelInfo (auto)  +  ModelData (persistent, links via registry_id)
//!   Cloud API   →  (no ModelInfo)    +  ModelData (standalone)

use serde::{Deserialize, Serialize};

use crate::inference_mode::InferenceMode;

// ── Source / Provider ─────────────────────────────────────────────────────────

/// Where inference for this model comes from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModelSource {
    /// A local GGUF file served through llama-server.
    LocalGguf {
        path: String,
        /// File-path hash from `ModelRegistry::stable_id()`.
        /// None if the file has been moved or not yet scanned.
        registry_id: Option<String>,
    },
    /// Any OpenAI-compatible HTTP endpoint (OpenAI, Anthropic, Groq, Ollama, etc.).
    OpenAICompatible {
        base_url:             String,
        model_id:             String,
        /// Human-readable provider name shown in the UI.
        provider_name:        String,
        /// Key name in `SecretsStore` — we never store the raw key here.
        api_key_secret_name:  Option<String>,
    },
}

impl ModelSource {
    pub fn source_type(&self) -> &'static str {
        match self {
            Self::LocalGguf { .. }        => "local_gguf",
            Self::OpenAICompatible { .. } => "openai_compatible",
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self, Self::LocalGguf { .. })
    }
}

// ── Capabilities ──────────────────────────────────────────────────────────────

/// How well the model supports tool/function calling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallingSupport {
    #[default]
    None,
    /// Single tool call per turn.
    Basic,
    /// Multiple parallel tool calls in a single turn.
    Parallel,
    /// Native vendor implementation (most reliable, e.g. Claude, GPT-4o).
    Native,
}

/// Qualitative model strengths — used for routing and UI display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelStrength {
    Coding,
    Math,
    Reasoning,
    Writing,
    Instruction,
    Multilingual,
    LongContext,
    Speed,
    Vision,
    Research,
    DataAnalysis,
}

/// Broad performance tier used by the swarm scheduler and auto-routing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    /// Best-available quality (Claude Opus, GPT-4o, Gemini Ultra).
    Frontier,
    /// High quality at lower cost (Claude Haiku, GPT-4o-mini).
    #[default]
    Capable,
    /// Optimised for throughput / low latency.
    Fast,
    /// Domain-specific capability (code, embeddings, vision).
    Specialized,
    /// Ultra-small — fits on edge hardware (<1 GB RAM).
    Embedded,
}

/// What chat-template / instruction-wrapping format the model expects.
/// Modern local models and all cloud endpoints use `OpenAIMessages`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PromptFormat {
    /// Standard role-based messages (used for cloud + modern llama.cpp).
    #[default]
    OpenAIMessages,
    ChatML,
    Llama3,
    Mistral,
    Alpaca,
    Gemma,
    Phi3,
    DeepSeek,
    Qwen2,
    CommandR,
    /// Arbitrary Jinja2-like template string.
    Custom(String),
}

impl PromptFormat {
    /// Infer likely prompt format from model family/architecture strings.
    pub fn infer_from(architecture: &str, name: &str) -> Self {
        let a = architecture.to_lowercase();
        let n = name.to_lowercase();
        if a.contains("llama") && (n.contains("llama-3") || n.contains("llama3")) {
            return Self::Llama3;
        }
        if a.contains("mistral") || a.contains("mixtral") { return Self::Mistral; }
        if a.contains("gemma")   { return Self::Gemma; }
        if a.contains("phi")     { return Self::Phi3; }
        if a.contains("qwen")    { return Self::Qwen2; }
        if a.contains("deepseek") || n.contains("deepseek") { return Self::DeepSeek; }
        if n.contains("command-r") || n.contains("command_r") { return Self::CommandR; }
        // Modern llama.cpp serves everything via the OpenAI /v1/chat/completions format.
        Self::OpenAIMessages
    }
}

/// Everything we know about what a model can do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Maximum total tokens in a single context (input + output).
    pub context_window:     u32,
    /// Maximum tokens the model can generate in one completion.
    pub max_output_tokens:  u32,

    // Input modalities
    pub accepts_images:     bool,
    pub accepts_audio:      bool,
    pub accepts_documents:  bool,

    // Output capabilities
    pub tool_calling:       ToolCallingSupport,
    /// Guaranteed valid JSON output mode (e.g. `response_format: json_object`).
    pub json_mode:          bool,
    /// Schema-constrained structured output (OpenAI `response_format: json_schema`).
    pub structured_output:  bool,
    pub streaming:          bool,

    // Reasoning
    /// Extended thinking / scratchpad reasoning (Claude 3.7+, o1-style).
    pub extended_thinking:  bool,

    /// Ordered list of qualitative strengths, most prominent first.
    pub strengths:          Vec<ModelStrength>,

    pub tier:               ModelTier,
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            context_window:    4096,
            max_output_tokens: 2048,
            accepts_images:    false,
            accepts_audio:     false,
            accepts_documents: false,
            tool_calling:      ToolCallingSupport::None,
            json_mode:         false,
            structured_output: false,
            streaming:         true,
            extended_thinking: false,
            strengths:         vec![],
            tier:              ModelTier::Capable,
        }
    }
}

impl ModelCapabilities {
    pub fn supports_tools(&self) -> bool {
        !matches!(self.tool_calling, ToolCallingSupport::None)
    }
}

// ── Inference Profile ─────────────────────────────────────────────────────────

/// Default sampling / generation parameters for this model.
/// All fields override the orchestrator's hardcoded defaults when set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceProfile {
    pub temperature:          f32,
    pub top_p:                Option<f32>,
    pub top_k:                Option<u32>,
    pub min_p:                Option<f32>,
    pub repeat_penalty:       Option<f32>,
    pub presence_penalty:     Option<f32>,
    pub frequency_penalty:    Option<f32>,
    /// Default max completion tokens; can be overridden per-request.
    pub max_tokens:           u32,
    /// Extra stop sequences beyond the model's built-in ones.
    pub stop_sequences:       Vec<String>,
    /// Text prepended to every system prompt sent to this model.
    pub system_prompt_prefix: Option<String>,
    /// Text appended to every system prompt sent to this model.
    pub system_prompt_suffix: Option<String>,
}

impl Default for InferenceProfile {
    fn default() -> Self {
        Self {
            temperature:          0.7,
            top_p:                None,
            top_k:                None,
            min_p:                None,
            repeat_penalty:       None,
            presence_penalty:     None,
            frequency_penalty:    None,
            max_tokens:           2048,
            stop_sequences:       vec![],
            system_prompt_prefix: None,
            system_prompt_suffix: None,
        }
    }
}

// ── Skill Affinity ────────────────────────────────────────────────────────────

/// How well a model performs with a specific registered tool/skill.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AffinityLevel {
    /// Actively route tasks requiring this skill to this model.
    Excellent,
    /// Works well — default for capable models.
    Good,
    /// Works but not optimal.
    Fair,
    /// Avoid if an alternative exists.
    Poor,
    /// Never pair this skill with this model.
    Incompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAffinity {
    /// Matches a tool/skill ID registered in `ToolRegistry`.
    pub skill_id: String,
    pub level:    AffinityLevel,
    /// Optional explanation — surfaced in the UI and generator output.
    pub note:     Option<String>,
}

// ── Hardware Info (local models only) ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalFileInfo {
    pub path:              String,
    pub file_size_bytes:   u64,
    pub ram_required_mb:   u64,
    pub quant_label:       String,
    /// Explicit GPU layer override. None = let orchestrator decide.
    /// -1 = all layers on GPU, 0 = CPU only.
    pub gpu_layers:        Option<i32>,
}

// ── Core ModelData ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelData {
    // ── Identity ──────────────────────────────────────────────────────────────
    /// Stable UUID — primary key in the database.
    pub id:          String,
    pub name:        String,
    /// e.g. "Llama 3", "Qwen 2.5", "Claude 3.5 Sonnet"
    pub family:      Option<String>,
    /// e.g. "3.1-8B-Instruct", "2.5-Coder-32B"
    pub version:     Option<String>,
    /// 1–3 sentence description of what this model is and what it excels at.
    pub description: String,

    // ── Source ────────────────────────────────────────────────────────────────
    pub source: ModelSource,

    // ── Rich Metadata ─────────────────────────────────────────────────────────
    pub capabilities:     ModelCapabilities,
    pub inference:        InferenceProfile,
    #[serde(default)]
    pub inference_mode:   InferenceMode,
    pub prompt_format:    PromptFormat,
    pub skill_affinities: Vec<SkillAffinity>,

    // ── Provenance ────────────────────────────────────────────────────────────
    pub authors:          Vec<String>,
    pub organization:     Option<String>,
    pub license:          Option<String>,
    pub homepage_url:     Option<String>,
    /// Approximate training data cutoff, e.g. "2024-10".
    pub training_cutoff:  Option<String>,
    pub parameter_count:  Option<u64>,
    pub architecture:     Option<String>,
    /// User-defined searchable tags.
    pub tags:             Vec<String>,
    /// Free-form notes visible in the UI.
    pub notes:            String,

    // ── Hardware (local models only) ──────────────────────────────────────────
    pub local_file: Option<LocalFileInfo>,

    // ── Timestamps (Unix ms) ──────────────────────────────────────────────────
    pub created_at: i64,
    pub updated_at: i64,
}

impl ModelData {
    /// Create a minimal ModelData from a local GGUF `ModelInfo`.
    /// Capabilities and description are left at defaults for the generator to fill.
    pub fn from_registry(info: &crate::model_registry::ModelInfo) -> Self {
        Self::from_registry_with_mode(info, InferenceMode::default())
    }

    pub fn from_registry_with_mode(
        info: &crate::model_registry::ModelInfo,
        inference_mode: InferenceMode,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        let prompt_fmt = PromptFormat::infer_from(&info.architecture, &info.name);
        let tool_support = if info.supports_tools {
            ToolCallingSupport::Basic
        } else {
            ToolCallingSupport::None
        };
        Self {
            id:           uuid::Uuid::new_v4().to_string(),
            name:         info.name.clone(),
            family:       None,
            version:      None,
            description:  String::new(),
            source: ModelSource::LocalGguf {
                path:        info.path.display().to_string(),
                registry_id: Some(info.id.clone()),
            },
            capabilities: ModelCapabilities {
                context_window:   info.context_length,
                max_output_tokens: (info.context_length / 4).max(512),
                tool_calling:     tool_support,
                streaming:        true,
                ..Default::default()
            },
            inference:        InferenceProfile::default(),
            inference_mode,
            prompt_format:    prompt_fmt,
            skill_affinities: vec![],
            authors:          vec![],
            organization:     None,
            license:          None,
            homepage_url:     None,
            training_cutoff:  None,
            parameter_count:  if info.parameter_count > 0 { Some(info.parameter_count) } else { None },
            architecture:     Some(info.architecture.clone()),
            tags:             vec![info.quant_label.clone()],
            notes:            String::new(),
            local_file: Some(LocalFileInfo {
                path:            info.path.display().to_string(),
                file_size_bytes: info.file_size_bytes,
                ram_required_mb: info.ram_required_mb,
                quant_label:     info.quant_label.clone(),
                gpu_layers:      None,
            }),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp_millis();
    }
}

// ── Summary view (used in list responses) ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDataSummary {
    pub id:           String,
    pub name:         String,
    pub family:       Option<String>,
    pub description:  String,
    pub source_type:  String,
    pub tier:         ModelTier,
    pub strengths:    Vec<ModelStrength>,
    pub context_window: u32,
    pub tool_calling: ToolCallingSupport,
    pub inference_mode: InferenceMode,
    pub tags:         Vec<String>,
    pub updated_at:   i64,
}

impl From<&ModelData> for ModelDataSummary {
    fn from(d: &ModelData) -> Self {
        Self {
            id:             d.id.clone(),
            name:           d.name.clone(),
            family:         d.family.clone(),
            description:    d.description.clone(),
            source_type:    d.source.source_type().to_string(),
            tier:           d.capabilities.tier.clone(),
            strengths:      d.capabilities.strengths.clone(),
            context_window: d.capabilities.context_window,
            tool_calling:   d.capabilities.tool_calling.clone(),
            inference_mode: d.inference_mode.clone(),
            tags:           d.tags.clone(),
            updated_at:     d.updated_at,
        }
    }
}

// ── Generator input ───────────────────────────────────────────────────────────

/// What the caller provides to kick off auto-generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GenerateModelDataInput {
    /// Auto-generate from a model already in the local registry.
    FromRegistry { registry_id: String },
    /// Auto-generate for a cloud/API model by provider + model ID.
    FromProvider  { provider: String, model_id: String, base_url: Option<String> },
}
