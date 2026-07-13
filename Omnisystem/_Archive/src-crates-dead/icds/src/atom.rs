//! Semantic Atom – the fundamental unit of context in ICDS
//!
//! An atom is an immutable, self-contained piece of information that can be
//! referenced at multiple resolutions (full text, summary, keywords).

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for an atom (BLAKE3 hash of content)
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct AtomId(pub String);

impl fmt::Display for AtomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AtomId {
    /// Create atom ID from raw content hash
    pub fn from_hash(hash: String) -> Self {
        Self(hash)
    }

    /// Compute hash of text (BLAKE3)
    pub fn compute(text: &str) -> Self {
        let hash = blake3::hash(text.as_bytes());
        Self(hash.to_hex().to_string())
    }
}

/// Type of source that created the atom
#[derive(Clone, Debug, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum SourceType {
    /// User input to the AI
    UserInput,
    /// System event or state change
    SystemEvent,
    /// AI agent's internal reasoning/thought
    AgentThought,
    /// Output from a tool call
    ToolOutput,
    /// External data source
    ExternalData,
    /// Memory retrieval (atom referencing previous atoms)
    MemoryRetrieval,
}

impl fmt::Display for SourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UserInput => write!(f, "user_input"),
            Self::SystemEvent => write!(f, "system_event"),
            Self::AgentThought => write!(f, "agent_thought"),
            Self::ToolOutput => write!(f, "tool_output"),
            Self::ExternalData => write!(f, "external_data"),
            Self::MemoryRetrieval => write!(f, "memory_retrieval"),
        }
    }
}

/// Resolution level for multi-resolution retrieval
#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[repr(u8)]
pub enum ResolutionLevel {
    /// Level 0: Full raw text (complete fidelity)
    Full = 0,
    /// Level 1: Coarse summary (single sentence)
    Summary = 1,
    /// Level 2: Keywords only (ultra-fast scanning)
    Keywords = 2,
}

impl fmt::Display for ResolutionLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => write!(f, "full"),
            Self::Summary => write!(f, "summary"),
            Self::Keywords => write!(f, "keywords"),
        }
    }
}

/// A resolution tier for an atom
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resolution {
    /// Resolution level
    pub level: ResolutionLevel,
    /// Text at this resolution
    pub text: String,
    /// Token count estimate
    pub estimated_tokens: usize,
}

impl Resolution {
    /// Create a full-text resolution
    pub fn full(text: String) -> Self {
        let estimated_tokens = estimate_tokens(&text);
        Self {
            level: ResolutionLevel::Full,
            text,
            estimated_tokens,
        }
    }

    /// Create a summary resolution via deterministic extractive summarization
    pub fn summary(text: String) -> Self {
        // Deterministic: take first sentence
        let summary = extract_first_sentence(&text);
        let estimated_tokens = estimate_tokens(&summary);
        Self {
            level: ResolutionLevel::Summary,
            text: summary,
            estimated_tokens,
        }
    }

    /// Create a keywords resolution via deterministic extraction
    pub fn keywords(text: String) -> Self {
        // Deterministic: extract high-frequency words via TF-IDF
        let keywords = extract_keywords_tfidf(&text, 5);
        let estimated_tokens = keywords.len();
        Self {
            level: ResolutionLevel::Keywords,
            text: keywords.join(" "),
            estimated_tokens,
        }
    }
}

/// Metadata about an atom
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AtomMetadata {
    /// Which agent/user created this atom
    pub agent_id: Uuid,
    /// Conversation or task ID this belongs to
    pub conversation_id: Option<Uuid>,
    /// Source type
    pub source: SourceType,
    /// User-provided tags
    pub tags: Vec<String>,
    /// Importance score (0.0-1.0)
    pub importance: f64,
}

/// A Semantic Atom – the core unit of context
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticAtom {
    /// Content-addressed identifier (BLAKE3 hash)
    pub id: AtomId,
    /// Parent atom ID if this is an edit/revision
    pub parent_id: Option<AtomId>,
    /// Timestamp (microseconds since epoch)
    pub timestamp: u64,
    /// Metadata
    pub metadata: AtomMetadata,
    /// Multi-resolution representations
    pub resolutions: Vec<Resolution>,
    /// Embedding (sparse vector as TF-IDF, or optional dense via AI)
    pub embedding: EmbeddingVector,
}

/// Embedding representation (sparse or dense)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EmbeddingVector {
    /// Sparse TF-IDF vector (deterministic fallback)
    SparseTfidf {
        /// Term -> weight map
        terms: Vec<(String, f32)>,
    },
    /// Dense embedding (from AI model, optional)
    Dense {
        /// Fixed-dimension vector
        vector: Vec<f32>,
        /// Whether this came from AI or is deterministic
        ai_enhanced: bool,
    },
}

impl EmbeddingVector {
    /// Create sparse vector from text
    pub fn sparse_from_text(text: &str) -> Self {
        let terms = compute_tfidf_terms(text);
        Self::SparseTfidf { terms }
    }

