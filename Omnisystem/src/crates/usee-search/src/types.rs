use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub text: String,
    pub limit: usize,
    pub offset: usize,
    pub filters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub documents: Vec<Document>,
    pub total: usize,
    pub query_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub document_id: String,
    pub vector: Vec<f32>,
    pub dimensions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_documents: u64,
    pub total_tokens: u64,
    pub index_size_mb: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document {
            id: "doc1".to_string(),
            title: "Test".to_string(),
            content: "Test content".to_string(),
            metadata: HashMap::new(),
            score: 0.95,
        };
        assert_eq!(doc.score, 0.95);
    }

    #[test]
    fn test_query_creation() {
        let query = Query {
            text: "search term".to_string(),
            limit: 10,
            offset: 0,
            filters: HashMap::new(),
        };
        assert_eq!(query.limit, 10);
    }

    #[test]
    fn test_embedding_creation() {
        let emb = Embedding {
            document_id: "doc1".to_string(),
            vector: vec![0.1, 0.2, 0.3],
            dimensions: 3,
        };
        assert_eq!(emb.dimensions, 3);
    }
}
