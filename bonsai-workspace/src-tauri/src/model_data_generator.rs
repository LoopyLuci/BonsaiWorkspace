//! AI-powered Model Data generator.
//!
//! Given a registry entry or a provider + model ID, this module:
//!   1. Consults a built-in knowledge base of well-known models.
//!   2. Runs an inference pass through the active local model to fill any gaps.
//!   3. Returns a fully-populated draft `ModelData` for user review.
//!
//! The knowledge base is kept in-process as a static map rather than a file to
//! avoid asset-loading complexity on all platforms.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use serde_json::{json, Value};

use crate::model_data::{
    AffinityLevel, InferenceProfile, LocalFileInfo, ModelCapabilities, ModelData,
    ModelSource, ModelStrength, ModelTier, PromptFormat, SkillAffinity,
    ToolCallingSupport,
};
use crate::model_orchestrator::ModelOrchestrator;
use crate::model_registry::ModelInfo;

// ── Built-in knowledge base ───────────────────────────────────────────────────

/// Seed entry used to pre-populate ModelData for known model families.
/// Fields left as None are filled by the LLM inference pass.
#[derive(Clone)]
struct KbEntry {
    family:           &'static str,
    organization:     &'static str,
    description:      &'static str,
    license:          &'static str,
    homepage_url:     &'static str,
    training_cutoff:  Option<&'static str>,
    strengths:        &'static [ModelStrength],
    tier:             ModelTier,
    tool_calling:     ToolCallingSupport,
    prompt_format:    PromptFormat,
    json_mode:        bool,
    extended_thinking: bool,
    skill_affinities: &'static [(&'static str, AffinityLevel)],
}

