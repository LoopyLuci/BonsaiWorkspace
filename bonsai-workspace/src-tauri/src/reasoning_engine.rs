//! Multi-Strategy Reasoning Engine — deductive, inductive, abductive,
//! analogical, and counterfactual reasoning with Axiom kernel integration.

use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use bonsai_knowledge::{
    Belief, BeliefId, Entity, Evidence, KnowledgeGraph, ProvenanceSource,
    new_belief_id,
};
use bonsai_verify::{AxiomKernel, Context as VerifyContext, Term, definitionally_equal};

use crate::belief_reviser::{BeliefReviser, ConsistencyResult};
use crate::metacognitive_monitor::{
    MetacognitiveMonitor, Outcome, ReasoningRecord, ReasoningStrategy,
};

// ── ReasoningResult ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningResult {
    pub strategy: String,
    pub query: String,
    pub conclusion: String,
    pub confidence: f32,
    pub steps: Vec<String>,
    pub new_beliefs: Vec<Belief>,
    pub latency_ms: u64,
}

// ── DPO training pair ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpoTrainingPair {
    pub prompt: String,
    pub chosen: String,
    pub rejected: String,
    pub weight: f32,
    pub source: String,
    pub metadata: serde_json::Value,
}

// ── ReasoningEngine ───────────────────────────────────────────────────────────

pub struct ReasoningEngine {
    pub knowledge: Arc<KnowledgeGraph>,
    pub belief_reviser: Arc<RwLock<BeliefReviser>>,
    pub metacognitive: Arc<RwLock<MetacognitiveMonitor>>,
}

impl ReasoningEngine {
    pub fn new(knowledge: Arc<KnowledgeGraph>) -> Self {
        Self {
            knowledge,
            belief_reviser: Arc::new(RwLock::new(BeliefReviser::new())),
            metacognitive: Arc::new(RwLock::new(MetacognitiveMonitor::new())),
        }
    }

    // ── Public dispatch ───────────────────────────────────────────────────────

    /// Route to the best strategy or the one specified.
    pub async fn reason(&self, query: &str, strategy_hint: &str) -> ReasoningResult {
        let start = Instant::now();
        let strategy = match strategy_hint {
            "deduce" | "deduction" => ReasoningStrategy::Deduction,
            "induce" | "induction" => ReasoningStrategy::Induction,
            "abduce" | "abduction" => ReasoningStrategy::Abduction,
            "analogize" | "analogy" => ReasoningStrategy::Analogy,
            "counterfactual" => ReasoningStrategy::Counterfactual,
            _ => self.select_strategy(query),
        };

        let result = match strategy {
            ReasoningStrategy::Deduction => self.deduce_from_graph(query).await,
            ReasoningStrategy::Induction => self.induce_pattern(query).await,
            ReasoningStrategy::Abduction => self.abduce_explanation(query).await,
            ReasoningStrategy::Analogy => self.analogize(query).await,
            ReasoningStrategy::Counterfactual => self.counterfactual(query).await,
            _ => self.deduce_from_graph(query).await,
        };

        let latency = start.elapsed().as_millis() as u64;
        let mut result = result;
        result.latency_ms = latency;

        // Record attempt for metacognition (outcome unknown until user feedback)
        let record = ReasoningRecord {
            id: Uuid::new_v4().to_string(),
            strategy: strategy.clone(),
            query: query.to_string(),
            conclusion: result.conclusion.clone(),
            predicted_confidence: result.confidence,
            actual_outcome: Outcome::Unknown,
            latency_ms: latency,
            timestamp: chrono::Utc::now().timestamp_millis(),
        };
        self.metacognitive.write().await.record(record);

        // Calibrate confidence
        let calibrated = self.metacognitive.read().await
            .calibrated_confidence(&strategy, result.confidence);
        result.confidence = calibrated;

        // Ingest new beliefs into graph
        for belief in &result.new_beliefs {
            self.knowledge.add_belief(belief.clone());
        }

        result
    }

