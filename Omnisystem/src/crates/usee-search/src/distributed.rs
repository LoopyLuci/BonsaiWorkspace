use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct DistributedIndex {
    shards: Arc<DashMap<u32, IndexShard>>,
    replica_count: u32,
}

#[derive(Debug, Clone)]
pub struct IndexShard {
    pub shard_id: u32,
    pub document_count: u64,
    pub term_count: u64,
}

impl DistributedIndex {
    pub fn new(shard_count: u32, replica_count: u32) -> Self {
        let shards = Arc::new(DashMap::new());
        for i in 0..shard_count {
            shards.insert(i, IndexShard {
                shard_id: i,
                document_count: 0,
                term_count: 0,
            });
        }
        Self {
            shards,
            replica_count,
        }
    }

    pub fn add_document(&self, doc_id: &str, shard_id: u32) -> Result<()> {
        if let Some(mut shard) = self.shards.get_mut(&shard_id) {
            shard.document_count += 1;
            tracing::info!("Document added to shard {}", shard_id);
            Ok(())
        } else {
            Err(crate::SearchError::IndexError("Shard not found".to_string()))
        }
    }

    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    pub fn total_documents(&self) -> u64 {
        self.shards.iter().map(|s| s.value().document_count).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distributed_index() {
        let index = DistributedIndex::new(4, 2);
        assert_eq!(index.shard_count(), 4);
        assert!(index.add_document("doc1", 0).is_ok());
    }
}