// Build the knowledge base keyed on lowercase family/architecture patterns.
fn knowledge_base() -> HashMap<&'static str, KbEntry> {
    use ModelStrength::*;
    use AffinityLevel::*;

    macro_rules! kb {
        ($key:expr => $entry:expr) => { ($key, $entry) };
    }

    HashMap::from([
        kb!("llama" => KbEntry {
            family:           "Llama 3",
            organization:     "Meta",
            description:      "Meta's Llama 3 family is a strong open-weights general-purpose model with excellent instruction-following, reasoning, and coding skills. Llama 3.1+ supports very long contexts and competitive tool-calling.",
            license:          "Llama 3 Community License",
            homepage_url:     "https://llama.meta.com/",
            training_cutoff:  Some("2023-12"),
            strengths:        &[Coding, Instruction, Reasoning],
            tier:             ModelTier::Capable,
            tool_calling:     ToolCallingSupport::Parallel,
            prompt_format:    PromptFormat::Llama3,
            json_mode:        true,
            extended_thinking: false,
            skill_affinities: &[
                ("read_file", Good),
                ("write_file", Good),
                ("run_shell", Fair),
                ("search_web", Good),
            ],
        }),
        kb!("qwen" => KbEntry {
            family:           "Qwen 2.5",
            organization:     "Alibaba Cloud",
            description:      "Alibaba's Qwen 2.5 family delivers exceptional coding, math, and multilingual capabilities. The Coder variant is among the strongest open-weights coding models available.",
            license:          "Qwen License",
            homepage_url:     "https://qwenlm.github.io/",
            training_cutoff:  Some("2024-06"),
            strengths:        &[Coding, Math, Multilingual, Reasoning],
            tier:             ModelTier::Capable,
            tool_calling:     ToolCallingSupport::Parallel,
            prompt_format:    PromptFormat::Qwen2,
            json_mode:        true,
            extended_thinking: false,
            skill_affinities: &[
                ("read_file", Excellent),
                ("write_file", Excellent),
                ("run_shell", Good),
                ("search_knowledge", Good),
            ],
        }),
        kb!("mistral" => KbEntry {
            family:           "Mistral",
            organization:     "Mistral AI",
            description:      "Mistral's models are highly efficient instruction followers with strong reasoning and European multilingual coverage. Mixtral MoE variants punch far above their activated-parameter weight.",
            license:          "Apache 2.0",
            homepage_url:     "https://mistral.ai/",
            training_cutoff:  Some("2024-01"),
            strengths:        &[Instruction, Reasoning, Multilingual],
            tier:             ModelTier::Capable,
            tool_calling:     ToolCallingSupport::Basic,
            prompt_format:    PromptFormat::Mistral,
            json_mode:        true,
            extended_thinking: false,
            skill_affinities: &[
                ("read_file", Good),
                ("search_web", Good),
            ],
        }),
        kb!("deepseek" => KbEntry {
            family:           "DeepSeek",
            organization:     "DeepSeek AI",
            description:      "DeepSeek models are frontier-quality reasoning and coding models from a Chinese AI lab. DeepSeek-R1 rivals o1-level reasoning; the Coder variant leads open-weights coding benchmarks.",
            license:          "DeepSeek License",
            homepage_url:     "https://www.deepseek.com/",
            training_cutoff:  Some("2024-07"),
            strengths:        &[Coding, Math, Reasoning, Research],
            tier:             ModelTier::Frontier,
            tool_calling:     ToolCallingSupport::Basic,
            prompt_format:    PromptFormat::DeepSeek,
            json_mode:        true,
            extended_thinking: true,
            skill_affinities: &[
                ("read_file", Excellent),
                ("write_file", Excellent),
                ("run_shell", Good),
                ("search_knowledge", Excellent),
            ],
        }),
        kb!("gemma" => KbEntry {
            family:           "Gemma",
            organization:     "Google DeepMind",
            description:      "Google's Gemma family are compact, efficient models with strong instruction-following and safety alignment. Gemma 2 shows excellent performance-per-parameter.",
            license:          "Gemma Terms of Use",
            homepage_url:     "https://ai.google.dev/gemma",
            training_cutoff:  Some("2024-04"),
            strengths:        &[Instruction, Speed, Reasoning],
            tier:             ModelTier::Fast,
            tool_calling:     ToolCallingSupport::Basic,
            prompt_format:    PromptFormat::Gemma,
            json_mode:        false,
            extended_thinking: false,
            skill_affinities: &[
                ("get_datetime", Excellent),
                ("get_system_stats", Good),
            ],
        }),
        kb!("phi" => KbEntry {
            family:           "Phi",
            organization:     "Microsoft",
            description:      "Microsoft's Phi series are small-but-mighty models that outperform much larger models on reasoning and coding tasks. Phi-3 and Phi-4 achieve near-frontier quality at a fraction of the size.",
            license:          "MIT",
            homepage_url:     "https://azure.microsoft.com/en-us/products/phi",
            training_cutoff:  Some("2024-08"),
            strengths:        &[Reasoning, Coding, Speed, Instruction],
            tier:             ModelTier::Fast,
            tool_calling:     ToolCallingSupport::Basic,
            prompt_format:    PromptFormat::Phi3,
            json_mode:        true,
            extended_thinking: false,
            skill_affinities: &[
                ("read_file", Good),
                ("run_shell", Fair),
            ],
        }),
        kb!("granite" => KbEntry {
            family:           "Granite",
            organization:     "IBM Research",
            description:      "IBM's Granite models are enterprise-focused with strong instruction-following, code generation, and data analysis capabilities. Well-suited for structured business tasks.",
            license:          "Apache 2.0",
            homepage_url:     "https://www.ibm.com/granite",
            training_cutoff:  Some("2024-05"),
            strengths:        &[Coding, Instruction, DataAnalysis],
            tier:             ModelTier::Capable,
            tool_calling:     ToolCallingSupport::Basic,
            prompt_format:    PromptFormat::OpenAIMessages,
            json_mode:        true,
            extended_thinking: false,
            skill_affinities: &[
                ("read_file", Excellent),
                ("search_knowledge", Good),
                ("write_file", Good),
            ],
        }),
        kb!("bonsai" => KbEntry {
            family:           "Bonsai",
            organization:     "Prism ML",
            description:      "Bonsai is an ultra-efficient 1.7B model purpose-built for the Bonsai Workspace. Uses BitNet-inspired quantization for minimal RAM usage and near-instant responses — ideal for quick questions, simple code generation, and rapid iteration.",
            license:          "Bonsai Community License",
            homepage_url:     "https://huggingface.co/prism-ml",
            training_cutoff:  Some("2024-06"),
            strengths:        &[Instruction, Speed, Coding],
            tier:             ModelTier::Embedded,
            tool_calling:     ToolCallingSupport::Basic,
            prompt_format:    PromptFormat::Llama3,
            json_mode:        false,
            extended_thinking: false,
            skill_affinities: &[
                ("get_datetime", Excellent),
                ("get_system_stats", Good),
                ("read_file", Good),
            ],
        }),
        kb!("qwen3.6-35b-a3b" => KbEntry {
            family:           "Qwen 3 MoE",
            organization:     "Alibaba Cloud / Community",
            description:      "A 35B Mixture-of-Experts model with only ~3B parameters active per token, distilled from Claude 4.7 Opus reasoning traces. Delivers frontier-quality reasoning and coding at 3B compute cost. The APEX-I series applies advanced post-training for sharper instruction following.",
            license:          "Qwen License",
            homepage_url:     "https://huggingface.co/Qwen",
            training_cutoff:  Some("2025-01"),
            strengths:        &[Reasoning, Coding, Math, Instruction],
            tier:             ModelTier::Frontier,
            tool_calling:     ToolCallingSupport::Parallel,
            prompt_format:    PromptFormat::Qwen2,
            json_mode:        true,
            extended_thinking: true,
            skill_affinities: &[
                ("read_file", Excellent),
                ("write_file", Excellent),
                ("run_shell", Excellent),
                ("search_knowledge", Excellent),
                ("write_code", Excellent),
            ],
        }),
        kb!("gemma-4" => KbEntry {
            family:           "Gemma 4",
            organization:     "Google DeepMind",
            description:      "Google's Gemma 4 31B instruction-tuned model. Strong multilingual reasoning and 128K context window. The Q2_K_XL quantization from Unsloth preserves quality in a memory-efficient format.",
            license:          "Gemma Terms of Use",
            homepage_url:     "https://ai.google.dev/gemma",
            training_cutoff:  Some("2025-01"),
            strengths:        &[Reasoning, Instruction, Multilingual, LongContext],
            tier:             ModelTier::Capable,
            tool_calling:     ToolCallingSupport::Basic,
            // Gemma 4 uses the standard OpenAI messages format, not the legacy <start_of_turn> template
            prompt_format:    PromptFormat::OpenAIMessages,
            json_mode:        true,
            extended_thinking: false,
            skill_affinities: &[
                ("read_file", Excellent),
                ("search_knowledge", Excellent),
                ("search_web", Good),
                ("write_file", Good),
            ],
        }),
        kb!("gliese" => KbEntry {
            family:           "Gliese",
            organization:     "Community",
            description:      "A sub-1B Qwen model fine-tuned and abliterated (uncensored) for image captioning tasks. Ultra-fast with minimal RAM — best used for quick caption generation and rapid simple responses where speed matters most.",
            license:          "Apache 2.0",
            homepage_url:     "https://huggingface.co/",
            training_cutoff:  None,
            strengths:        &[Speed, Vision],
            tier:             ModelTier::Embedded,
            tool_calling:     ToolCallingSupport::None,
            prompt_format:    PromptFormat::Qwen2,
            json_mode:        false,
            extended_thinking: false,
            skill_affinities: &[
                ("get_datetime", Excellent),
            ],
        }),
        kb!("command-r" => KbEntry {
            family:           "Command R",
            organization:     "Cohere",
            description:      "Cohere's Command R family is optimised for retrieval-augmented generation and enterprise tool use. Excellent at document Q&A and multi-step agentic tasks.",
            license:          "CC BY-NC 4.0",
            homepage_url:     "https://cohere.com/command",
            training_cutoff:  Some("2024-03"),
            strengths:        &[Research, Instruction, LongContext],
            tier:             ModelTier::Capable,
            tool_calling:     ToolCallingSupport::Parallel,
            prompt_format:    PromptFormat::CommandR,
            json_mode:        true,
            extended_thinking: false,
            skill_affinities: &[
                ("search_knowledge", Excellent),
                ("search_web", Excellent),
                ("read_file", Excellent),
            ],
        }),
        // ── Cloud models ──────────────────────────────────────────────────────
        kb!("claude" => KbEntry {
            family:           "Claude",
            organization:     "Anthropic",
            description:      "Anthropic's Claude models lead on safety, instruction-following, and long-context analysis. Claude 3.5+ and Claude 4 achieve top scores on coding, reasoning, and agentic tasks.",
            license:          "Anthropic API Terms",
            homepage_url:     "https://www.anthropic.com/claude",
            training_cutoff:  Some("2024-08"),
            strengths:        &[Reasoning, Coding, Writing, Instruction, LongContext, Research],
            tier:             ModelTier::Frontier,
            tool_calling:     ToolCallingSupport::Native,
            prompt_format:    PromptFormat::OpenAIMessages,
            json_mode:        true,
            extended_thinking: true,
            skill_affinities: &[
                ("read_file", Excellent),
                ("write_file", Excellent),
                ("run_shell", Excellent),
                ("search_knowledge", Excellent),
                ("search_web", Excellent),
                ("send_email", Excellent),
                ("write_code", Excellent),
            ],
        }),
        kb!("gpt" => KbEntry {
            family:           "GPT",
            organization:     "OpenAI",
            description:      "OpenAI's GPT family sets the standard for general-purpose AI assistants. GPT-4o offers multimodal input, structured output, and native function calling at high throughput.",
            license:          "OpenAI API Terms",
            homepage_url:     "https://openai.com/",
            training_cutoff:  Some("2024-04"),
            strengths:        &[Reasoning, Coding, Writing, Vision, Instruction],
            tier:             ModelTier::Frontier,
            tool_calling:     ToolCallingSupport::Native,
            prompt_format:    PromptFormat::OpenAIMessages,
            json_mode:        true,
            extended_thinking: false,
            skill_affinities: &[
                ("read_file", Excellent),
                ("write_file", Excellent),
                ("run_shell", Excellent),
                ("search_web", Excellent),
                ("write_code", Excellent),
            ],
        }),
        kb!("gemini" => KbEntry {
            family:           "Gemini",
            organization:     "Google DeepMind",
            description:      "Google's Gemini Ultra and Pro models offer very long context windows (up to 1M tokens), multimodal input, and strong reasoning. Excellent for document analysis and research workflows.",
            license:          "Google API Terms",
            homepage_url:     "https://gemini.google.com/",
            training_cutoff:  Some("2024-06"),
            strengths:        &[LongContext, Vision, Research, Reasoning, Multilingual],
            tier:             ModelTier::Frontier,
            tool_calling:     ToolCallingSupport::Native,
            prompt_format:    PromptFormat::OpenAIMessages,
            json_mode:        true,
            extended_thinking: false,
            skill_affinities: &[
                ("search_knowledge", Excellent),
                ("search_web", Excellent),
                ("read_file", Excellent),
            ],
        }),
    ])
}

