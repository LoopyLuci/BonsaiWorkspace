//! Text Tokenization and Normalization Pipeline
//!
//! Converts raw text into normalized tokens for indexing and search.

use std::collections::HashSet;

/// Text tokenizer with configurable pipeline
pub struct Tokenizer {
    lowercase: bool,
    remove_punctuation: bool,
    remove_stopwords: bool,
    stopwords: HashSet<String>,
}

impl Tokenizer {
    pub fn new() -> Self {
        let mut stopwords = HashSet::new();
        for word in &[
            "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for",
            "of", "is", "are", "was", "were", "be", "been", "being",
        ] {
            stopwords.insert(word.to_string());
        }

        Self {
            lowercase: true,
            remove_punctuation: true,
            remove_stopwords: true,
            stopwords,
        }
    }

    pub fn with_lowercase(mut self, enabled: bool) -> Self {
        self.lowercase = enabled;
        self
    }

    pub fn with_punctuation_removal(mut self, enabled: bool) -> Self {
        self.remove_punctuation = enabled;
        self
    }

    pub fn with_stopword_removal(mut self, enabled: bool) -> Self {
        self.remove_stopwords = enabled;
        self
    }

    pub fn add_stopword(&mut self, word: String) {
        self.stopwords.insert(word);
    }

    /// Tokenize text into terms
    pub fn tokenize(&self, text: &str) -> Vec<String> {
        let mut tokens = Vec::new();

        for word in text.split_whitespace() {
            let mut token = word.to_string();

            // Remove punctuation
            if self.remove_punctuation {
                token = token
                    .chars()
                    .filter(|c| !c.is_ascii_punctuation())
                    .collect();
            }

            // Skip empty tokens
            if token.is_empty() {
                continue;
            }

            // Lowercase
            if self.lowercase {
                token = token.to_lowercase();
            }

            // Skip stopwords
            if self.remove_stopwords && self.stopwords.contains(&token) {
                continue;
            }

            tokens.push(token);
        }

        tokens
    }

    /// Tokenize and get unique terms
    pub fn tokenize_unique(&self, text: &str) -> Vec<String> {
        let mut unique = HashSet::new();
        for token in self.tokenize(text) {
            unique.insert(token);
        }

        let mut result: Vec<String> = unique.into_iter().collect();
        result.sort();
        result
    }
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokenization() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Hello, World! This is a test.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
    }

    #[test]
    fn test_lowercase() {
        let tokenizer = Tokenizer::new().with_lowercase(true);
        let tokens = tokenizer.tokenize("HELLO WORLD");
        assert_eq!(tokens[0], "hello");
    }

    #[test]
    fn test_punctuation_removal() {
        let tokenizer = Tokenizer::new().with_punctuation_removal(true);
        let tokens = tokenizer.tokenize("Hello, World!");
        assert_eq!(tokens[0], "hello");
        assert_eq!(tokens[1], "world");
    }

    #[test]
    fn test_stopword_removal() {
        let tokenizer = Tokenizer::new().with_stopword_removal(true);
        let tokens = tokenizer.tokenize("the cat is on the mat");
        // Should not contain "the", "is", "on"
        assert!(!tokens.contains(&"the".to_string()));
        assert!(!tokens.contains(&"is".to_string()));
        assert!(!tokens.contains(&"on".to_string()));
    }

    #[test]
    fn test_custom_stopwords() {
        let mut tokenizer = Tokenizer::new();
        tokenizer.add_stopword("test".to_string());
        let tokens = tokenizer.tokenize("this is a test");
        assert!(!tokens.contains(&"test".to_string()));
    }

    #[test]
    fn test_unique_tokens() {
        let tokenizer = Tokenizer::new();
        let unique = tokenizer.tokenize_unique("hello hello world world hello");
        assert_eq!(unique.len(), 2);
        assert!(unique.contains(&"hello".to_string()));
        assert!(unique.contains(&"world".to_string()));
    }

    #[test]
    fn test_disabled_processing() {
        let tokenizer = Tokenizer::new()
            .with_lowercase(false)
            .with_punctuation_removal(false);
        let tokens = tokenizer.tokenize("Hello, World");
        assert_eq!(tokens[0], "Hello,");
    }
}