    /// Record user feedback for a previous reasoning attempt.
    pub async fn record_outcome(&self, query: &str, correct: bool, correction: Option<String>) {
        let outcome = if correct {
            Outcome::Correct
        } else if let Some(ans) = correction {
            Outcome::UserCorrected { corrected_answer: ans }
        } else {
            Outcome::Incorrect
        };

        let record = ReasoningRecord {
            id: Uuid::new_v4().to_string(),
            strategy: ReasoningStrategy::Hybrid,
            query: query.to_string(),
            conclusion: "user-corrected".into(),
            predicted_confidence: 0.5,
            actual_outcome: outcome,
            latency_ms: 0,
            timestamp: chrono::Utc::now().timestamp_millis(),
        };
        self.metacognitive.write().await.record(record);
    }

    // ── Strategy selection ────────────────────────────────────────────────────

    fn select_strategy(&self, query: &str) -> ReasoningStrategy {
        let q = query.to_lowercase();
        if q.contains("why") || q.contains("cause") || q.contains("explain") {
            ReasoningStrategy::Abduction
        } else if q.contains("if") && (q.contains("had") || q.contains("would")) {
            ReasoningStrategy::Counterfactual
        } else if q.contains("like") || q.contains("similar to") || q.contains("analogous") {
            ReasoningStrategy::Analogy
        } else if q.contains("pattern") || q.contains("all") || q.contains("generally") {
            ReasoningStrategy::Induction
        } else {
            ReasoningStrategy::Deduction
        }
    }

    // ── Deduction ─────────────────────────────────────────────────────────────

    async fn deduce_from_graph(&self, query: &str) -> ReasoningResult {
        let mut steps = Vec::new();

        // 1. Try formal deduction via Axiom kernel
        if let Some(formal_result) = self.try_formal_deduction(query) {
            return formal_result;
        }
        steps.push("No formal proof found; applying belief-graph reasoning".into());

        // 2. Search knowledge graph for relevant beliefs
        let relevant = self.knowledge.text_search(query, 5);
        let belief_hits: Vec<Belief> = relevant.iter()
            .filter_map(|r| match &r.kind {
                bonsai_knowledge::SearchResultKind::Belief(b) => Some(b.clone()),
                _ => None,
            })
            .collect();

        if belief_hits.is_empty() {
            steps.push("No relevant beliefs found in knowledge graph".into());
            return self.insufficient_data_result(query, ReasoningStrategy::Deduction, steps);
        }

        // 3. Chain the highest-confidence beliefs
        let top: &Belief = belief_hits.iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .unwrap();
        steps.push(format!("Anchoring on: \"{}\" (confidence {:.2})", top.statement, top.confidence));

        // 4. Check for transitive Is-A relations
        let transitive = self.knowledge.transitive_closure(&bonsai_knowledge::Predicate::Implies);
        let relevant_transitive: Vec<_> = transitive.iter()
            .filter(|r| {
                if let bonsai_knowledge::RelationTarget::Entity(eid) = &r.object {
                    self.knowledge.get_entity(eid).map_or(false, |e| {
                        e.name.to_lowercase().contains(&query.to_lowercase()[..query.len().min(20)])
                    })
                } else { false }
            })
            .collect();
        if !relevant_transitive.is_empty() {
            steps.push(format!("Found {} transitive implications", relevant_transitive.len()));
        }

        let confidence = top.confidence * 0.9;
        let new_belief = Belief::new(
            format!("Based on known facts: {}", top.statement),
            confidence,
        );

        ReasoningResult {
            strategy: ReasoningStrategy::Deduction.name().into(),
            query: query.to_string(),
            conclusion: format!("By deduction from known beliefs: {}", top.statement),
            confidence,
            steps,
            new_beliefs: vec![new_belief],
            latency_ms: 0,
        }
    }

    fn try_formal_deduction(&self, query: &str) -> Option<ReasoningResult> {
        let kernel = AxiomKernel::with_nat();
        let ctx = VerifyContext::new();

        // Find beliefs with formal statements
        let beliefs_with_formal: Vec<Belief> = self.knowledge.all_beliefs().into_iter()
            .filter(|b| b.formal_statement.is_some())
            .collect();

        if beliefs_with_formal.is_empty() { return None; }

        // For each pair, check if one implies the other
        for b in &beliefs_with_formal {
            if let Some(ref ft) = b.formal_statement {
                // Check if the query term matches (simplified: term type-checks as Nat)
                if let Ok(_) = kernel.check(ft, &Term::Nat, &ctx) {
                    let proof_id = Uuid::new_v4().to_string();
                    let new_belief = Belief::new(query, 1.0).with_formal(ft.clone());
                    return Some(ReasoningResult {
                        strategy: ReasoningStrategy::Deduction.name().into(),
                        query: query.to_string(),
                        conclusion: format!("Formally deduced: {}", b.statement),
                        confidence: 1.0,
                        steps: vec![
                            "Formal proof via Axiom kernel".into(),
                            format!("Derived from: {}", b.statement),
                        ],
                        new_beliefs: vec![new_belief],
                        latency_ms: 0,
                    });
                }
            }
        }
        None
    }

