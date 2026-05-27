//! Belief Revision System — Bayesian updating, contradiction resolution,
//! temporal evidence decay, and source diversity scoring.

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use bonsai_knowledge::{Belief, BeliefId, Evidence, ProvenanceSource};
use bonsai_verify::{AxiomKernel, definitionally_equal};

// ── BeliefRevision audit entry ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefRevision {
    pub belief_id: BeliefId,
    pub old_confidence: f32,
    pub new_confidence: f32,
    pub reason: String,
}

// ── BeliefReviser ─────────────────────────────────────────────────────────────

pub struct BeliefReviser {
    /// Prior confidences keyed by belief ID.
    priors: HashMap<BeliefId, f32>,
    /// Rolling likelihood samples per belief.
    likelihoods: HashMap<BeliefId, Vec<f32>>,
}

impl BeliefReviser {
    pub fn new() -> Self {
        Self { priors: HashMap::new(), likelihoods: HashMap::new() }
    }

    // ── Bayesian update ───────────────────────────────────────────────────────

    /// P(H|E) = P(E|H) · P(H) / P(E).
    /// Clamps posterior to (0.01, 0.99) — never fully certain or impossible.
    pub fn update_belief(&mut self, belief: &mut Belief, new_evidence: Evidence) -> f32 {
        let prior = belief.confidence;
        let likelihood = new_evidence.strength.clamp(0.0, 1.0);

        // Marginal P(E): weighted average of likelihood under H and ~H
        let marginal = self.compute_marginal_likelihood(belief, &new_evidence);

        let posterior = if marginal > 0.0 {
            (likelihood * prior / marginal).clamp(0.01, 0.99)
        } else {
            prior
        };

        belief.confidence = posterior;
        belief.evidence.push(new_evidence);
        belief.times_confirmed += 1;
        belief.last_updated = chrono::Utc::now().timestamp_millis();

        self.priors.insert(belief.id.clone(), prior);
        self.likelihoods.entry(belief.id.clone()).or_default().push(likelihood);

        posterior
    }

    fn compute_marginal_likelihood(&self, belief: &Belief, evidence: &Evidence) -> f32 {
        let p_e_given_h = evidence.strength.clamp(0.01, 0.99);
        // Conservative assumption: P(E|~H) = 1 - P(E|H)
        let p_e_given_not_h = 1.0 - p_e_given_h;
        let p_h = belief.confidence;
        p_e_given_h * p_h + p_e_given_not_h * (1.0 - p_h)
    }

    // ── Contradiction resolution ───────────────────────────────────────────────

    /// When two beliefs contradict, the weaker one has its confidence halved.
    /// Returns one `BeliefRevision` per weakened belief.
    pub fn resolve_contradictions(&self, beliefs: &mut Vec<Belief>) -> Vec<BeliefRevision> {
        let mut revisions = Vec::new();
        let n = beliefs.len();

        for i in 0..n {
            for j in (i + 1)..n {
                if self.are_contradictory(&beliefs[i], &beliefs[j]) {
                    let weaker_idx = if beliefs[i].confidence <= beliefs[j].confidence { i } else { j };
                    let stronger_idx = 1 - weaker_idx + (i + j - weaker_idx); // the other one
                    // recalculate cleanly:
                    let (stronger_conf, stronger_id) = if i == weaker_idx {
                        (beliefs[j].confidence, beliefs[j].id.clone())
                    } else {
                        (beliefs[i].confidence, beliefs[i].id.clone())
                    };

                    let old = beliefs[weaker_idx].confidence;
                    let new_c = (old * 0.5).max(0.01);
                    revisions.push(BeliefRevision {
                        belief_id: beliefs[weaker_idx].id.clone(),
                        old_confidence: old,
                        new_confidence: new_c,
                        reason: format!("Contradicted by belief {} (confidence {:.2})",
                            stronger_id, stronger_conf),
                    });
                    beliefs[weaker_idx].confidence = new_c;
                    beliefs[weaker_idx].times_challenged += 1;
                }
            }
        }
        revisions
    }

    /// Formal contradiction check via Axiom kernel when both beliefs have
    /// formal statements; otherwise uses simple negation keyword heuristic.
    fn are_contradictory(&self, a: &Belief, b: &Belief) -> bool {
        if let (Some(fa), Some(fb)) = (&a.formal_statement, &b.formal_statement) {
            let k = AxiomKernel::new();
            // a ≡ ¬b iff a ≡ (b → ⊥); use definitional equality in the env
            let not_b = bonsai_verify::Term::pi("_", fb.clone(), bonsai_verify::Term::prop());
            definitionally_equal(fa, &not_b, &k.env)
        } else {
            // Heuristic: one statement is the negation of the other
            let neg_a = negate_statement(&a.statement);
            let neg_b = negate_statement(&b.statement);
            b.statement.to_lowercase() == neg_a || a.statement.to_lowercase() == neg_b
        }
    }

    // ── Temporal decay ────────────────────────────────────────────────────────

