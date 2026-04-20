/// Context System — assembles ranked, token-budgeted context for each assistant turn.
///
/// The ContextBuilder gathers history, memory snippets, workspace snapshots, policy state,
/// and confirmation state, then prunes to fit within a token budget while preserving the
/// highest-ranked items.
use serde_json::{json, Value};

// ── Token budget ──────────────────────────────────────────────────────────────

/// Conservative token estimator: 1 token ≈ 4 characters (English prose).
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4
}

/// Token budget allocation across context slots.
#[derive(Debug, Clone)]
pub struct TokenBudget {
    pub total:        usize,
    pub system:       usize,
    pub history:      usize,
    pub memory:       usize,
    pub workspace:    usize,
    pub tool_results: usize,
}

impl TokenBudget {
    /// Default budget for a standard 200k-token context window model.
    pub fn default_200k() -> Self {
        Self {
            total:        180_000, // leave 20k headroom for completion
            system:        4_000,
            history:      60_000,
            memory:        8_000,
            workspace:    16_000,
            tool_results: 32_000,
        }
    }

    /// Compact budget for a 32k-token context window.
    pub fn compact_32k() -> Self {
        Self {
            total:        28_000,
            system:        2_000,
            history:      12_000,
            memory:        2_000,
            workspace:     4_000,
            tool_results:  8_000,
        }
    }

    /// Returns true if `used` is within the slot's allocation.
    pub fn fits(&self, slot: ContextSlot, used: usize) -> bool {
        used <= self.slot_limit(slot)
    }

