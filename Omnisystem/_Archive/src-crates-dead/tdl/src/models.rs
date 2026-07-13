//! Core data models for the Training Data Library.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single training example with content and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    /// Unique identifier for this example
    pub id: Uuid,

    /// The actual training content (text, JSON, code, etc.)
    pub content: String,

    /// Structured metadata (source, author, domain, tags, etc.)
    pub metadata: Metadata,

    /// Quality score from 0.0 to 1.0
    pub quality_score: f32,

    /// When this example was created
    pub created_at: DateTime<Utc>,

    /// When this example was last modified
    pub updated_at: DateTime<Utc>,

    /// The version this example belongs to
    pub version_id: Uuid,

    /// Optional hash for deduplication
    pub content_hash: String,
}

/// Structured metadata for training examples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Source of the training data (web URL, paper, etc.)
    pub source: Option<String>,

    /// Author or contributor name
    pub author: Option<String>,

    /// Domain/category (ml, nlp, code, etc.)
    pub domain: Option<String>,

    /// Free-form tags for filtering
    pub tags: Vec<String>,

    /// Language of the content
    pub language: Option<String>,

    /// Custom JSON field for additional metadata
    pub custom: Option<serde_json::Value>,
}

impl Metadata {
    /// Create a new empty metadata struct.
    pub fn new() -> Self {
        Self {
            source: None,
            author: None,
            domain: None,
            tags: Vec::new(),
            language: None,
            custom: None,
        }
    }

    /// Add a tag to this metadata.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set the source.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the author.
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set the domain.
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Set the language.
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self::new()
    }
}

/// A version of the training dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    /// Unique identifier for this version
    pub id: Uuid,

    /// Human-readable version string (e.g., "1.0.0")
    pub version_string: String,

    /// Number of examples in this version
    pub example_count: usize,

    /// Total content size in bytes
    pub total_size_bytes: i64,

    /// Who created this version
    pub created_by: String,

    /// Description of changes/purpose
    pub description: String,

    /// When this version was created
    pub created_at: DateTime<Utc>,

    /// Tags for version classification
    pub tags: Vec<String>,

    /// Average quality score across examples
    pub avg_quality_score: f32,

    /// Hash of the version data for integrity checking
    pub version_hash: String,
}

/// Summary information about a version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: Uuid,
    pub version_string: String,
    pub example_count: usize,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub avg_quality_score: f32,
}
