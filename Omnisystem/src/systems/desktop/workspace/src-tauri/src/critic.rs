//! BonsAI Critic — lightweight response quality gating.
//!
//! After every inference, the Critic re-asks the model to rate its own response
//! on a 0.0–1.0 scale.  If the score is below the configured threshold the
//! caller can retry with higher temperature, up to `max_retries`.
//!
//! The Critic is self-contained: it reuses the existing ModelOrchestrator slot
//! so no second model or extra VRAM is needed.  The only cost is one small
//! additional inference call (~20 tokens) per response.

use std::sync::Arc;
use tracing::{debug, warn};

use crate::model_orchestrator::ModelOrchestrator;

// ── Critic ────────────────────────────────────────────────────────────────────

pub struct Critic {
    orchestrator: Arc<ModelOrchestrator>,
    /// Minimum acceptable score (0.0–1.0).  Responses below this trigger a retry.
    pub threshold: f32,
    /// Maximum number of regeneration attempts before returning the best so far.
    pub max_retries: usize,
}

impl Critic {
    pub fn new(orchestrator: Arc<ModelOrchestrator>, threshold: f32, max_retries: usize) -> Self {
        Self {
            orchestrator,
            threshold,
            max_retries,
        }
    }

    /// Score a (prompt, response) pair.  Returns a value in [0.0, 1.0].
    /// On failure returns `threshold` so the caller doesn't block.
    pub async fn score(&self, prompt: &str, response: &str) -> f32 {
        let eval_prompt = format!(
            "Rate the quality of the following AI response on a scale from 0.0 to 1.0.\n\
             Criteria: accuracy, completeness, clarity, helpfulness.\n\
             Output ONLY the decimal number, nothing else.\n\n\
             User question: {prompt}\n\
             AI response: {response}\n\n\
             Quality score (0.0–1.0):"
        );
        match self
            .orchestrator
            .infer_simple(&eval_prompt, 8, "critic")
            .await
        {
            Ok((text, _)) => parse_score(&text),
            Err(e) => {
                debug!("[critic] score error (using default): {e}");
                self.threshold // non-blocking fallback
            }
        }
    }

    /// Score and, if below threshold, retry inference with increased temperature.
    /// Returns `(best_response, final_score, retries_used)`.
    pub async fn gate(
        &self,
        prompt: &str,
        initial_response: String,
        retry_fn: impl Fn(f32) -> futures::future::BoxFuture<'static, Result<String, String>>,
    ) -> (String, f32, usize) {
        let mut best = initial_response.clone();
        let mut best_score = self.score(prompt, &initial_response).await;
        debug!(score=%best_score, "[critic] initial score");

        if best_score >= self.threshold {
            return (best, best_score, 0);
        }

        for attempt in 1..=self.max_retries {
            let temperature = 0.7 + attempt as f32 * 0.15;
            match retry_fn(temperature).await {
                Ok(new_response) => {
                    let new_score = self.score(prompt, &new_response).await;
                    debug!(attempt, score=%new_score, "[critic] retry score");
                    if new_score > best_score {
                        best_score = new_score;
                        best = new_response;
                    }
                    if best_score >= self.threshold {
                        return (best, best_score, attempt);
                    }
                }
                Err(e) => {
                    warn!("[critic] retry {attempt} failed: {e}");
                    break;
                }
            }
        }

        (best, best_score, self.max_retries)
    }
}

// ── Score parsing ─────────────────────────────────────────────────────────────

pub fn parse_score(text: &str) -> f32 {
    // Find first decimal-looking token in the response
    for word in text.split_whitespace() {
        let cleaned = word.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
        if let Ok(v) = cleaned.parse::<f32>() {
            return v.clamp(0.0, 1.0);
        }
    }
    0.6 // neutral default when we can't parse
}

// ── Nearest-neighbor domain router ───────────────────────────────────────────
//
// Queries past self-play examples to guess the task domain (code, math,
// music, creative, general) before inference.  The domain is injected into
// the system prompt so the model "knows" what kind of task it's doing.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskDomain {
    Code,
    Math,
    Music,
    Vision,
    Creative,
    Research,
    System,
    General,
}

impl TaskDomain {
    /// Heuristic domain classifier — runs in microseconds, no model needed.
    pub fn classify(prompt: &str) -> Self {
        let lower = prompt.to_lowercase();
        if lower.contains("music")
            || lower.contains("beat")
            || lower.contains("melody")
            || lower.contains("chord")
            || lower.contains("bpm")
            || lower.contains("audio")
            || lower.contains("song")
            || lower.contains("compose")
        {
            return Self::Music;
        }
        if lower.contains("image")
            || lower.contains("photo")
            || lower.contains("picture")
            || lower.contains("vision")
            || lower.contains("screenshot")
            || lower.contains("visual")
        {
            return Self::Vision;
        }
        if lower.contains("code")
            || lower.contains("function")
            || lower.contains("rust")
            || lower.contains("python")
            || lower.contains("javascript")
            || lower.contains("debug")
            || lower.contains("compile")
            || lower.contains("error")
            || lower.contains("api")
            || lower.contains("implement")
            || lower.contains("script")
            || lower.contains("program")
        {
            return Self::Code;
        }
        if lower.contains("math")
            || lower.contains("calcul")
            || lower.contains("equation")
            || lower.contains("formula")
            || lower.contains("proof")
            || lower.contains("derive")
            || lower.contains("integral")
            || lower.contains("statistic")
        {
            return Self::Math;
        }
        if lower.contains("write")
            || lower.contains("story")
            || lower.contains("creative")
            || lower.contains("poem")
            || lower.contains("essay")
            || lower.contains("narrative")
        {
            return Self::Creative;
        }
        if lower.contains("search")
            || lower.contains("explain")
            || lower.contains("research")
            || lower.contains("what is")
            || lower.contains("how does")
            || lower.contains("why")
        {
            return Self::Research;
        }
        if lower.contains("system")
            || lower.contains("file")
            || lower.contains("disk")
            || lower.contains("process")
            || lower.contains("memory")
            || lower.contains("cpu")
            || lower.contains("run command")
            || lower.contains("shell")
        {
            return Self::System;
        }
        Self::General
    }

    /// Short context hint injected at the start of the system prompt.
    pub fn system_hint(&self) -> &'static str {
        match self {
            Self::Code     => "Task domain: software engineering. Prefer precise, working code with minimal prose.",
            Self::Math     => "Task domain: mathematics. Show work step-by-step. Be exact.",
            Self::Music    => "Task domain: music production. You can generate audio with the generate_music tool.",
            Self::Vision   => "Task domain: visual understanding. Describe images precisely.",
            Self::Creative => "Task domain: creative writing. Be expressive and original.",
            Self::Research => "Task domain: research and explanation. Be thorough and cite reasoning.",
            Self::System   => "Task domain: system administration. Prefer safe, idempotent commands.",
            Self::General  => "",
        }
    }

    pub fn suggested_temperature(&self) -> f32 {
        match self {
            Self::Math | Self::Code | Self::System => 0.2,
            Self::Creative | Self::Music => 0.85,
            _ => 0.7,
        }
    }
}
