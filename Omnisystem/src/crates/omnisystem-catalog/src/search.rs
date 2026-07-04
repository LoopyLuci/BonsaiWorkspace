pub struct SearchEngine;

impl SearchEngine {
    pub fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    pub fn calculate_relevance(
        query_tokens: &[String],
        text: &str,
    ) -> f32 {
        let text_tokens = Self::tokenize(text);
        let matches = query_tokens
            .iter()
            .filter(|q| text_tokens.contains(q))
            .count();
        if query_tokens.is_empty() {
            0.0
        } else {
            matches as f32 / query_tokens.len() as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokens = SearchEngine::tokenize("Hello World");
        assert_eq!(tokens.len(), 2);
    }

    #[test]
    fn test_calculate_relevance() {
        let query = vec!["hello".to_string(), "world".to_string()];
        let relevance = SearchEngine::calculate_relevance(&query, "hello there");
        assert!(relevance > 0.0);
    }
}
