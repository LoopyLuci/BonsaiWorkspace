//! Hierarchical vector index for semantic search
//!
//! Uses HNSW (Hierarchical Navigable Small World) for O(log N) retrieval.

use crate::atom::{AtomId, EmbeddingVector, SemanticAtom};
use crate::error::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;

/// Entry in the vector index
#[derive(Clone, Debug)]
struct IndexEntry {
    atom_id: AtomId,
    embedding: EmbeddingVector,
}

/// Hierarchical HNSW-style vector index
///
/// In production, this would use a proper HNSW library (e.g., hnswlib).
/// For MVP, we use a simple approximate index with clustering.
pub struct HierarchicalIndex {
    /// Index entries keyed by atom ID
    entries: Arc<DashMap<AtomId, IndexEntry>>,
    /// Cluster assignments (atom ID -> cluster ID)
    clusters: Arc<DashMap<AtomId, usize>>,
    /// Cluster centroids
    cluster_centroids: Arc<DashMap<usize, Vec<String>>>,
    /// HNSW parameters
    m: usize,
    ef_construction: usize,
}

impl HierarchicalIndex {
    /// Create a new hierarchical index
    pub fn new(m: usize, ef_construction: usize) -> Result<Self> {
        Ok(Self {
            entries: Arc::new(DashMap::new()),
            clusters: Arc::new(DashMap::new()),
            cluster_centroids: Arc::new(DashMap::new()),
            m,
            ef_construction,
        })
    }

    /// Insert an atom into the index
    pub async fn insert(&self, atom: &SemanticAtom) -> Result<()> {
        let entry = IndexEntry {
            atom_id: atom.id.clone(),
            embedding: atom.embedding.clone(),
        };

        self.entries.insert(atom.id.clone(), entry);

        // Simple clustering: assign to nearest cluster or create new one
        let cluster_id = self.find_best_cluster(&atom.embedding).await.unwrap_or(0);
        self.clusters.insert(atom.id.clone(), cluster_id);

        Ok(())
    }

    /// Remove an atom from the index
    pub async fn remove(&self, atom_id: &AtomId) -> Result<()> {
        self.entries.remove(atom_id);
        self.clusters.remove(atom_id);
        Ok(())
    }

    /// Search for nearest neighbors
    pub async fn search(&self, query: &EmbeddingVector, k: usize) -> Result<Vec<(AtomId, f32)>> {
        // Simple brute-force search for MVP
        // In production: use HNSW traversal with cluster pruning
        let mut candidates = Vec::new();

        for entry in self.entries.iter() {
            let similarity = query.cosine_similarity(&entry.embedding);
            candidates.push((entry.atom_id.clone(), similarity));
        }

        // Sort by similarity (descending) and take top k
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(candidates.into_iter().take(k).collect())
    }

    /// Get total entries in index
    pub async fn size(&self) -> Result<u64> {
        Ok(self.entries.len() as u64)
    }

    /// Find the best cluster for an embedding (simple centroid matching)
    async fn find_best_cluster(&self, embedding: &EmbeddingVector) -> Option<usize> {
        // Simplified: return 0 for MVP
        // In production: compute distance to cluster centroids
        if self.clusters.is_empty() {
            None
        } else {
            Some(0)
        }
    }
}

/// Query result with similarity score
#[derive(Clone, Debug)]
pub struct SearchResult {
    /// The matching atom
    pub atom: SemanticAtom,
    /// Similarity score (0.0-1.0)
    pub score: f32,
    /// Resolution level returned
    pub resolution_level: crate::atom::ResolutionLevel,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atom::{AtomMetadata, SourceType};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_index_creation() {
        let index = HierarchicalIndex::new(16, 200).unwrap();
        let size = index.size().await.unwrap();
        assert_eq!(size, 0);
    }

    #[tokio::test]
    async fn test_index_insert_and_search() {
        let index = HierarchicalIndex::new(16, 200).unwrap();

        let atom = SemanticAtom::from_text(
            "hello world test".to_string(),
            AtomMetadata {
                source: SourceType::UserInput,
                agent_id: Uuid::nil(),
                conversation_id: None,
                tags: vec![],
                importance: 1.0,
            },
            3,
        )
        .unwrap();

        index.insert(&atom).await.unwrap();

        let results = index
            .search(&atom.embedding, 1)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, atom.id);
        assert!(results[0].1 > 0.99); // Should match itself
    }

    #[tokio::test]
    async fn test_index_remove() {
        let index = HierarchicalIndex::new(16, 200).unwrap();

        let atom = SemanticAtom::from_text(
            "hello world".to_string(),
            AtomMetadata {
                source: SourceType::UserInput,
                agent_id: Uuid::nil(),
                conversation_id: None,
                tags: vec![],
                importance: 1.0,
            },
            3,
        )
        .unwrap();

        let id = atom.id.clone();
        index.insert(&atom).await.unwrap();
        assert_eq!(index.size().await.unwrap(), 1);

        index.remove(&id).await.unwrap();
        assert_eq!(index.size().await.unwrap(), 0);
    }
}