// ── Generator ─────────────────────────────────────────────────────────────────

pub struct ModelDataGenerator {
    orchestrator: Arc<ModelOrchestrator>,
    kb:           HashMap<&'static str, KbEntry>,
}

impl ModelDataGenerator {
    pub fn new(orchestrator: Arc<ModelOrchestrator>) -> Self {
        Self { orchestrator, kb: knowledge_base() }
    }

    /// Auto-generate ModelData for a local GGUF entry from the registry.
    pub async fn from_registry_info(&self, info: &ModelInfo) -> Result<ModelData> {
        let mut data = ModelData::from_registry(info);

        // Give the model a clean display name.
        let clean = clean_display_name(&info.name);
        if !clean.is_empty() { data.name = clean; }

        // 1. Apply knowledge base if we recognise the model family.
        self.apply_knowledge_base(&mut data, &info.name, &info.architecture);

        // 2. Use LLM to fill in description + any remaining gaps.
        if let Err(e) = self.enrich_via_llm(&mut data).await {
            tracing::warn!("[model-data-gen] LLM enrichment failed for '{}': {e}", info.name);
            // Non-fatal — return what we have from the knowledge base.
        }

        data.touch();
        Ok(data)
    }

    /// Auto-generate ModelData for a cloud / API model.
    pub async fn from_provider(
        &self,
        provider: &str,
        model_id: &str,
        base_url: Option<&str>,
    ) -> Result<ModelData> {
        let now = chrono::Utc::now().timestamp_millis();
        let provider_lower = provider.to_lowercase();
        let model_lower    = model_id.to_lowercase();

        // Infer a human-readable name from the model_id.
        let name = model_id_to_display_name(model_id);

        let base = base_url.unwrap_or_else(|| default_base_url(&provider_lower));

        let mut data = ModelData {
            id:          uuid::Uuid::new_v4().to_string(),
            name:        name.clone(),
            family:      None,
            version:     None,
            description: String::new(),
            source: ModelSource::OpenAICompatible {
                base_url:            base.to_string(),
                model_id:            model_id.to_string(),
                provider_name:       capitalise(&provider_lower),
                api_key_secret_name: Some(format!("{}_api_key", provider_lower)),
            },
            capabilities: ModelCapabilities {
                context_window:   128_000,
                max_output_tokens: 8_192,
                streaming:        true,
                ..Default::default()
            },
            inference:        InferenceProfile::default(),
            inference_mode:   crate::inference_mode::InferenceMode::default(),
            prompt_format:    PromptFormat::OpenAIMessages,
            skill_affinities: vec![],
            authors:          vec![],
            organization:     None,
            license:          None,
            homepage_url:     None,
            training_cutoff:  None,
            parameter_count:  None,
            architecture:     None,
            tags:             vec![provider.to_string()],
            notes:            String::new(),
            local_file:       None,
            created_at:       now,
            updated_at:       now,
        };

        // Apply knowledge base using both provider name and model_id as hints.
        self.apply_knowledge_base(&mut data, &model_lower, &provider_lower);

        // Refine context window for well-known model IDs.
        apply_known_context_overrides(&mut data, model_id);

        if let Err(e) = self.enrich_via_llm(&mut data).await {
            tracing::warn!("[model-data-gen] LLM enrichment failed for '{model_id}': {e}");
        }

        data.touch();
        Ok(data)
    }

