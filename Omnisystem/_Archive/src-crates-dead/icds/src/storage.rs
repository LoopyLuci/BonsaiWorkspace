//! Atom storage backend
//!
//! Provides persistent, content-addressed storage for semantic atoms.

use crate::atom::{AtomId, SemanticAtom};
use crate::error::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;

/// Trait for atom storage backends
#[async_trait]
pub trait AtomStore: Send + Sync {
    /// Store an atom
    async fn store(&self, atom: &SemanticAtom) -> Result<()>;

    /// Retrieve an atom by ID
    async fn get(&self, id: &AtomId) -> Result<Option<SemanticAtom>>;

    /// Delete an atom
    async fn delete(&self, id: &AtomId) -> Result<()>;

    /// Check if atom exists
    async fn exists(&self, id: &AtomId) -> Result<bool>;

    /// Get total atom count
    async fn count(&self) -> Result<u64>;

    /// Scan all atoms (for maintenance operations)
    async fn scan(&self, callback: Box<dyn Fn(&SemanticAtom) + Send + Sync>) -> Result<()>;
}

/// In-memory atom store (MVP implementation)
///
/// Suitable for development and small deployments. For production,
/// use a persistent backend (LSM tree, RocksDB, etc.).
pub struct MemoryAtomStore {
    atoms: Arc<DashMap<AtomId, SemanticAtom>>,
}

impl MemoryAtomStore {
    /// Create a new in-memory store
    pub fn new() -> Self {
        Self {
            atoms: Arc::new(DashMap::new()),
        }
    }
}

impl Default for MemoryAtomStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AtomStore for MemoryAtomStore {
    async fn store(&self, atom: &SemanticAtom) -> Result<()> {
        self.atoms.insert(atom.id.clone(), atom.clone());
        Ok(())
    }

    async fn get(&self, id: &AtomId) -> Result<Option<SemanticAtom>> {
        Ok(self.atoms.get(id).map(|entry| entry.clone()))
    }

    async fn delete(&self, id: &AtomId) -> Result<()> {
        self.atoms.remove(id);
        Ok(())
    }

    async fn exists(&self, id: &AtomId) -> Result<bool> {
        Ok(self.atoms.contains_key(id))
    }

    async fn count(&self) -> Result<u64> {
        Ok(self.atoms.len() as u64)
    }

    async fn scan(&self, callback: Box<dyn Fn(&SemanticAtom) + Send + Sync>) -> Result<()> {
        for entry in self.atoms.iter() {
            callback(entry.value());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atom::{AtomMetadata, SourceType};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_memory_store_store_and_get() {
        let store = MemoryAtomStore::new();
        let atom = SemanticAtom::from_text(
            "Test atom".to_string(),
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
        store.store(&atom).await.unwrap();

        let retrieved = store.get(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, id);
    }

    #[tokio::test]
    async fn test_memory_store_count() {
        let store = MemoryAtomStore::new();

        for i in 0..3 {
            let atom = SemanticAtom::from_text(
                format!("Atom {}", i),
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

            store.store(&atom).await.unwrap();
        }

        assert_eq!(store.count().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_memory_store_delete() {
        let store = MemoryAtomStore::new();
        let atom = SemanticAtom::from_text(
            "Test atom".to_string(),
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
        store.store(&atom).await.unwrap();
        assert!(store.exists(&id).await.unwrap());

        store.delete(&id).await.unwrap();
        assert!(!store.exists(&id).await.unwrap());
    }
}
