use crate::tool_core::ToolRegistry;
/// Semantic tool selector — keyword-index approach, no embedding model required.
///
/// Scores tools by how many of the query's tokens appear in each tool's
/// keyword index (name + description + tags). Returns the top-K tool names
/// most relevant to the user's query.
///
/// Upgrade path: replace `keyword_score()` with cosine similarity against
/// pre-computed nomic-embed-text embeddings for a full semantic selector.
use std::collections::HashMap;

pub struct ToolSelector {
    /// term → list of tool names that mention it
    index: HashMap<String, Vec<String>>,
    /// All tool names (for fallback full-set queries)
    all_names: Vec<String>,
}

impl ToolSelector {
    /// Build the keyword index from a registry snapshot.
    pub fn build(registry: &ToolRegistry) -> Self {
        let mut index: HashMap<String, Vec<String>> = HashMap::new();

        for def in registry.all_definitions() {
            let name = def
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let desc = def
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let tags: Vec<String> = def
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| t.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default();

            let text = format!("{name} {desc} {}", tags.join(" "));
            for token in tokenize(&text) {
                index.entry(token).or_default().push(name.clone());
            }
        }

        let all_names = registry.names();
        Self { index, all_names }
    }

    /// Select up to `top_k` tool names most relevant to `query`.
    ///
    /// `previously_selected` is the list from the prior ReAct iteration —
    /// we boost tools that were already selected to maintain context continuity.
    ///
    /// Returns names sorted by relevance score descending.
    pub fn select(&self, query: &str, top_k: usize, previously_selected: &[String]) -> Vec<String> {
        if self.all_names.is_empty() {
            return Vec::new();
        }

        let tokens = tokenize(query);
        if tokens.is_empty() {
            // No query signal — return first top_k tools deterministically
            return self.all_names.iter().take(top_k).cloned().collect();
        }

        let mut scores: HashMap<String, f32> = HashMap::new();

        for token in &tokens {
            if let Some(matches) = self.index.get(token) {
                for name in matches {
                    *scores.entry(name.clone()).or_default() += 1.0;
                }
            }
            // Stemmed lookup (drop last char)
            if token.len() > 3 {
                let stem = &token[..token.len() - 1];
                if let Some(matches) = self.index.get(stem) {
                    for name in matches {
                        *scores.entry(name.clone()).or_default() += 0.5;
                    }
                }
            }
        }

        // Continuity boost: slightly prefer tools that were selected last iter
        for name in previously_selected {
            *scores.entry(name.clone()).or_default() += 0.3;
        }

        // Sort by score descending, break ties alphabetically
        let mut ranked: Vec<(String, f32)> = scores.into_iter().collect();
        ranked.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });

        // Take top_k; fall back to all tools if fewer than top_k scored
        let mut result: Vec<String> = ranked
            .into_iter()
            .take(top_k)
            .map(|(name, _)| name)
            .collect();

        // If fewer than top_k matched, pad with unscored tools in stable order
        if result.len() < top_k {
            for name in &self.all_names {
                if result.len() >= top_k {
                    break;
                }
                if !result.contains(name) {
                    result.push(name.clone());
                }
            }
        }

        result
    }

    /// Return ALL tool names (used when the selector can't confidently narrow down).
    pub fn all(&self) -> Vec<String> {
        self.all_names.clone()
    }

    pub fn tool_count(&self) -> usize {
        self.all_names.len()
    }
}

// ── Tokenizer ─────────────────────────────────────────────────────────────────

fn tokenize(text: &str) -> Vec<String> {
    // Split on whitespace and punctuation, lowercase, filter short tokens,
    // deduplicate (preserving first-occurrence order).
    let mut seen = std::collections::HashSet::new();
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_lowercase())
        .filter(|t| !STOP_WORDS.contains(t.as_str()))
        .filter(|t| seen.insert(t.clone()))
        .collect()
}

static STOP_WORDS: std::sync::LazyLock<std::collections::HashSet<&'static str>> =
    std::sync::LazyLock::new(|| {
        [
            "the", "and", "for", "from", "with", "that", "this", "are", "you", "can", "use", "get",
            "set", "all", "any", "new", "not", "but", "its", "into", "via", "per", "etc",
        ]
        .into_iter()
        .collect()
    });

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_registry() -> ToolRegistry {
        use crate::assistant_tools::build_registry;
        build_registry()
    }

    #[test]
    fn selects_weather_for_weather_query() {
        let reg = mock_registry();
        let sel = ToolSelector::build(&reg);
        let result = sel.select("what is the weather in Tokyo", 4, &[]);
        assert!(
            result.contains(&"get_weather".to_string()),
            "weather tool should be selected"
        );
    }

    #[test]
    fn selects_datetime_for_time_query() {
        let reg = mock_registry();
        let sel = ToolSelector::build(&reg);
        let result = sel.select("what time is it now", 4, &[]);
        assert!(
            result.contains(&"get_datetime".to_string()),
            "datetime tool should be selected"
        );
    }

    #[test]
    fn fallback_returns_tools_when_no_match() {
        let reg = mock_registry();
        let sel = ToolSelector::build(&reg);
        let result = sel.select("xyzzy frobnicate", 4, &[]);
        assert!(!result.is_empty(), "should return tools even with no match");
    }

    #[test]
    fn selection_is_deterministic_for_same_query() {
        let reg = mock_registry();
        let sel = ToolSelector::build(&reg);

        let first = sel.select("read and find files in workspace", 6, &[]);
        let second = sel.select("read and find files in workspace", 6, &[]);

        assert_eq!(
            first, second,
            "selector output should be stable for identical input"
        );
    }

    #[test]
    fn continuity_boost_keeps_previous_tool_in_set() {
        let reg = mock_registry();
        let sel = ToolSelector::build(&reg);

        let result = sel.select(
            "show current time and system status",
            4,
            &["get_datetime".to_string()],
        );

        assert!(
            result.contains(&"get_datetime".to_string()),
            "previously selected tool should receive continuity boost"
        );
    }
}