    // ── Private ───────────────────────────────────────────────────────────────

    fn apply_knowledge_base(&self, data: &mut ModelData, name: &str, arch: &str) {
        let haystack = format!("{} {}", name.to_lowercase(), arch.to_lowercase());

        // Find the best matching knowledge base key.
        let best = self.kb.iter().max_by_key(|(key, _)| {
            if haystack.contains(*key) { key.len() } else { 0 }
        });

        let Some((_, entry)) = best.filter(|(key, _)| haystack.contains(*key)) else {
            return;
        };

        // Fill in fields that are still blank.
        if data.family.is_none() {
            data.family = Some(entry.family.to_string());
        }
        if data.description.is_empty() {
            data.description = entry.description.to_string();
        }
        if data.organization.is_none() {
            data.organization = Some(entry.organization.to_string());
        }
        if data.license.is_none() {
            data.license = Some(entry.license.to_string());
        }
        if data.homepage_url.is_none() {
            data.homepage_url = Some(entry.homepage_url.to_string());
        }
        if data.training_cutoff.is_none() {
            data.training_cutoff = entry.training_cutoff.map(|s| s.to_string());
        }
        if data.capabilities.strengths.is_empty() {
            data.capabilities.strengths = entry.strengths.to_vec();
        }
        data.capabilities.tier              = entry.tier.clone();
        data.capabilities.tool_calling      = entry.tool_calling.clone();
        data.capabilities.json_mode         = entry.json_mode;
        data.capabilities.extended_thinking  = entry.extended_thinking;
        data.prompt_format                   = entry.prompt_format.clone();

        // Merge skill affinities (don't overwrite any the user already set).
        let existing_ids: Vec<String> = data.skill_affinities.iter()
            .map(|a| a.skill_id.clone())
            .collect();
        for (skill_id, level) in entry.skill_affinities {
            if !existing_ids.iter().any(|id| id == skill_id) {
                data.skill_affinities.push(SkillAffinity {
                    skill_id: skill_id.to_string(),
                    level:    level.clone(),
                    note:     None,
                });
            }
        }
    }

