//! LoRA distillation: converts a skill's extracted rules into a DPO dataset
//! and triggers fine-tuning via the local BonsAI training API.
//!
//! Offline-first: all generation is heuristic. The LLM API call is optional
//! and only happens when a running inference server is present.

use std::io::Write as _;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{extractor::Rule, parser::SkillMetadata};

// ── Training example format ───────────────────────────────────────────────────

/// A single DPO training triple written to the JSONL dataset.
#[derive(Debug, Serialize, Deserialize)]
pub struct DpoExample {
    /// Instruction / user turn
    pub prompt: String,
    /// Preferred response (follows the rule)
    pub chosen: String,
    /// Rejected response (ignores or violates the rule)
    pub rejected: String,
    /// Source tag for filtering
    pub source: String,
    /// Weight hint for DPO loss (0.0–1.0)
    pub weight: f32,
}

/// Expanded SFT example for curriculum-learning warm-up.
#[derive(Debug, Serialize, Deserialize)]
pub struct SftExample {
    pub instruction: String,
    pub response: String,
    pub source: String,
}

// ── Heuristic example generation ─────────────────────────────────────────────

/// Generate DPO examples from extracted rules without any LLM call.
pub fn generate_dpo_examples(metadata: &SkillMetadata, rules: &[Rule]) -> Vec<DpoExample> {
    let skill_name = &metadata.name;
    let mut out = Vec::new();

    for rule in rules {
        let condition_display = if rule.condition == "always" {
            "a user asks for help".to_string()
        } else {
            rule.condition.clone()
        };

        // chosen: directly applies the rule
        let chosen = format!(
            "{} (applying the {} skill guideline: {})",
            rule.action, skill_name, rule.action
        );

        // rejected: explicitly ignores the guideline
        let rejected = format!(
            "I'll handle this without following the {} skill guidelines.",
            skill_name
        );

        out.push(DpoExample {
            prompt: format!("When {}, what should I do?", condition_display),
            chosen,
            rejected,
            source: format!("skill/{}", skill_name),
            weight: rule.confidence,
        });

        // Second example: ask about the skill directly
        out.push(DpoExample {
            prompt: format!(
                "I'm using the {} skill. The rule says: '{}'. How do I apply it?",
                skill_name, rule.action
            ),
            chosen: format!("To apply this rule: {}", rule.action),
            rejected: format!(
                "This rule isn't relevant. I'll respond however seems best."
            ),
            source: format!("skill/{}", skill_name),
            weight: rule.confidence * 0.9,
        });
    }

    // Skill-level meta examples
    out.push(DpoExample {
        prompt: format!("What does the {} skill do?", skill_name),
        chosen: format!(
            "The {} skill provides the following guidelines: {}",
            skill_name,
            rules.iter().map(|r| r.action.as_str()).collect::<Vec<_>>().join("; ")
        ),
        rejected: "I don't know what that skill does.".into(),
        source: format!("skill/{}/meta", skill_name),
        weight: 0.7,
    });

    out
}

/// Generate SFT warm-up examples (simpler, single-turn).
pub fn generate_sft_examples(metadata: &SkillMetadata, rules: &[Rule]) -> Vec<SftExample> {
    rules
        .iter()
        .map(|rule| SftExample {
            instruction: format!(
                "Apply the {} skill guideline to this situation: {}",
                metadata.name, rule.condition
            ),
            response: rule.action.clone(),
            source: format!("skill/{}/sft", metadata.name),
        })
        .collect()
}

// ── Dataset persistence ───────────────────────────────────────────────────────

/// Write DPO examples to a JSONL file. Returns number of examples written.
pub fn write_dpo_dataset(examples: &[DpoExample], path: &Path) -> Result<usize> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    for ex in examples {
        serde_json::to_writer(&mut f, ex)?;
        writeln!(f)?;
    }
    Ok(examples.len())
}

/// Write SFT examples to a JSONL file.
pub fn write_sft_dataset(examples: &[SftExample], path: &Path) -> Result<usize> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    for ex in examples {
        serde_json::to_writer(&mut f, ex)?;
        writeln!(f)?;
    }
    Ok(examples.len())
}

// ── Training API trigger ──────────────────────────────────────────────────────

/// Result returned after submitting a distillation job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistillationJob {
    pub skill_id: String,
    pub dpo_examples: usize,
    pub sft_examples: usize,
    pub dpo_dataset_path: String,
    pub sft_dataset_path: String,
    /// `Some(job_id)` if successfully submitted to the training API.
    pub training_job_id: Option<String>,
    pub status: DistillationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DistillationStatus {
    DatasetReady,
    Submitted,
    Failed(String),
}

/// Generate datasets and optionally submit to the local training endpoint.
///
/// The training API call is best-effort: if the server is not running the
/// datasets are still written and the job is returned with `DatasetReady`.
pub async fn distill_skill(
    metadata: &SkillMetadata,
    rules: &[Rule],
    base_model_path: &str,
    output_adapter_dir: &str,
    training_api_url: Option<&str>,
) -> Result<DistillationJob> {
    let skill_id = format!(
        "{}/{}",
        metadata.owner.as_deref().unwrap_or("local"),
        &metadata.name
    );
    let safe_id = skill_id.replace('/', "__");
    let data_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai")
        .join("training")
        .join("skills")
        .join(&safe_id);

    let dpo_path = data_dir.join("dpo.jsonl");
    let sft_path = data_dir.join("sft.jsonl");

    // Generate
    let dpo_examples = generate_dpo_examples(metadata, rules);
    let sft_examples = generate_sft_examples(metadata, rules);
    let dpo_count = write_dpo_dataset(&dpo_examples, &dpo_path)?;
    let sft_count = write_sft_dataset(&sft_examples, &sft_path)?;

    let mut job = DistillationJob {
        skill_id: skill_id.clone(),
        dpo_examples: dpo_count,
        sft_examples: sft_count,
        dpo_dataset_path: dpo_path.to_string_lossy().into_owned(),
        sft_dataset_path: sft_path.to_string_lossy().into_owned(),
        training_job_id: None,
        status: DistillationStatus::DatasetReady,
    };

    // Optionally submit
    let api_url = training_api_url.unwrap_or("http://127.0.0.1:11369");
    let endpoint = format!("{}/api/v1/training/fine_tune", api_url);

    let payload = serde_json::json!({
        "skill_id":      skill_id,
        "base_model":    base_model_path,
        "dpo_dataset":   dpo_path.to_str(),
        "sft_dataset":   sft_path.to_str(),
        "output_adapter": output_adapter_dir,
        "method":        "dpo",
        "epochs":        1,
        "source":        "skill_compiler",
    });

    match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap()
        .post(&endpoint)
        .json(&payload)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                job.training_job_id = body["job_id"].as_str().map(|s| s.to_string());
            }
            job.status = DistillationStatus::Submitted;
        }
        Ok(resp) => {
            let msg = format!("HTTP {}", resp.status());
            tracing::warn!(skill=%skill_id, error=%msg, "training API rejected job; dataset ready locally");
        }
        Err(e) => {
            tracing::debug!(skill=%skill_id, error=%e, "training API not reachable; dataset saved locally");
        }
    }

    Ok(job)
}
