use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Embedding {
    pub embedding_id: Uuid,
    pub document_id: String,
    pub vector: Vec<f32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticIndex {
    pub index_id: Uuid,
    pub index_type: String,
    pub vector_dimension: u32,
    pub document_count: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimilarityResult {
    pub result_id: Uuid,
    pub query_id: String,
    pub matched_document: String,
    pub similarity_score: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankedResult {
    pub rank_id: Uuid,
    pub document_id: String,
    pub relevance_score: f32,
    pub rank_position: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticQuery {
    pub query_id: Uuid,
    pub text: String,
    pub embedding: Vec<f32>,
    pub executed_at: DateTime<Utc>,
}
