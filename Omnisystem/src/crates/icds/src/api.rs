//! OpenAI-compatible API and gRPC interfaces
//!
//! Provides REST/HTTP and gRPC endpoints for ICDS operations.

use crate::{InfiniteContextEngine, atom::AtomMetadata, error::Result};
use serde::{Deserialize, Serialize};

/// Request to append context
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppendRequest {
    /// The text to add
    pub text: String,
    /// Source type
    pub source: String,
    /// Associated metadata
    pub metadata: Option<serde_json::Value>,
}

/// Response from append
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppendResponse {
    /// Created atom IDs
    pub atom_ids: Vec<String>,
}

/// Request to query context
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryRequest {
    /// Query text
    pub query: String,
    /// Number of results
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    10
}

/// Response from query
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryResponse {
    /// Retrieved atoms
    pub atoms: Vec<QueryAtom>,
    /// Latency in ms
    pub latency_ms: u64,
}

/// A retrieved atom in the response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryAtom {
    /// Atom ID
    pub id: String,
    /// Atom text
    pub text: String,
    /// Relevance score
    pub score: f32,
}

/// Request to assemble context for an AI model
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssembleRequest {
    /// Query for context selection
    pub query: String,
    /// Maximum tokens in output
    pub max_tokens: usize,
}

/// Response with assembled context
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssembleResponse {
    /// The assembled context
    pub context: String,
    /// Token estimate
    pub tokens: usize,
}

/// API handler (would normally use Axum/Actix in production)
pub struct IcdsApiHandler {
    engine: InfiniteContextEngine,
}

impl IcdsApiHandler {
    /// Create API handler
    pub fn new(engine: InfiniteContextEngine) -> Self {
        Self { engine }
    }

    /// Handle append request
    pub async fn append(&self, req: AppendRequest) -> Result<AppendResponse> {
        let metadata = AtomMetadata {
            source: crate::atom::SourceType::UserInput,
            agent_id: uuid::Uuid::new_v4(),
            conversation_id: None,
            tags: vec![],
            importance: 1.0,
        };

        let atom_ids = self.engine.ingest(&req.text, metadata).await?;

        Ok(AppendResponse {
            atom_ids: atom_ids.iter().map(|id| id.to_string()).collect(),
        })
    }

    /// Handle query request
    pub async fn query(&self, req: QueryRequest) -> Result<QueryResponse> {
        let results = self.engine.query(&req.query, req.limit).await?;

        let atoms = results
            .atoms
            .iter()
            .map(|retrieved| QueryAtom {
                id: retrieved.atom.id.to_string(),
                text: retrieved
                    .atom
                    .full_text()
                    .unwrap_or("")
                    .to_string(),
                score: retrieved.score,
            })
            .collect();

        Ok(QueryResponse {
            atoms,
            latency_ms: results.latency_us / 1000,
        })
    }

    /// Handle context assembly request
    pub async fn assemble(&self, req: AssembleRequest) -> Result<AssembleResponse> {
        let context = self
            .engine
            .assemble_context(&req.query, req.max_tokens)
            .await?;

        let tokens = context.len() / 4; // Rough estimate

        Ok(AssembleResponse { context, tokens })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_handler_creation() {
        let engine = InfiniteContextEngine::new().await.unwrap();
        let _handler = IcdsApiHandler::new(engine);
    }

    #[tokio::test]
    async fn test_append_request() {
        let engine = InfiniteContextEngine::new().await.unwrap();
        let handler = IcdsApiHandler::new(engine);

        let req = AppendRequest {
            text: "Test content".to_string(),
            source: "user_input".to_string(),
            metadata: None,
        };

        let resp = handler.append(req).await.unwrap();
        assert!(!resp.atom_ids.is_empty());
    }

    #[tokio::test]
    async fn test_query_request() {
        let engine = InfiniteContextEngine::new().await.unwrap();
        let handler = IcdsApiHandler::new(engine);

        // First append
        let append_req = AppendRequest {
            text: "Test content for query".to_string(),
            source: "user_input".to_string(),
            metadata: None,
        };
        handler.append(append_req).await.unwrap();

        // Then query
        let query_req = QueryRequest {
            query: "test".to_string(),
            limit: 10,
        };

        let resp = handler.query(query_req).await.unwrap();
        assert!(!resp.atoms.is_empty());
    }
}