    /// Use the active local model to generate a description and validate the
    /// capability assessment, returning the enriched ModelData.
    async fn enrich_via_llm(&self, data: &mut ModelData) -> Result<()> {
        // Only run if a model slot is ready — don't block startup.
        if self.orchestrator.active_slot_url().await.is_none() {
            return Ok(());
        }

        let prompt = build_enrichment_prompt(data);
        let (response, _stats) = self.orchestrator.infer_simple(&prompt, 512, "model-gen").await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        // Parse JSON from the response (may be wrapped in ```json...```).
        let json_str = extract_json(&response);
        let Ok(val)  = serde_json::from_str::<Value>(&json_str) else {
            return Ok(());   // Gracefully ignore unparseable LLM output.
        };

        // Apply only non-empty / non-null fields so we don't overwrite KB data.
        if data.description.is_empty() {
            if let Some(s) = val["description"].as_str().filter(|s| !s.is_empty()) {
                data.description = s.to_string();
            }
        }
        if data.family.is_none() {
            if let Some(s) = val["family"].as_str().filter(|s| !s.is_empty()) {
                data.family = Some(s.to_string());
            }
        }
        if data.version.is_none() {
            if let Some(s) = val["version"].as_str().filter(|s| !s.is_empty()) {
                data.version = Some(s.to_string());
            }
        }
        if data.organization.is_none() {
            if let Some(s) = val["organization"].as_str().filter(|s| !s.is_empty()) {
                data.organization = Some(s.to_string());
            }
        }
        if data.training_cutoff.is_none() {
            if let Some(s) = val["training_cutoff"].as_str().filter(|s| !s.is_empty()) {
                data.training_cutoff = Some(s.to_string());
            }
        }

        // Strengths — only override if the response is non-empty and KB gave nothing.
        if data.capabilities.strengths.is_empty() {
            if let Some(arr) = val["strengths"].as_array() {
                let strengths: Vec<ModelStrength> = arr.iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| parse_strength(s))
                    .collect();
                if !strengths.is_empty() {
                    data.capabilities.strengths = strengths;
                }
            }
        }