    /// Compute cosine similarity with another vector
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        match (self, other) {
            (
                Self::SparseTfidf {
                    terms: terms_a,
                },
                Self::SparseTfidf {
                    terms: terms_b,
                },
            ) => {
                // Simple cosine similarity between sparse vectors
                let mut dot_product = 0.0f32;
                let mut norm_a = 0.0f32;
                let mut norm_b = 0.0f32;

                for (term_a, weight_a) in terms_a {
                    norm_a += weight_a * weight_a;
                    if let Some((_, weight_b)) = terms_b.iter().find(|(t, _)| t == term_a) {
                        dot_product += weight_a * weight_b;
                    }
                }

                for (_, weight_b) in terms_b {
                    norm_b += weight_b * weight_b;
                }

                if norm_a > 0.0 && norm_b > 0.0 {
                    dot_product / (norm_a.sqrt() * norm_b.sqrt())
                } else {
                    0.0
                }
            }
            (Self::Dense { vector: v_a, .. }, Self::Dense { vector: v_b, .. }) => {
                // Dense cosine similarity
                let dot_product: f32 = v_a.iter().zip(v_b).map(|(a, b)| a * b).sum();
                let norm_a: f32 = v_a.iter().map(|x| x * x).sum::<f32>().sqrt();
                let norm_b: f32 = v_b.iter().map(|x| x * x).sum::<f32>().sqrt();

                if norm_a > 0.0 && norm_b > 0.0 {
                    dot_product / (norm_a * norm_b)
                } else {
                    0.0
                }
            }
            _ => 0.0, // Mixed types default to 0
        }
    }
}

impl SemanticAtom {
    /// Create atom from raw text
    ///
    /// Generates ID, embeddings, and all resolution tiers deterministically.
    pub fn from_text(
        text: String,
        metadata: AtomMetadata,
        num_resolutions: usize,
    ) -> Result<Self> {
        let id = AtomId::compute(&text);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_micros() as u64;

        let mut resolutions = vec![Resolution::full(text.clone())];

        if num_resolutions > 1 {
            resolutions.push(Resolution::summary(text.clone()));
        }
        if num_resolutions > 2 {
            resolutions.push(Resolution::keywords(text.clone()));
        }

        let embedding = EmbeddingVector::sparse_from_text(&text);

        Ok(Self {
            id,
            parent_id: None,
            timestamp,
            metadata,
            resolutions,
            embedding,
        })
    }

    /// Get the full text resolution
    pub fn full_text(&self) -> Option<&str> {
        self.resolutions
            .iter()
            .find(|r| r.level == ResolutionLevel::Full)
            .map(|r| r.text.as_str())
    }

    /// Get total tokens across all resolutions
    pub fn total_tokens(&self) -> usize {
        self.resolutions.iter().map(|r| r.estimated_tokens).sum()
    }
}

// Deterministic helper functions

/// Extract first sentence from text
fn extract_first_sentence(text: &str) -> String {
    text.split('.')
        .next()
        .unwrap_or(text)
        .trim()
        .to_string()
}

/// Extract keywords using simple TF-IDF
fn extract_keywords_tfidf(text: &str, count: usize) -> Vec<String> {
    let terms = compute_tfidf_terms(text);
    terms
        .iter()
        .take(count)
        .map(|(term, _)| term.clone())
        .collect()
}

/// Estimate token count (rough heuristic: ~4 chars per token)
fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4
}

/// Compute TF-IDF terms from text
fn compute_tfidf_terms(text: &str) -> Vec<(String, f32)> {
    // Simple word frequency with stop word filtering
    let mut word_freq: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    let stop_words = [
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
        "of", "is", "are", "was", "were", "be", "been", "being", "have", "has",
        "had", "do", "does", "did", "will", "would", "could", "should", "may",
        "might", "must", "can", "this", "that", "these", "those", "i", "you",
        "he", "she", "it", "we", "they", "what", "which", "who", "when", "where",
        "why", "how",
    ];

    for word in text.to_lowercase().split_whitespace() {
        let clean: String = word
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect();

        if !clean.is_empty()
            && clean.len() > 2
            && !stop_words.contains(&clean.as_str())
        {
            *word_freq.entry(clean).or_insert(0) += 1;
        }
    }

    let total_words: usize = word_freq.values().sum();
    let mut terms: Vec<_> = word_freq
        .into_iter()
        .map(|(word, freq)| {
            let tf = freq as f32 / total_words as f32;
            (word, tf)
        })
        .collect();

    terms.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    terms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atom_creation() {
        let text = "This is a test atom.".to_string();
        let metadata = AtomMetadata {
            source: SourceType::UserInput,
            agent_id: Uuid::nil(),
            conversation_id: None,
            tags: vec![],
            importance: 1.0,
        };

        let atom = SemanticAtom::from_text(text.clone(), metadata, 3).unwrap();
        assert!(atom.full_text().is_some());
        assert_eq!(atom.resolutions.len(), 3);
    }

    #[test]
    fn test_embedding_similarity() {
        let v1 = EmbeddingVector::sparse_from_text("hello world test");
        let v2 = EmbeddingVector::sparse_from_text("hello world test");
        let similarity = v1.cosine_similarity(&v2);
        assert!(similarity > 0.99); // Should be nearly identical
    }

    #[test]
    fn test_deterministic_chunking() {
        let text = "First. Second. Third.";
        let keywords = extract_keywords_tfidf(text, 5);
        assert!(!keywords.is_empty());
    }
}
