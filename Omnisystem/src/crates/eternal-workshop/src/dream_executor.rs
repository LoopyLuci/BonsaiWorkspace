//! DreamAgent client — consolidates memory nodes via a local llama-server.
//!
//! The DreamAgent is a small (1.5B–3B) model fine-tuned for summarization and
//! context pruning.  It runs as a sidecar on port 8082, loaded from
//! `D:/Models/bonsai-dreamagent.gguf` (or wherever the user placed it).
//!
//! If the sidecar is unavailable, we fall back to a deterministic heuristic
//! consolidation (dedup by content hash, keep most recent per topic).

use std::collections::HashSet;

use log::{info, warn};
use serde_json::json;

use crate::config::Config;
use crate::memory_nodes::MemoryNode;

const SYSTEM_PROMPT: &str =
    "You are the Bonsai Memory Consolidator. You receive a JSON array of raw activity \
     nodes from a programming session. Your job is to:\n\
     1. Merge related information into concise, high-value insights.\n\
     2. Remove duplicate or trivial entries (e.g., repeated keystrokes, minor cursor moves).\n\
     3. Preserve important facts: errors fixed, decisions made, code patterns learned.\n\
     4. Output ONLY a JSON array of consolidated MemoryNode objects with fields: \
     {id, node_type, source, content, tags}.\n\
     Be concise. Prefer one rich insight over five shallow ones.";

/// Run a full dream cycle: fetch pending nodes → call DreamAgent → mark consolidated.
/// Returns a human-readable summary string for the UI toast.
pub async fn run_dream_cycle(
    store: &crate::memory_nodes::MemoryNodeStore,
    cfg: &Config,
    workspace_path: Option<&std::path::Path>,
) -> Result<String, String> {
    let nodes = store
        .get_pending_nodes()
        .await
        .map_err(|e| format!("db read failed: {e}"))?;

    if nodes.is_empty() {
        return Ok("No new activity to consolidate.".to_string());
    }

    info!("[dream] consolidating {} nodes", nodes.len());

    let consolidated = if is_dream_agent_available(cfg).await {
        call_dream_agent(&nodes, cfg).await.unwrap_or_else(|e| {
            warn!("[dream] DreamAgent unavailable: {e}. Using heuristic fallback.");
            heuristic_consolidate(&nodes)
        })
    } else {
        info!("[dream] DreamAgent not running — using heuristic consolidation");
        heuristic_consolidate(&nodes)
    };

    // Mark all processed nodes as consolidated
    let ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    store
        .mark_consolidated(&ids)
        .await
        .map_err(|e| format!("mark_consolidated failed: {e}"))?;

    // Update BONSAI.md if workspace path is known
    if let Some(ws) = workspace_path {
        let learnings = nodes_to_learnings(&consolidated);
        let bonsai_path = ws.join("BONSAI.md");
        append_learnings_to_bonsai_md(&bonsai_path, &learnings);
    }

    let summary = format!(
        "Consolidated {} activity nodes into {} insights.",
        nodes.len(),
        consolidated.len()
    );
    info!("[dream] {}", summary);
    Ok(summary)
}

async fn is_dream_agent_available(cfg: &Config) -> bool {
    let client = reqwest::Client::new();
    client
        .get(format!("{}/health", cfg.dream_agent_url()))
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

async fn call_dream_agent(nodes: &[MemoryNode], cfg: &Config) -> Result<Vec<MemoryNode>, String> {
    // Batch into chunks of 50 nodes to stay within context limits
    let mut all_consolidated = Vec::new();
    for chunk in nodes.chunks(50) {
        let batch_json =
            serde_json::to_string(chunk).map_err(|e| format!("serialize failed: {e}"))?;

        let payload = json!({
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user",   "content": format!("Consolidate these nodes:\n\n{batch_json}")}
            ],
            "temperature": 0.1,
            "max_tokens":  4096,
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/v1/chat/completions", cfg.dream_agent_url()))
            .json(&payload)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("response parse failed: {e}"))?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("[]");

        // Try to extract JSON array from response (model may add prose before/after)
        let array_start = content.find('[').unwrap_or(0);
        let array_end = content.rfind(']').map(|i| i + 1).unwrap_or(content.len());
        let json_part = &content[array_start..array_end];

        if let Ok(batch_result) = serde_json::from_str::<Vec<MemoryNode>>(json_part) {
            all_consolidated.extend(batch_result);
        } else {
            warn!("[dream] failed to parse DreamAgent response as JSON array — keeping raw nodes");
            all_consolidated.extend_from_slice(chunk);
        }
    }

    Ok(all_consolidated)
}

/// Heuristic consolidation when DreamAgent is unavailable.
/// Deduplicates by content hash and keeps the most recent per unique content prefix.
fn heuristic_consolidate(nodes: &[MemoryNode]) -> Vec<MemoryNode> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut result = Vec::new();

    // Process most recent first, keep first unique
    for node in nodes.iter().rev() {
        // Use first 100 chars as dedup key
        let key: String = node.content.chars().take(100).collect();
        if seen.insert(key) {
            result.push(node.clone());
        }
    }

    // Restore chronological order
    result.reverse();

    // Cap at 200 consolidated nodes
    if result.len() > 200 {
        result.truncate(200);
    }

    result
}

fn nodes_to_learnings(nodes: &[MemoryNode]) -> String {
    let mut lines = Vec::new();
    for node in nodes.iter().take(20) {
        let preview: String = node.content.chars().take(120).collect();
        lines.push(format!("- [{}] {}", node.node_type, preview));
    }
    if nodes.len() > 20 {
        lines.push(format!(
            "- *(+{} more consolidated insights)*",
            nodes.len() - 20
        ));
    }
    lines.join("\n")
}

fn append_learnings_to_bonsai_md(path: &std::path::Path, learnings: &str) {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    const MARKER: &str = "## Active Context";
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M");

    let new_content = if let Some(pos) = existing.find(MARKER) {
        format!(
            "{}{MARKER}\n*(Updated: {timestamp})*\n\n{learnings}\n",
            &existing[..pos],
        )
    } else {
        format!("{existing}\n\n{MARKER}\n*(Updated: {timestamp})*\n\n{learnings}\n")
    };

    let _ = std::fs::write(path, new_content);
    info!("[dream] BONSAI.md updated at {}", path.display());
}