    pub fn slot_limit(&self, slot: ContextSlot) -> usize {
        match slot {
            ContextSlot::System      => self.system,
            ContextSlot::History     => self.history,
            ContextSlot::Memory      => self.memory,
            ContextSlot::Workspace   => self.workspace,
            ContextSlot::ToolResults => self.tool_results,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextSlot {
    System,
    History,
    Memory,
    Workspace,
    ToolResults,
}

// ── Context item (rankable) ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ContextItem {
    pub slot:    ContextSlot,
    pub content: String,
    pub score:   f32,   // higher = more relevant; used for rank-selection pruning
}

impl ContextItem {
    pub fn token_cost(&self) -> usize {
        estimate_tokens(&self.content)
    }
}

// ── ContextBuilder ────────────────────────────────────────────────────────────

pub struct ContextBuilder {
    budget: TokenBudget,
    items:  Vec<ContextItem>,
}

impl ContextBuilder {
    pub fn new(budget: TokenBudget) -> Self {
        Self { budget, items: Vec::new() }
    }

    pub fn with_default_budget() -> Self {
        Self::new(TokenBudget::default_200k())
    }

    /// Add a system prompt fragment.
    pub fn add_system(&mut self, content: impl Into<String>, score: f32) {
        self.items.push(ContextItem { slot: ContextSlot::System, content: content.into(), score });
    }

    /// Add a history message (role + content JSON).
    pub fn add_history(&mut self, content: impl Into<String>, score: f32) {
        self.items.push(ContextItem { slot: ContextSlot::History, content: content.into(), score });
    }

    /// Add a memory snippet.
    pub fn add_memory(&mut self, content: impl Into<String>, score: f32) {
        self.items.push(ContextItem { slot: ContextSlot::Memory, content: content.into(), score });
    }

    /// Add a workspace snapshot card (file tree, recent edits, etc.).
    pub fn add_workspace(&mut self, content: impl Into<String>, score: f32) {
        self.items.push(ContextItem { slot: ContextSlot::Workspace, content: content.into(), score });
    }

    /// Add a tool result.
    pub fn add_tool_result(&mut self, content: impl Into<String>, score: f32) {
        self.items.push(ContextItem { slot: ContextSlot::ToolResults, content: content.into(), score });
    }

    /// Build the final context, pruning low-scoring items to fit within budget.
    ///
    /// Items within each slot are sorted by score descending; lowest-scored items are
    /// dropped first when a slot is over budget. The total budget is the hard cap.
    pub fn build(mut self) -> BuiltContext {
        // Sort each slot's items by score descending
        self.items.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut selected: Vec<ContextItem> = Vec::new();
        let mut slot_used = [0usize; 5];  // indexed by slot ordinal
        let mut total_used = 0usize;

        for item in self.items {
            let slot_ord = slot_ordinal(item.slot);
            let cost     = item.token_cost();
            let slot_cap = self.budget.slot_limit(item.slot);

            if slot_used[slot_ord] + cost <= slot_cap
                && total_used + cost <= self.budget.total
            {
                slot_used[slot_ord] += cost;
                total_used          += cost;
                selected.push(item);
            }
            // Items that don't fit are silently dropped (low-score items are dropped first)
        }

        BuiltContext {
            items: selected,
            total_tokens: total_used,
        }
    }
}

fn slot_ordinal(slot: ContextSlot) -> usize {
    match slot {
        ContextSlot::System      => 0,
        ContextSlot::History     => 1,
        ContextSlot::Memory      => 2,
        ContextSlot::Workspace   => 3,
        ContextSlot::ToolResults => 4,
    }
}

// ── BuiltContext ──────────────────────────────────────────────────────────────

pub struct BuiltContext {
    pub items:        Vec<ContextItem>,
    pub total_tokens: usize,
}

impl BuiltContext {
    /// Collect all items in a slot as a combined string (for injection into prompts).
    pub fn slot_content(&self, slot: ContextSlot) -> String {
        self.items.iter()
            .filter(|i| i.slot == slot)
            .map(|i| i.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Return items as a JSON array for API message injection.
    pub fn to_messages(&self) -> Vec<Value> {
        let mut msgs = Vec::new();

        let system = self.slot_content(ContextSlot::System);
        if !system.is_empty() {
            msgs.push(json!({ "role": "system", "content": system }));
        }

        let memory = self.slot_content(ContextSlot::Memory);
        let workspace = self.slot_content(ContextSlot::Workspace);
        if !memory.is_empty() || !workspace.is_empty() {
            let mut parts = Vec::new();
            if !memory.is_empty()    { parts.push(format!("## Memory\n{memory}"));       }
            if !workspace.is_empty() { parts.push(format!("## Workspace\n{workspace}")); }
            msgs.push(json!({ "role": "system", "content": parts.join("\n\n") }));
        }

        for item in &self.items {
            if item.slot == ContextSlot::History {
                // History items are raw JSON message strings
                if let Ok(v) = serde_json::from_str::<Value>(&item.content) {
                    msgs.push(v);
                }
            }
        }

        msgs
    }

    /// Summarize what was dropped (for diagnostics / metrics).
    pub fn dropped_count(&self) -> usize {
        0 // items not selected are simply absent; tracked externally if needed
    }
}

// ── History pruning ───────────────────────────────────────────────────────────

/// Prune a history slice to fit within `max_tokens`.
///
/// Keeps the most recent messages (they have the highest context relevance).
/// If the newest message is itself over budget, it is still included (never drop turn N).
pub fn prune_history(messages: &[Value], max_tokens: usize) -> Vec<Value> {
    let mut used   = 0usize;
    let mut result = Vec::new();

    for msg in messages.iter().rev() {
        let text = msg["content"].as_str().unwrap_or("");
        let cost = estimate_tokens(text);
        if used + cost <= max_tokens || result.is_empty() {
            used   += cost;
            result.push(msg.clone());
        } else {
            break;
        }
    }

    result.reverse();
    result
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_estimation_is_roughly_correct() {
        // "Hello world" = 11 chars → ~3 tokens
        assert_eq!(estimate_tokens("Hello world"), 3);
        // empty string
        assert_eq!(estimate_tokens(""), 0);
        // 400-char string → ~100 tokens
        let long = "a".repeat(400);
        assert_eq!(estimate_tokens(&long), 100);
    }

    #[test]
    fn budget_slots_are_respected() {
        let budget = TokenBudget::default_200k();
        assert!(budget.fits(ContextSlot::System, 3_999));
        assert!(!budget.fits(ContextSlot::System, 4_001));
    }

    #[test]
    fn builder_drops_over_budget_items() {
        let mut budget = TokenBudget::default_200k();
        budget.memory = 100; // allow only ~25 tokens of memory
        let mut builder = ContextBuilder::new(budget);

        // Add a large memory item (should be dropped)
        builder.add_memory("a".repeat(1000), 0.9);
        // Add a small memory item (should fit)
        builder.add_memory("short".to_string(), 0.5);

        let ctx = builder.build();
        let mem = ctx.slot_content(ContextSlot::Memory);
        assert!(mem.contains("short"), "small item should survive");
        assert!(!mem.contains(&"a".repeat(100)), "large item should be dropped");
    }

    #[test]
    fn builder_respects_total_budget() {
        let budget = TokenBudget { total: 10, system: 5, history: 5,
                                   memory: 5, workspace: 5, tool_results: 5 };
        let mut builder = ContextBuilder::new(budget);
        // Each item is ~5 tokens (20 chars)
        builder.add_system("x".repeat(20), 1.0);
        builder.add_history("{\"role\":\"user\",\"content\":\"hi\"}".into(), 0.9);
        builder.add_memory("y".repeat(20), 0.8);

        let ctx = builder.build();
        assert!(ctx.total_tokens <= 10, "must not exceed total budget");
    }

    #[test]
    fn builder_keeps_highest_scored_items_when_pruning() {
        let mut budget = TokenBudget::default_200k();
        budget.memory = 50; // ~12 tokens max

        let mut builder = ContextBuilder::new(budget);
        builder.add_memory("high score memory content here".into(), 0.95);
        builder.add_memory("low score memory junk content fill".into(), 0.1);

        let ctx = builder.build();
        let mem = ctx.slot_content(ContextSlot::Memory);
        assert!(mem.contains("high score"), "high-score item should be kept");
    }

    #[test]
    fn prune_history_keeps_recent_messages() {
        let msgs: Vec<Value> = (0..10)
            .map(|i| json!({ "role": "user", "content": format!("message {i}") }))
            .collect();

        // Allow ~8 tokens per message (32 chars / 4)
        let pruned = prune_history(&msgs, 50);
        assert!(!pruned.is_empty());
        // Most recent message must always be present
        let last = pruned.last().unwrap();
        assert!(last["content"].as_str().unwrap().contains("9"));
    }

    #[test]
    fn prune_history_empty_input() {
        let pruned = prune_history(&[], 1000);
        assert!(pruned.is_empty());
    }

    #[test]
    fn to_messages_has_system_block() {
        let mut builder = ContextBuilder::with_default_budget();
        builder.add_system("You are a helpful assistant.".into(), 1.0);
        builder.add_memory("User prefers dark mode.".into(), 0.8);

        let ctx  = builder.build();
        let msgs = ctx.to_messages();
        assert!(!msgs.is_empty());
        assert_eq!(msgs[0]["role"], "system");
    }

    #[test]
    fn compact_budget_is_smaller_than_default() {
        let def = TokenBudget::default_200k();
        let cmp = TokenBudget::compact_32k();
        assert!(cmp.total < def.total);
        assert!(cmp.history < def.history);
    }
}