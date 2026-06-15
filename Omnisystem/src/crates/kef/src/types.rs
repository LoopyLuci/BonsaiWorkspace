//! Core data types for the Knowledge Extraction Fabric

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Supported extraction methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExtractionMethod {
    /// Generate synthetic explanations from model outputs
    Synthetic,
    /// Extract and cluster activation vectors
    Activation,
    /// Extract attention patterns and triplets
    Attention,
    /// Membership inference for high-confidence data
    MembershipInference,
}

impl fmt::Display for ExtractionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtractionMethod::Synthetic => write!(f, "synthetic"),
            ExtractionMethod::Activation => write!(f, "activation"),
            ExtractionMethod::Attention => write!(f, "attention"),
            ExtractionMethod::MembershipInference => write!(f, "membership_inference"),
        }
    }
}

impl std::str::FromStr for ExtractionMethod {
    type Err = crate::KefError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "synthetic" => Ok(ExtractionMethod::Synthetic),
            "activation" => Ok(ExtractionMethod::Activation),
            "attention" => Ok(ExtractionMethod::Attention),
            "membership_inference" | "membership-inference" => {
                Ok(ExtractionMethod::MembershipInference)
            }
            other => Err(crate::KefError::Other(format!(
                "unknown extraction method: {}",
                other
            ))),
        }
    }
}

/// Quality scores for extracted knowledge
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct QualityScores {
    /// Relevance to the extraction target (0.0-1.0)
    pub relevance: f32,
    /// Estimated factual accuracy (0.0-1.0)
    pub accuracy: f32,
    /// Clarity and coherence (0.0-1.0)
    pub clarity: f32,
    /// Uniqueness relative to existing data (0.0-1.0)
    pub uniqueness: f32,
    /// Aggregate quality score (0.0-1.0)
    pub aggregate: f32,
}

impl QualityScores {
    /// Create default quality scores
    pub fn new() -> Self {
        Self {
            relevance: 0.5,
            accuracy: 0.5,
            clarity: 0.5,
            uniqueness: 0.5,
            aggregate: 0.5,
        }
    }

    /// Update aggregate score based on component scores
    pub fn update_aggregate(&mut self) {
        self.aggregate =
            (self.relevance + self.accuracy + self.clarity + self.uniqueness) / 4.0;
    }
}

impl Default for QualityScores {
    fn default() -> Self {
        Self::new()
    }
}

/// A single curated knowledge chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratedChunk {
    /// Content of the chunk
    pub content: String,
    /// Quality scores for this chunk
    pub quality_scores: QualityScores,
    /// Whether PII was redacted from original
    pub pii_redacted: bool,
    /// Source extraction method
    pub extraction_method: String,
    /// Model that produced this chunk
    pub source_model: String,
    /// Extraction timestamp
    pub extracted_at: DateTime<Utc>,
    /// Optional tags for categorization
    pub tags: Vec<String>,
}

impl CuratedChunk {
    /// Create a new curated chunk
    pub fn new(
        content: String,
        source_model: String,
        extraction_method: String,
    ) -> Self {
        Self {
            content,
            quality_scores: QualityScores::default(),
            pii_redacted: false,
            extraction_method,
            source_model,
            extracted_at: Utc::now(),
            tags: Vec::new(),
        }
    }

    /// Add a tag to the chunk
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }
}

/// A packaged Knowledge Database module containing extracted knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KmodPackage {
    /// Module unique identifier
    pub id: Uuid,
    /// Module name
    pub name: String,
    /// Module version
    pub version: String,
    /// Knowledge domain/category
    pub domain: String,
    /// Number of entries in module
    pub entry_count: usize,
    /// Embedding vector dimension
    pub embedding_dim: usize,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Module description
    pub description: String,
    /// Extraction methods used
    pub extraction_methods: Vec<String>,
    /// Average quality score of entries
    pub avg_quality_score: f32,
}

impl KmodPackage {
    /// Create a new KDB module package
    pub fn new(name: String, domain: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            version: "1.0.0".to_string(),
            domain,
            entry_count: 0,
            embedding_dim: 768, // Default
            created_at: Utc::now(),
            description: String::new(),
            extraction_methods: Vec::new(),
            avg_quality_score: 0.0,
        }
    }

    /// Set the description
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    /// Set embedding dimension
    pub fn with_embedding_dim(mut self, dim: usize) -> Self {
        self.embedding_dim = dim;
        self
    }
}

/// Report of a complete extraction operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionReport {
    /// Total chunks extracted
    pub total_extracted: usize,
    /// Chunks after deduplication
    pub deduplicated: usize,
    /// Chunks with PII redacted
    pub pii_redacted: usize,
    /// Chunks meeting quality threshold
    pub quality_passed: usize,
    /// Average quality score
    pub avg_quality: f32,
    /// Methods used in extraction
    pub methods_used: Vec<String>,
    /// Extraction duration in seconds
    pub duration_secs: f64,
    /// Generated KDB modules
    pub modules: Vec<String>,
    /// Errors encountered
    pub errors: Vec<String>,
}

impl ExtractionReport {
    /// Create a new extraction report
    pub fn new() -> Self {
        Self {
            total_extracted: 0,
            deduplicated: 0,
            pii_redacted: 0,
            quality_passed: 0,
            avg_quality: 0.0,
            methods_used: Vec::new(),
            duration_secs: 0.0,
            modules: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Calculate deduplication ratio
    pub fn dedup_ratio(&self) -> f64 {
        if self.total_extracted == 0 {
            0.0
        } else {
            (self.deduplicated as f64) / (self.total_extracted as f64)
        }
    }

    /// Calculate quality pass ratio
    pub fn quality_pass_ratio(&self) -> f64 {
        if self.deduplicated == 0 {
            0.0
        } else {
            (self.quality_passed as f64) / (self.deduplicated as f64)
        }
    }
}

impl Default for ExtractionReport {
    fn default() -> Self {
        Self::new()
    }
}