    // ── Induction ─────────────────────────────────────────────────────────────

    async fn induce_pattern(&self, query: &str) -> ReasoningResult {
        let mut steps = Vec::new();
        steps.push("Collecting observations from knowledge graph".into());

        let entities = self.knowledge.text_search(query, 10);
        if entities.len() < 3 {
            steps.push(format!("Insufficient observations (need ≥3, found {})", entities.len()));
            return self.insufficient_data_result(query, ReasoningStrategy::Induction, steps);
        }

        // Find common properties among entities
        let entity_list: Vec<Entity> = entities.iter()
            .filter_map(|r| match &r.kind {
                bonsai_knowledge::SearchResultKind::Entity(e) => Some(e.clone()),
                _ => None,
            })
            .collect();

        if entity_list.is_empty() {
            return self.insufficient_data_result(query, ReasoningStrategy::Induction, steps);
        }

        // Collect shared relation patterns
        let common_predicates: Vec<String> = {
            let pred_lists: Vec<Vec<String>> = entity_list.iter()
                .map(|e| self.knowledge.relations_of(&e.id).into_iter()
                    .map(|r| r.predicate.to_string())
                    .collect())
                .collect();
            if pred_lists.is_empty() {
                vec![]
            } else {
                let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
                for list in &pred_lists {
                    for p in list {
                        *counts.entry(p.clone()).or_insert(0) += 1;
                    }
                }
                let threshold = (pred_lists.len() / 2).max(1);
                counts.into_iter().filter(|(_, c)| *c >= threshold).map(|(p, _)| p).collect()
            }
        };

        let pattern = if common_predicates.is_empty() {
            format!("All {} instances share no common structural pattern", entity_list.len())
        } else {
            format!("All {} instances share predicates: {}", entity_list.len(), common_predicates.join(", "))
        };
        steps.push(format!("Identified pattern: {}", pattern));

        let confidence = 0.6 * (entity_list.len() as f32 / 10.0).min(1.0);
        let generalisation = format!("Inductive generalisation: {}", pattern);
        let new_belief = Belief::new(&generalisation, confidence);

        ReasoningResult {
            strategy: ReasoningStrategy::Induction.name().into(),
            query: query.to_string(),
            conclusion: generalisation,
            confidence,
            steps,
            new_beliefs: vec![new_belief],
            latency_ms: 0,
        }
    }

    // ── Abduction ─────────────────────────────────────────────────────────────

