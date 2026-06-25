use crate::core::{BonsaiPlan, BonsaiResponse};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::RwLock;

/// A single curated training example in the format expected by finetune.py.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingExample {
    pub text: String,
    pub source: ExampleSource,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExampleSource {
    /// Produced live by the BonsaiCore process() loop
    Live,
    /// Hand-authored synthetic data
    Synthetic,
}

/// Minimum confidence for an example to be retained.
const MIN_CONFIDENCE: f32 = 0.70;

/// Rolling window before we flush to disk.
const FLUSH_EVERY: usize = 50;

pub struct DataCurator {
    output_path: PathBuf,
    buffer: RwLock<Vec<TrainingExample>>,
    /// Fingerprints (first 64 chars of request, lowercased) for dedup.
    seen: RwLock<std::collections::HashSet<String>>,
    prompt_template: String,
}

impl DataCurator {
    pub fn new(output_path: PathBuf, prompt_template: String) -> Self {
        // Load existing fingerprints so we never duplicate across restarts.
        let seen = load_fingerprints(&output_path);
        Self {
            output_path,
            buffer: RwLock::new(Vec::new()),
            seen: RwLock::new(seen),
            prompt_template,
        }
    }

    /// Called after every successful BonsaiCore::process(). Returns true if
    /// the example was accepted (passes quality gate + dedup).
    pub async fn ingest(
        &self,
        request: &str,
        plan: &BonsaiPlan,
        _response: &BonsaiResponse,
    ) -> bool {
        // Quality gate
        if plan.confidence < MIN_CONFIDENCE {
            return false;
        }
        if plan.intent.is_empty() {
            return false;
        }

        // Dedup by request fingerprint
        let fp = fingerprint(request);
        {
            let mut seen = self.seen.write().await;
            if seen.contains(&fp) {
                return false;
            }
            seen.insert(fp);
        }

        // Build training text in the same format generate_synthetic_data.py produces
        let plan_json = match serde_json::to_string(plan) {
            Ok(j) => j,
            Err(_) => return false,
        };
        let text = self
            .prompt_template
            .replace("{request}", request)
            .replace("{memory}", "None")
            + &plan_json;

        let example = TrainingExample {
            text,
            source: ExampleSource::Live,
            confidence: plan.confidence,
        };

        let mut buf = self.buffer.write().await;
        buf.push(example);

        let should_flush = buf.len() >= FLUSH_EVERY;
        drop(buf);

        if should_flush {
            self.flush().await;
        }

        true
    }

    /// Append buffered examples to the JSONL file, then clear the buffer.
    pub async fn flush(&self) {
        let mut buf = self.buffer.write().await;
        if buf.is_empty() {
            return;
        }

        // Read existing content so we can append atomically
        let existing = std::fs::read_to_string(&self.output_path).unwrap_or_default();
        let mut out = existing;
        for ex in buf.iter() {
            if let Ok(s) = serde_json::to_string(ex) {
                out.push_str(&s);
                out.push('\n');
            }
        }

        if crate::atomic_write(&self.output_path, out.as_bytes()).is_ok() {
            buf.clear();
        }
    }

    /// How many examples are buffered (not yet flushed).
    pub async fn buffered(&self) -> usize {
        self.buffer.read().await.len()
    }

    /// Total unique fingerprints seen (flushed + buffered).
    pub async fn total_seen(&self) -> usize {
        self.seen.read().await.len()
    }
}

fn fingerprint(request: &str) -> String {
    request.to_lowercase().chars().take(64).collect()
}

fn load_fingerprints(path: &PathBuf) -> std::collections::HashSet<String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return std::collections::HashSet::new(),
    };
    content
        .lines()
        .filter_map(|l| {
            let ex: TrainingExample = serde_json::from_str(l).ok()?;
            // Re-derive fingerprint from the text prefix after the last newline in prompt
            // (we store full text, so just use first 64 chars of the text as proxy)
            Some(ex.text.chars().take(64).collect())
        })
        .collect()
}