    /// Exponential decay: evidence older than `half_life_hours` hours loses half its weight.
    pub fn apply_temporal_decay(&mut self, beliefs: &mut Vec<Belief>, half_life_hours: f64) {
        let now = chrono::Utc::now().timestamp_millis() as f64;
        let decay_k = (2.0f64).ln() / (half_life_hours * 3_600_000.0);

        for belief in beliefs.iter_mut() {
            if belief.evidence.is_empty() { continue; }

            let mut total_weight = 0.0f64;
            let mut weighted_sum = 0.0f64;

            for ev in &belief.evidence {
                let age_ms = (now - ev.timestamp as f64).max(0.0);
                let w = (-decay_k * age_ms).exp();
                weighted_sum += ev.strength as f64 * w;
                total_weight += w;
            }

            if total_weight > 0.0 {
                let decayed = (weighted_sum / total_weight) as f32;
                let prior = self.priors.get(&belief.id).copied().unwrap_or(0.5);
                belief.confidence = (0.3 * prior + 0.7 * decayed).clamp(0.01, 0.99);
                belief.last_updated = chrono::Utc::now().timestamp_millis();
            }
        }
    }

    // ── Source diversity bonus ────────────────────────────────────────────────

    /// Multi-source confirmation adds up to 10% confidence bonus.
    pub fn compute_source_diversity_bonus(&self, belief: &Belief) -> f32 {
        let unique_source_types: HashSet<u8> = belief.evidence.iter()
            .map(|e| source_discriminant(&e.source))
            .collect();
        let diversity = unique_source_types.len() as f32 / 5.0;
        (diversity * 0.1).min(0.1)
    }

    /// Apply the diversity bonus in-place.
    pub fn apply_diversity_bonus(&self, belief: &mut Belief) {
        let bonus = self.compute_source_diversity_bonus(belief);
        belief.confidence = (belief.confidence + bonus).min(0.99);
    }

    // ── Consistency check ─────────────────────────────────────────────────────

    /// Check whether a new statement is consistent with existing high-confidence beliefs.
    pub fn check_consistency(&self, beliefs: &[Belief], statement: &str) -> ConsistencyResult {
        let candidate = Belief::new(statement, 0.5);
        let contradictions: Vec<&Belief> = beliefs.iter()
            .filter(|b| b.confidence > 0.7 && self.are_contradictory(&candidate, b))
            .collect();

        if contradictions.is_empty() {
            ConsistencyResult::Consistent
        } else {
            ConsistencyResult::Contradicts {
                conflicting: contradictions.iter().map(|b| b.id.clone()).collect(),
                max_conflict_confidence: contradictions.iter()
                    .map(|b| b.confidence)
                    .fold(0.0f32, f32::max),
            }
        }
    }
}

impl Default for BeliefReviser {
    fn default() -> Self { Self::new() }
}

// ── ConsistencyResult ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsistencyResult {
    Consistent,
    Contradicts { conflicting: Vec<BeliefId>, max_conflict_confidence: f32 },
    Uncertain,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn negate_statement(s: &str) -> String {
    let s = s.to_lowercase();
    if s.starts_with("not ") {
        s[4..].to_string()
    } else if s.starts_with("it is not true that ") {
        s[20..].to_string()
    } else {
        format!("not {}", s)
    }
}

fn source_discriminant(source: &ProvenanceSource) -> u8 {
    match source {
        ProvenanceSource::UserStatement { .. } => 0,
        ProvenanceSource::ModelInference { .. } => 1,
        ProvenanceSource::ToolExecution { .. } => 2,
        ProvenanceSource::DeductiveProof { .. } => 3,
        ProvenanceSource::ExternalDocument { .. } => 4,
        ProvenanceSource::Observation { .. } => 5,
        ProvenanceSource::Derived { .. } => 6,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use bonsai_knowledge::Evidence;

    fn make_evidence(strength: f32) -> Evidence {
        Evidence {
            source: ProvenanceSource::UserStatement { session_id: "test".into() },
            strength,
            timestamp: chrono::Utc::now().timestamp_millis(),
            description: "test evidence".into(),
        }
    }

    #[test]
    fn bayesian_update_increases_high_evidence() {
        let mut br = BeliefReviser::new();
        let mut belief = Belief::new("sky is blue", 0.5);
        let posterior = br.update_belief(&mut belief, make_evidence(0.9));
        assert!(posterior > 0.5);
    }

    #[test]
    fn bayesian_update_decreases_low_evidence() {
        let mut br = BeliefReviser::new();
        let mut belief = Belief::new("moon is made of cheese", 0.9);
        let posterior = br.update_belief(&mut belief, make_evidence(0.1));
        assert!(posterior < 0.9);
    }

    #[test]
    fn contradiction_resolution_reduces_weaker() {
        let br = BeliefReviser::new();
        let mut beliefs = vec![
            Belief::new("the system is secure", 0.9),
            Belief::new("not the system is secure", 0.3),
        ];
        let revisions = br.resolve_contradictions(&mut beliefs);
        assert!(!revisions.is_empty());
        // The weaker belief (index 1) should have been reduced
        assert!(beliefs[1].confidence < 0.3);
    }

    #[test]
    fn diversity_bonus_capped_at_10_percent() {
        let br = BeliefReviser::new();
        let mut b = Belief::new("test", 0.8);
        for d in [
            ProvenanceSource::UserStatement { session_id: "s".into() },
            ProvenanceSource::ModelInference { model_id: "m".into(), adapter_id: None },
            ProvenanceSource::ToolExecution { tool_name: "t".into(), result_hash: "h".into() },
            ProvenanceSource::DeductiveProof { proof_id: "p".into(), kernel_version: "1".into() },
            ProvenanceSource::ExternalDocument { document_hash: "d".into(), source_url: None },
        ] {
            b.evidence.push(Evidence { source: d, strength: 0.8, timestamp: 0, description: "".into() });
        }
        let bonus = br.compute_source_diversity_bonus(&b);
        assert!(bonus <= 0.101); // ≤10%
    }
}