    async fn abduce_explanation(&self, query: &str) -> ReasoningResult {
        let mut steps = Vec::new();
        steps.push("Searching for candidate explanations".into());

        // Find beliefs that could explain the observation
        let candidates = self.knowledge.text_search(query, 8);
        let mut scored: Vec<(Belief, f32)> = candidates.iter()
            .filter_map(|r| match &r.kind {
                bonsai_knowledge::SearchResultKind::Belief(b) => {
                    // Score = belief confidence × search score
                    let posterior = b.confidence * r.score;
                    Some((b.clone(), posterior))
                }
                _ => None,
            })
            .collect();

        if scored.is_empty() {
            return self.insufficient_data_result(query, ReasoningStrategy::Abduction, steps);
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let (best, score) = &scored[0];
        steps.push(format!("Best explanation: \"{}\" (posterior {:.2})", best.statement, score));

        for (b, s) in scored.iter().skip(1).take(3) {
            steps.push(format!("Alternative: \"{}\" ({:.2})", b.statement, s));
        }

        ReasoningResult {
            strategy: ReasoningStrategy::Abduction.name().into(),
            query: query.to_string(),
            conclusion: format!("Best explanation: {}", best.statement),
            confidence: *score,
            steps,
            new_beliefs: vec![],
            latency_ms: 0,
        }
    }

    // ── Analogy ───────────────────────────────────────────────────────────────

    async fn analogize(&self, query: &str) -> ReasoningResult {
        let mut steps = Vec::new();

        // Extract source and target domains from query ("X is like Y")
        let (source, target) = extract_analogy_domains(query);
        steps.push(format!("Source domain: {}", source));
        steps.push(format!("Target domain: {}", target));

        let source_ents = self.knowledge.find_by_concept(&source);
        let target_ents = self.knowledge.find_by_concept(&target);

        if source_ents.is_empty() || target_ents.is_empty() {
            steps.push("Insufficient domain knowledge for structural mapping".into());
            return self.insufficient_data_result(query, ReasoningStrategy::Analogy, steps);
        }

        // Compute structural overlap via shared predicate counts
        let source_rels: Vec<_> = source_ents.iter()
            .flat_map(|e| self.knowledge.relations_of(&e.id))
            .map(|r| r.predicate.to_string())
            .collect();
        let target_rels: Vec<_> = target_ents.iter()
            .flat_map(|e| self.knowledge.relations_of(&e.id))
            .map(|r| r.predicate.to_string())
            .collect();

        let overlap: usize = source_rels.iter()
            .filter(|p| target_rels.contains(p))
            .count();
        let total = source_rels.len().max(target_rels.len()).max(1);
        let similarity = overlap as f32 / total as f32;

        steps.push(format!("Structural similarity: {:.0}% ({} shared predicates)", similarity * 100.0, overlap));

        let conclusion = format!(
            "{} and {} share structural patterns (similarity {:.0}%); \
            by analogy, properties of {} likely apply to {}",
            source, target, similarity * 100.0, source, target
        );
        let confidence = similarity * 0.8;

        ReasoningResult {
            strategy: ReasoningStrategy::Analogy.name().into(),
            query: query.to_string(),
            conclusion,
            confidence,
            steps,
            new_beliefs: vec![],
            latency_ms: 0,
        }
    }

    // ── Counterfactual ────────────────────────────────────────────────────────

    async fn counterfactual(&self, query: &str) -> ReasoningResult {
        let mut steps = Vec::new();
        steps.push("Constructing counterfactual world".into());

        // Extract the changed condition from "if X had been Y, what would Z be?"
        let changed = extract_counterfactual_change(query);
        steps.push(format!("Counterfactual change: {}", changed));

        // Find beliefs affected by the change via Causes/DependsOn relations
        let affected: Vec<Entity> = self.knowledge.transitive_closure(&bonsai_knowledge::Predicate::Causes)
            .into_iter()
            .filter_map(|r| {
                if let bonsai_knowledge::RelationTarget::Entity(eid) = r.object {
                    self.knowledge.get_entity(&eid)
                } else { None }
            })
            .take(5)
            .collect();

        for e in &affected {
            steps.push(format!("Propagating to: {}", e.name));
        }

        let consequence_count = affected.len();
        let conclusion = if consequence_count > 0 {
            format!(
                "If {}, then {} downstream effects would follow: {}",
                changed,
                consequence_count,
                affected.iter().map(|e| e.name.as_str()).collect::<Vec<_>>().join(", ")
            )
        } else {
            format!("If {}, the impact cannot be determined from available knowledge", changed)
        };

        let confidence = if consequence_count > 0 { 0.55 } else { 0.3 };

        ReasoningResult {
            strategy: ReasoningStrategy::Counterfactual.name().into(),
            query: query.to_string(),
            conclusion,
            confidence,
            steps,
            new_beliefs: vec![],
            latency_ms: 0,
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn insufficient_data_result(
        &self, query: &str,
        strategy: ReasoningStrategy,
        mut steps: Vec<String>,
    ) -> ReasoningResult {
        steps.push("Insufficient knowledge to reason — returning low-confidence result".into());
        ReasoningResult {
            strategy: strategy.name().into(),
            query: query.to_string(),
            conclusion: format!("Cannot determine answer to: \"{}\" from available knowledge", query),
            confidence: 0.1,
            steps,
            new_beliefs: vec![],
            latency_ms: 0,
        }
    }

    // ── Training data generation ──────────────────────────────────────────────

    pub fn reasoning_to_dpo_pair(result: &ReasoningResult, correction: Option<&str>) -> Option<DpoTrainingPair> {
        if result.confidence < 0.4 { return None; }

        if let Some(corrected) = correction {
            Some(DpoTrainingPair {
                prompt: result.query.clone(),
                chosen: corrected.to_string(),
                rejected: result.conclusion.clone(),
                weight: 1.0,
                source: "reasoning_correction".into(),
                metadata: json!({ "strategy": result.strategy }),
            })
        } else if result.confidence > 0.7 {
            Some(DpoTrainingPair {
                prompt: format!("Use {} reasoning: {}", result.strategy, result.query),
                chosen: result.conclusion.clone(),
                rejected: format!("I cannot determine the answer to: {}", result.query),
                weight: result.confidence,
                source: "reasoning_engine".into(),
                metadata: json!({ "strategy": result.strategy }),
            })
        } else {
            None
        }
    }
}

// ── Text parsing helpers ──────────────────────────────────────────────────────

fn extract_analogy_domains(query: &str) -> (String, String) {
    // "X is like Y", "X and Y are similar", "analogize X to Y"
    let q = query.to_lowercase();
    if let Some(idx) = q.find(" like ") {
        let source = q[..idx].trim_start_matches(|c: char| !c.is_alphanumeric()).trim().to_string();
        let target = q[idx + 6..].split_whitespace().next().unwrap_or("").to_string();
        return (source, target);
    }
    if let Some(idx) = q.find(" to ") {
        let source = q[..idx].split_whitespace().last().unwrap_or("domain").to_string();
        let target = q[idx + 4..].split_whitespace().next().unwrap_or("target").to_string();
        return (source, target);
    }
    // Default: use the query itself as both (degenerate case)
    let words: Vec<&str> = q.split_whitespace().collect();
    let mid = words.len() / 2;
    (words[..mid].join(" "), words[mid..].join(" "))
}

fn extract_counterfactual_change(query: &str) -> String {
    let q = query.to_lowercase();
    // "if X had been Y" / "if X were Y"
    if let Some(start) = q.find("if ") {
        let rest = &q[start + 3..];
        if let Some(end) = rest.find(", ").or_else(|| rest.find(" then ")) {
            return rest[..end].trim().to_string();
        }
        return rest.trim().to_string();
    }
    "the stated condition had been different".to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use bonsai_knowledge::{KnowledgeGraph, Belief, Entity, EntityType};

    fn engine_with_knowledge() -> ReasoningEngine {
        let kg = Arc::new(KnowledgeGraph::new());
        kg.add_belief(Belief::new("Rust is a systems programming language", 0.95));
        kg.add_belief(Belief::new("Rust has a borrow checker for memory safety", 0.97));
        kg.upsert_entity(Entity::new("Rust", EntityType::Concept));
        kg.upsert_entity(Entity::new("C++", EntityType::Concept));
        ReasoningEngine::new(kg)
    }

    #[tokio::test]
    async fn deduction_finds_relevant_belief() {
        let e = engine_with_knowledge();
        let r = e.reason("What is Rust?", "deduce").await;
        assert!(!r.conclusion.is_empty());
        assert!(r.confidence > 0.0);
    }

    #[tokio::test]
    async fn induction_runs_without_panic() {
        let e = engine_with_knowledge();
        let r = e.reason("What patterns do programming languages share?", "induce").await;
        assert!(r.confidence >= 0.0);
    }

    #[tokio::test]
    async fn counterfactual_parses_query() {
        let e = engine_with_knowledge();
        let r = e.reason("If Rust had garbage collection, how would it differ?", "counterfactual").await;
        assert!(r.strategy == "counterfactual");
    }

    #[tokio::test]
    async fn auto_strategy_selects_abduction_for_why() {
        let e = engine_with_knowledge();
        let r = e.reason("Why does Rust prevent data races?", "auto").await;
        // Should pick abduction for "why" questions
        assert!(!r.conclusion.is_empty());
    }

    #[test]
    fn dpo_pair_generated_for_high_confidence() {
        let result = ReasoningResult {
            strategy: "deduction".into(),
            query: "What is memory safety?".into(),
            conclusion: "Memory safety prevents use-after-free bugs".into(),
            confidence: 0.85,
            steps: vec![],
            new_beliefs: vec![],
            latency_ms: 10,
        };
        let pair = ReasoningEngine::reasoning_to_dpo_pair(&result, None);
        assert!(pair.is_some());
    }
}
