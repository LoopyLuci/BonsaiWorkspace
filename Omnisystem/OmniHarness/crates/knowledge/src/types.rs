//! Core knowledge representation types.

use verify::Term;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ── IDs ───────────────────────────────────────────────────────────────────────

pub type EntityId = String;
pub type RelationId = String;
pub type BeliefId = String;

pub fn new_entity_id() -> EntityId {
    Uuid::new_v4().to_string()
}
pub fn new_relation_id() -> RelationId {
    Uuid::new_v4().to_string()
}
pub fn new_belief_id() -> BeliefId {
    Uuid::new_v4().to_string()
}

// ── EntityType ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntityType {
    Person,
    Organization,
    Location,
    Event,
    Concept,
    Artifact,
    Document,
    CodeSymbol,
    TemporalExpression,
    Quantity,
    Custom(String),
}

// ── Entity ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub name: String,
    pub entity_type: EntityType,
    pub properties: HashMap<String, serde_json::Value>,
    /// Embedding vector for semantic similarity (model-specific dimensionality).
    pub embeddings: Option<Vec<f32>>,
    pub created_at: i64,
    pub updated_at: i64,
    /// Bayesian confidence that this entity exists (0..1).
    pub confidence: f32,
}

impl Entity {
    pub fn new(name: impl Into<String>, entity_type: EntityType) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: new_entity_id(),
            name: name.into(),
            entity_type,
            properties: HashMap::new(),
            embeddings: None,
            created_at: now,
            updated_at: now,
            confidence: 1.0,
        }
    }

    pub fn with_property(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.properties.insert(key.into(), value);
        self
    }
}

// ── Predicate ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Predicate {
    IsA,
    HasProperty,
    PartOf,
    Causes,
    Precedes,
    DependsOn,
    Contradicts,
    Supports,
    Implies,
    EquivalentTo,
    Implements,
    Calls,
    Contains,
    Configures,
    Authenticates,
    Authorizes,
    Custom(String),
}

impl std::fmt::Display for Predicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Predicate::IsA => "is_a",
            Predicate::HasProperty => "has_property",
            Predicate::PartOf => "part_of",
            Predicate::Causes => "causes",
            Predicate::Precedes => "precedes",
            Predicate::DependsOn => "depends_on",
            Predicate::Contradicts => "contradicts",
            Predicate::Supports => "supports",
            Predicate::Implies => "implies",
            Predicate::EquivalentTo => "equivalent_to",
            Predicate::Implements => "implements",
            Predicate::Calls => "calls",
            Predicate::Contains => "contains",
            Predicate::Configures => "configures",
            Predicate::Authenticates => "authenticates",
            Predicate::Authorizes => "authorizes",
            Predicate::Custom(s) => s.as_str(),
        };
        write!(f, "{}", s)
    }
}

// ── Provenance ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProvenanceSource {
    UserStatement {
        session_id: String,
    },
    ModelInference {
        model_id: String,
        adapter_id: Option<String>,
    },
    ToolExecution {
        tool_name: String,
        result_hash: String,
    },
    DeductiveProof {
        proof_id: String,
        kernel_version: String,
    },
    ExternalDocument {
        document_hash: String,
        source_url: Option<String>,
    },
    Observation {
        timestamp: i64,
        observer: String,
    },
    Derived {
        from_beliefs: Vec<BeliefId>,
        rule: String,
    },
}

// ── RelationTarget ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationTarget {
    Entity(EntityId),
    Literal(serde_json::Value),
    Belief(BeliefId),
}

// ── TemporalBounds ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalBounds {
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
}

// ── Relation ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: RelationId,
    pub subject: EntityId,
    pub predicate: Predicate,
    pub object: RelationTarget,
    pub confidence: f32,
    pub source: ProvenanceSource,
    pub temporal_bounds: Option<TemporalBounds>,
    pub created_at: i64,
}

impl Relation {
    pub fn new(
        subject: impl Into<String>,
        predicate: Predicate,
        object: RelationTarget,
        source: ProvenanceSource,
    ) -> Self {
        Self {
            id: new_relation_id(),
            subject: subject.into(),
            predicate,
            object,
            confidence: 1.0,
            source,
            temporal_bounds: None,
            created_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn with_confidence(mut self, c: f32) -> Self {
        self.confidence = c;
        self
    }
}

// ── Evidence & Belief ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub source: ProvenanceSource,
    /// How strongly does this evidence support the belief (0..1).
    pub strength: f32,
    pub timestamp: i64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Belief {
    pub id: BeliefId,
    pub statement: String,
    /// Optional formal representation verified by the Axiom kernel.
    pub formal_statement: Option<Term>,
    /// P(belief is true) — Bayesian probability.
    pub confidence: f32,
    pub evidence: Vec<Evidence>,
    pub last_updated: i64,
    pub times_challenged: u32,
    pub times_confirmed: u32,
}

impl Belief {
    pub fn new(statement: impl Into<String>, confidence: f32) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: new_belief_id(),
            statement: statement.into(),
            formal_statement: None,
            confidence: confidence.clamp(0.0, 1.0),
            evidence: Vec::new(),
            last_updated: now,
            times_challenged: 0,
            times_confirmed: 0,
        }
    }

    pub fn with_formal(mut self, term: Term) -> Self {
        self.formal_statement = Some(term);
        self
    }
}

// ── Search result ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub score: f32,
    pub kind: SearchResultKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchResultKind {
    Entity(Entity),
    Belief(Belief),
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum KnowledgeError {
    #[error("entity not found: {0}")]
    EntityNotFound(EntityId),
    #[error("relation not found: {0}")]
    RelationNotFound(RelationId),
    #[error("belief not found: {0}")]
    BeliefNotFound(BeliefId),
    #[error("serialisation error: {0}")]
    Serialisation(#[from] serde_json::Error),
    #[error("storage error: {0}")]
    Storage(String),
}
