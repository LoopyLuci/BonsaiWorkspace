//! Knowledge graph tools registered in the ToolRegistry.
//! Three tools: `reason`, `query_knowledge`, `fact_check`.

use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::sync::RwLock;

use bonsai_knowledge::{Belief, KnowledgeGraph};
use crate::tool_registry::{Tool, ToolResult};
use crate::reasoning_engine::ReasoningEngine;
use crate::belief_reviser::BeliefReviser;

// ── ReasonTool ────────────────────────────────────────────────────────────────

pub struct ReasonTool {
    pub engine: Arc<ReasoningEngine>,
}

#[async_trait]
impl Tool for ReasonTool {
    fn name(&self) -> &str { "reason" }
    fn description(&self) -> &str {
        "Reason about a question using deduction, induction, abduction, analogy, or counterfactual reasoning."
    }

    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let query = args["query"].as_str()
            .ok_or("missing field: query")?;
        let strategy = args["strategy"].as_str().unwrap_or("auto");

        let result = self.engine.reason(query, strategy).await;
        Ok(ToolResult::json(&json!({
            "strategy": result.strategy,
            "conclusion": result.conclusion,
            "confidence": result.confidence,
            "steps": result.steps,
            "latency_ms": result.latency_ms,
        })))
    }
}

// ── KnowledgeQueryTool ────────────────────────────────────────────────────────

pub struct KnowledgeQueryTool {
    pub knowledge: Arc<KnowledgeGraph>,
}

#[async_trait]
impl Tool for KnowledgeQueryTool {
    fn name(&self) -> &str { "query_knowledge" }
    fn description(&self) -> &str {
        "Search the Bonsai knowledge graph for entities, relationships, and beliefs."
    }

    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let query = args["query"].as_str()
            .ok_or("missing field: query")?;
        let top_k = args["top_k"].as_u64().unwrap_or(10) as usize;

        let results = self.knowledge.text_search(query, top_k);
        let stats = self.knowledge.stats();

        let items: Vec<Value> = results.iter().map(|r| {
            let (kind, name, confidence) = match &r.kind {
                bonsai_knowledge::SearchResultKind::Entity(e) =>
                    ("entity", e.name.as_str(), e.confidence),
                bonsai_knowledge::SearchResultKind::Belief(b) =>
                    ("belief", b.statement.as_str(), b.confidence),
            };
            json!({ "kind": kind, "text": name, "confidence": confidence, "score": r.score })
        }).collect();

        Ok(ToolResult::json(&json!({
            "results": items,
            "total_found": items.len(),
            "graph_stats": stats,
        })))
    }
}

// ── FactCheckTool ─────────────────────────────────────────────────────────────

pub struct FactCheckTool {
    pub knowledge: Arc<KnowledgeGraph>,
    pub belief_reviser: Arc<RwLock<BeliefReviser>>,
}

#[async_trait]
impl Tool for FactCheckTool {
    fn name(&self) -> &str { "fact_check" }
    fn description(&self) -> &str {
        "Check whether a statement is consistent with Bonsai's knowledge graph."
    }

    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let statement = args["statement"].as_str()
            .ok_or("missing field: statement")?;

        let all_beliefs = self.knowledge.all_beliefs();
        let result = self.belief_reviser.read().await
            .check_consistency(&all_beliefs, statement);

        let (status, details) = match &result {
            crate::belief_reviser::ConsistencyResult::Consistent => {
                ("consistent", json!({ "message": "No contradictions found in knowledge graph." }))
            }
            crate::belief_reviser::ConsistencyResult::Contradicts { conflicting, max_conflict_confidence } => {
                ("contradicts", json!({
                    "message": format!("Statement contradicts {} known belief(s)", conflicting.len()),
                    "conflicting_belief_ids": conflicting,
                    "max_conflict_confidence": max_conflict_confidence,
                }))
            }
            crate::belief_reviser::ConsistencyResult::Uncertain => {
                ("uncertain", json!({ "message": "Cannot determine consistency from available knowledge." }))
            }
        };

        Ok(ToolResult::json(&json!({
            "statement": statement,
            "status": status,
            "details": details,
        })))
    }
}

// ── Registration helper ───────────────────────────────────────────────────────

pub async fn register_knowledge_tools(
    registry: &crate::tool_registry::ToolRegistryState,
    engine: Arc<ReasoningEngine>,
    knowledge: Arc<KnowledgeGraph>,
    belief_reviser: Arc<RwLock<BeliefReviser>>,
) {
    registry.registry.register(Box::new(ReasonTool { engine })).await;
    registry.registry.register(Box::new(KnowledgeQueryTool { knowledge: knowledge.clone() })).await;
    registry.registry.register(Box::new(FactCheckTool { knowledge, belief_reviser })).await;
}
