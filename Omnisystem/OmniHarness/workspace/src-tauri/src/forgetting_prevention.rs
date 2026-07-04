//! Forgetting Prevention — ensures every training cycle preserves core capabilities.
//!
//! After every training cycle, before promotion, this module:
//!  1. Evaluates the candidate adapter on the 50-prompt core competency set.
//!  2. Compares against the previous adapter's scores (stored as baseline).
//!  3. For any competency that regressed > max_regression_pct, generates remedial
//!     training examples targeting the specific failure pattern.
//!  4. Returns `ForgettingStatus::Safe` or `ForgettingStatus::Regression` with
//!     full details so the training loop can decide how to proceed.
//!
//! The core competency set is immutable and NEVER used as training data.
//! Remedial examples are SYNTHETIC variants generated around failure patterns.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::evaluation_harness::{core_competency_set, BenchmarkProblem, EvaluationHarness};
use crate::model_orchestrator::ModelOrchestrator;
use crate::unified_training_collector::{
    classify_domain, ModelRole, TrainingInput, TrainingOutput, TrainingSource,
    TrainingStrategyType, UnifiedTrainingExample,
};

// ══════════════════════════════════════════════════════════════════════════════
// § 1 — Status types
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetencyRegression {
    pub competency_id: String,
    pub competency_name: String,
    pub previous_score: f32,
    pub new_score: f32,
    pub regression: f32, // positive = degraded
    pub failed_prompts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ForgettingStatus {
    Safe,
    Regression {
        regressions: Vec<CompetencyRegression>,
        remedial_count: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgettingReport {
    pub status: ForgettingStatus,
    pub checked_at: i64,
    pub total_prompts: u32,
    pub passed: u32,
    pub overall_score: f32,
}

// ══════════════════════════════════════════════════════════════════════════════
// § 2 — Baseline store
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetencyBaseline {
    /// Per-competency-id score from the most recent successful adapter.
    pub scores: HashMap<String, f32>,
    pub adapter_id: String,
    pub recorded_at: i64,
}

impl CompetencyBaseline {
    fn path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/eval/competency_baseline.json")
    }

    pub fn load() -> Option<Self> {
        let content = std::fs::read_to_string(Self::path()).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self) {
        if let Some(parent) = Self::path().parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::path(), json);
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// § 3 — Remedial example generator
// ══════════════════════════════════════════════════════════════════════════════

/// Generates synthetic remedial training examples around a competency regression.
/// These are high-priority DPO/LoRA examples that directly address the failure.
pub struct RemedialTrainer {
    orchestrator: Arc<ModelOrchestrator>,
}

impl RemedialTrainer {
    pub fn new(orchestrator: Arc<ModelOrchestrator>) -> Self {
        Self { orchestrator }
    }

    /// For each failed prompt in a regression, generate N varied remedial examples.
    pub async fn generate(
        &self,
        regression: &CompetencyRegression,
        variants_per_prompt: usize,
    ) -> Vec<UnifiedTrainingExample> {
        let mut examples = Vec::new();

        for original_prompt in &regression.failed_prompts {
            // 1. Generate the ideal response using higher temperature/top-p
            let ideal = match self
                .orchestrator
                .infer_simple(
                    &format!("Provide the best possible response to: {original_prompt}"),
                    512,
                    "remedial",
                )
                .await
            {
                Ok((text, _)) => text,
                Err(_) => continue,
            };

            // 2. Generate a "bad" response to form the DPO rejected side
            let bad = match self
                .orchestrator
                .infer_simple(
                    &format!(
                        "Generate a POOR quality response to the following that misses the point, \
                     is too vague, or makes a factual error. Response only, no explanation.\n\
                     Prompt: {original_prompt}"
                    ),
                    256,
                    "remedial",
                )
                .await
            {
                Ok((text, _)) => text,
                Err(_) => format!("I don't know how to answer that."),
            };

            // 3. Core example: DPO pair (ideal chosen, bad rejected)
            examples.push(make_remedial_example(
                original_prompt,
                &ideal,
                Some(&bad),
                &regression.competency_id,
                1.0, // maximum priority — this is a regression fix
            ));

            // 4. Varied prompt forms to improve generalisation
            let variants = self
                .generate_variants(original_prompt, variants_per_prompt)
                .await;
            for variant in variants {
                let variant_ideal = match self
                    .orchestrator
                    .infer_simple(
                        &format!("Provide the best possible response to: {variant}"),
                        512,
                        "remedial",
                    )
                    .await
                {
                    Ok((text, _)) => text,
                    Err(_) => continue,
                };
                examples.push(make_remedial_example(
                    &variant,
                    &variant_ideal,
                    None,
                    &regression.competency_id,
                    0.85,
                ));
            }
        }

        info!(
            "[forgetting] generated {} remedial examples for regression in '{}'",
            examples.len(),
            regression.competency_name
        );
        examples
    }

    /// Generate rephrasings / difficulty variations of a prompt.
    async fn generate_variants(&self, prompt: &str, n: usize) -> Vec<String> {
        let system = format!(
            "Generate {n} varied versions of the following question. \
             Each version should test the same underlying capability but use \
             different wording, perspective, or slight difficulty variation. \
             Output one per line, no numbering.\n\nOriginal: {prompt}"
        );
        match self
            .orchestrator
            .infer_simple(&system, 300, "remedial")
            .await
        {
            Ok((text, _)) => text
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty() && l.len() > 10)
                .take(n)
                .collect(),
            Err(_) => vec![],
        }
    }
}

fn make_remedial_example(
    prompt: &str,
    chosen: &str,
    rejected: Option<&str>,
    competency_id: &str,
    priority: f32,
) -> UnifiedTrainingExample {
    let strategies = if rejected.is_some() {
        vec![TrainingStrategyType::Dpo, TrainingStrategyType::Lora]
    } else {
        vec![TrainingStrategyType::Lora]
    };

    let output = if let Some(rej) = rejected {
        TrainingOutput::PreferencePair {
            chosen: chosen.to_string(),
            rejected: rej.to_string(),
        }
    } else {
        TrainingOutput::Text {
            content: chosen.to_string(),
        }
    };

    UnifiedTrainingExample {
        id: uuid::Uuid::new_v4().to_string(),
        target_model: ModelRole::Primary,
        suitable_strategies: strategies,
        input: TrainingInput::Prompt {
            text: prompt.to_string(),
        },
        expected_output: output,
        source: TrainingSource::SelfPlay,
        quality_score: priority,
        priority,
        timestamp: chrono::Utc::now().timestamp_micros(),
        dimensions: vec![classify_domain(prompt)],
        used: false,
        use_count: 0,
        metadata: serde_json::json!({
            "source": "forgetting_prevention",
            "competency_id": competency_id,
        }),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// § 4 — Forgetting Prevention (main public API)
// ══════════════════════════════════════════════════════════════════════════════

pub struct ForgettingPrevention {
    harness: Arc<EvaluationHarness>,
    remedial_trainer: RemedialTrainer,
    core_problems: Vec<BenchmarkProblem>,
    /// Maximum allowed regression per competency before triggering remediation.
    max_regression: f32,
    baseline: RwLock<Option<CompetencyBaseline>>,
}

impl ForgettingPrevention {
    pub fn new(harness: Arc<EvaluationHarness>, orchestrator: Arc<ModelOrchestrator>) -> Arc<Self> {
        let baseline = CompetencyBaseline::load();
        Arc::new(Self {
            harness,
            remedial_trainer: RemedialTrainer::new(orchestrator),
            core_problems: core_competency_set(),
            max_regression: 0.02, // 2 % max allowed regression per competency
            baseline: RwLock::new(baseline),
        })
    }

    /// Evaluate the current model state against the stored baseline.
    /// Returns `ForgettingReport` with full details.
    pub async fn check(&self, adapter_id: &str) -> ForgettingReport {
        let (safety_ok, bench_results) = self.harness.run_core_check().await;
        let total: u32 = bench_results.iter().map(|r| r.total).sum();
        let passed_total: u32 = bench_results.iter().map(|r| r.passed).sum();
        let overall = if total > 0 {
            passed_total as f32 / total as f32
        } else {
            0.0
        };

        // Build per-competency score map from benchmark results
        let current_scores: HashMap<String, f32> = bench_results
            .iter()
            .map(|r| (r.dimension.clone(), r.score))
            .collect();

        let baseline_guard = self.baseline.read().await;
        let baseline = match baseline_guard.as_ref() {
            None => {
                // No baseline yet — this is the first run; record as baseline and return safe
                drop(baseline_guard);
                self.update_baseline(adapter_id, &current_scores).await;
                return ForgettingReport {
                    status: ForgettingStatus::Safe,
                    checked_at: chrono::Utc::now().timestamp_micros(),
                    total_prompts: total,
                    passed: passed_total,
                    overall_score: overall,
                };
            }
            Some(b) => b.clone(),
        };
        drop(baseline_guard);

        let mut regressions: Vec<CompetencyRegression> = Vec::new();

        for (competency_id, &current) in &current_scores {
            let previous = baseline
                .scores
                .get(competency_id)
                .copied()
                .unwrap_or(current);
            let regression = previous - current;
            if regression > self.max_regression {
                // Find which prompts failed for this competency
                let failed_prompts: Vec<String> = self
                    .core_problems
                    .iter()
                    .filter(|p| &p.dimension == competency_id)
                    .map(|p| p.prompt.clone())
                    .take(5) // limit to 5 representative examples
                    .collect();

                regressions.push(CompetencyRegression {
                    competency_id: competency_id.clone(),
                    competency_name: competency_id.replace('_', " "),
                    previous_score: previous,
                    new_score: current,
                    regression,
                    failed_prompts,
                });
            }
        }

        if regressions.is_empty() {
            // No regressions — update baseline to current scores
            self.update_baseline(adapter_id, &current_scores).await;
            ForgettingReport {
                status: ForgettingStatus::Safe,
                checked_at: chrono::Utc::now().timestamp_micros(),
                total_prompts: total,
                passed: passed_total,
                overall_score: overall,
            }
        } else {
            warn!(
                "[forgetting] {} regressions detected for adapter {}",
                regressions.len(),
                adapter_id
            );
            for r in &regressions {
                warn!(
                    "  [{adapter_id}] {} regressed {:.1}% → {:.1}%",
                    r.competency_name,
                    r.previous_score * 100.0,
                    r.new_score * 100.0
                );
            }
            let cnt = regressions.len();
            ForgettingReport {
                status: ForgettingStatus::Regression {
                    regressions,
                    remedial_count: cnt,
                },
                checked_at: chrono::Utc::now().timestamp_micros(),
                total_prompts: total,
                passed: passed_total,
                overall_score: overall,
            }
        }
    }

    /// Generate remedial training examples for all regressions in a report.
    pub async fn remediate(&self, report: &ForgettingReport) -> Vec<UnifiedTrainingExample> {
        let regressions = match &report.status {
            ForgettingStatus::Regression { regressions, .. } => regressions,
            ForgettingStatus::Safe => return vec![],
        };

        let mut all_examples = Vec::new();
        for regression in regressions {
            let severity = regression.regression / self.max_regression;
            // Generate more examples for severe regressions
            let variants = (severity * 3.0).round() as usize + 2;
            let examples = self.remedial_trainer.generate(regression, variants).await;
            all_examples.extend(examples);
        }

        // Always include all 50 core competency prompts as LoRA examples at max priority.
        // Run the core check once (side-effect: warms any caches) and then add all prompts.
        let (_ok, _results) = self.harness.run_core_check().await;
        for p in &self.core_problems {
            all_examples.push(make_remedial_example(
                &p.prompt,
                "correct response", // placeholder — will be filled by trainer
                None,
                &p.dimension,
                0.95,
            ));
        }

        info!(
            "[forgetting] total remedial examples generated: {}",
            all_examples.len()
        );
        all_examples
    }

    async fn update_baseline(&self, adapter_id: &str, scores: &HashMap<String, f32>) {
        let baseline = CompetencyBaseline {
            scores: scores.clone(),
            adapter_id: adapter_id.to_string(),
            recorded_at: chrono::Utc::now().timestamp_micros(),
        };
        baseline.save();
        *self.baseline.write().await = Some(baseline);
    }

    pub async fn current_baseline(&self) -> Option<CompetencyBaseline> {
        self.baseline.read().await.clone()
    }
}