        // Tier — only override from LLM if KB did not recognise the model.
        if matches!(data.capabilities.tier, ModelTier::Capable) {
            if let Some(s) = val["tier"].as_str() {
                if let Some(tier) = parse_tier(s) {
                    data.capabilities.tier = tier;
                }
            }
        }

        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_enrichment_prompt(data: &ModelData) -> String {
    let name    = &data.name;
    let arch    = data.architecture.as_deref().unwrap_or("unknown");
    let family  = data.family.as_deref().unwrap_or("unknown");
    let params  = data.parameter_count.map(|p| format!("{:.1}B", p as f64 / 1e9)).unwrap_or_else(|| "unknown".into());
    let ctx     = data.capabilities.context_window;
    let quant   = data.local_file.as_ref().map(|lf| lf.quant_label.as_str()).unwrap_or("N/A");
    let source_type = data.source.source_type();

    format!(r#"You are an AI model metadata expert. Given this information about an AI model, return a JSON object with the fields below.

Model name: {name}
Architecture: {arch}
Family: {family}
Parameter count: {params}
Context window: {ctx} tokens
Quantization: {quant}
Source type: {source_type}

Return ONLY valid JSON with these fields:
{{
  "description": "1-2 clear sentences about what this model is and what it excels at",
  "family": "model family name (e.g. Llama 3, Qwen 2.5, Claude 3.5)",
  "version": "version or variant name (e.g. 3.1-8B-Instruct)",
  "organization": "organization that created this model",
  "training_cutoff": "approximate training cutoff in YYYY-MM format, or empty string if unknown",
  "strengths": ["array", "of", "strengths", "from: coding|math|reasoning|writing|instruction|multilingual|long_context|speed|vision|research|data_analysis"],
  "tier": "one of: frontier|capable|fast|specialized|embedded"
}}

Respond with the JSON object only."#)
}

fn extract_json(s: &str) -> String {
    // Strip markdown code fences if present.
    let stripped = s.trim();
    let stripped = stripped.strip_prefix("```json").unwrap_or(stripped);
    let stripped = stripped.strip_prefix("```").unwrap_or(stripped);
    let stripped = stripped.strip_suffix("```").unwrap_or(stripped);
    // Find the first '{' and last '}'.
    if let (Some(start), Some(end)) = (stripped.find('{'), stripped.rfind('}')) {
        stripped[start..=end].to_string()
    } else {
        stripped.to_string()
    }
}

fn parse_strength(s: &str) -> Option<ModelStrength> {
    match s {
        "coding"        => Some(ModelStrength::Coding),
        "math"          => Some(ModelStrength::Math),
        "reasoning"     => Some(ModelStrength::Reasoning),
        "writing"       => Some(ModelStrength::Writing),
        "instruction"   => Some(ModelStrength::Instruction),
        "multilingual"  => Some(ModelStrength::Multilingual),
        "long_context"  => Some(ModelStrength::LongContext),
        "speed"         => Some(ModelStrength::Speed),
        "vision"        => Some(ModelStrength::Vision),
        "research"      => Some(ModelStrength::Research),
        "data_analysis" => Some(ModelStrength::DataAnalysis),
        _               => None,
    }
}

fn parse_tier(s: &str) -> Option<ModelTier> {
    match s {
        "frontier"    => Some(ModelTier::Frontier),
        "capable"     => Some(ModelTier::Capable),
        "fast"        => Some(ModelTier::Fast),
        "specialized" => Some(ModelTier::Specialized),
        "embedded"    => Some(ModelTier::Embedded),
        _             => None,
    }
}

fn model_id_to_display_name(model_id: &str) -> String {
    let parts: Vec<&str> = model_id.split('-').collect();
    let capitalised: Vec<String> = parts.iter().map(|p| {
        let mut chars = p.chars();
        match chars.next() {
            None    => String::new(),
            Some(c) => c.to_uppercase().to_string() + chars.as_str(),
        }
    }).collect();
    capitalised.join(" ")
}

/// Produce a clean human-readable name from a raw GGUF filename stem.
/// Strips quantization suffixes, normalises separators, and applies
/// known model-family rewrites for a polished display label.
pub fn clean_display_name(raw: &str) -> String {
    // Known specific rewrites (match substring, most-specific first).
    let rewrites: &[(&str, &str)] = &[
        ("Qwen3.6-35B-A3B-Claude-4.7-Opus-Reasoning-Distilled-APEX-I-Quality",
            "Qwen 3 35B MoE · APEX-I Quality (Claude Opus Distilled)"),
        ("Qwen3.6-35B-A3B-Claude-4.7-Opus-Reasoning-Distilled-APEX-I-Compact",
            "Qwen 3 35B MoE · APEX-I Compact (Claude Opus Distilled)"),
        ("Qwen3.6-35B-A3B-Claude-4.7-Opus-Reasoning-Distilled-APEX-I",
            "Qwen 3 35B MoE · APEX-I (Claude Opus Distilled)"),
        ("Gliese-Qwen3.5-0.8B-Abliterated-Caption", "Gliese 0.8B Caption (Abliterated)"),
        ("Qwen3.5-0.8B-UD-IQ2_XXS",   "Qwen 2.5 0.8B · IQ2_XXS"),
        ("gemma-4-31B-it-UD-Q2_K_XL",  "Gemma 4 31B IT · Q2_K_XL"),
        ("Bonsai-1.7B-IQ1_S",          "Bonsai 1.7B · IQ1_S"),
        ("Bonsai-1.7B-Q2_K",           "Bonsai 1.7B · Q2_K"),
        ("Bonsai-1.7B-TQ1_0",          "Bonsai 1.7B · TQ1_0"),
        ("Bonsai-1.7B-TQ2_0",          "Bonsai 1.7B · TQ2_0"),
    ];
    for (pattern, label) in rewrites {
        if raw.contains(pattern) { return label.to_string(); }
    }

    // Generic fallback: strip trailing quant token(s) and clean separators.
    // Pattern: anything matching known quant suffixes at the end.
    let stripped = regex::Regex::new(
        r"[-_](?:IQ\d+_\w+|Q\d+[_K]\w*|TQ\d+_\d+|UD-\w+|i1-\w+|gguf)$"
    )
    .ok()
    .map(|re| re.replace(raw, "").to_string())
    .unwrap_or_else(|| raw.to_string());

    stripped
        .replace('-', " ")
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None    => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

fn default_base_url(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "https://api.anthropic.com/v1",
        "openai"    => "https://api.openai.com/v1",
        "groq"      => "https://api.groq.com/openai/v1",
        "mistral"   => "https://api.mistral.ai/v1",
        "ollama"    => "http://127.0.0.1:11434/v1",
        "together"  => "https://api.together.xyz/v1",
        "deepseek"  => "https://api.deepseek.com/v1",
        _           => "https://api.openai.com/v1",
    }
}

fn apply_known_context_overrides(data: &mut ModelData, model_id: &str) {
    let id = model_id.to_lowercase();
    let (ctx, out) = if id.contains("claude") {
        if id.contains("opus") { (200_000, 16_384) }
        else                   { (200_000, 8_192)  }
    } else if id.contains("gpt-4o") {
        (128_000, 16_384)
    } else if id.contains("gpt-4-turbo") {
        (128_000, 4_096)
    } else if id.contains("o1") || id.contains("o3") {
        (200_000, 32_768)
    } else if id.contains("gemini-1.5") || id.contains("gemini-2") {
        (1_000_000, 8_192)
    } else if id.contains("gemini") {
        (32_768, 8_192)
    } else if id.contains("llama-3.1") || id.contains("llama3.1") {
        (131_072, 4_096)
    } else if id.contains("qwen2.5") || id.contains("qwen-2.5") {
        (32_768, 8_192)
    } else {
        return;
    };
    data.capabilities.context_window    = ctx;
    data.capabilities.max_output_tokens = out;
}
