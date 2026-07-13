use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Shard {
    pub shard_id: u32,
    pub node_id: u32,
    pub key_range_start: String,
    pub key_range_end: String,
    pub document_count: usize,
}

pub struct ShardingStrategy {
    shards: Arc<DashMap<u32, Shard>>,
    num_shards: u32,
}

impl ShardingStrategy {
    pub fn new(num_shards: u32) -> Self {
        Self {
            shards: Arc::new(DashMap::new()),
            num_shards,
        }
    }

    pub fn create_shard(&self, node_id: u32, start: String, end: String) -> u32 {
        let shard_id = self.shards.len() as u32;
        let shard = Shard {
            shard_id,
            node_id,
            key_range_start: start,
            key_range_end: end,
            document_count: 0,
        };
        self.shards.insert(shard_id, shard);
        shard_id
    }

    pub fn get_shard_for_key(&self, key: &str) -> Option<Shard> {
        for entry in self.shards.iter() {
            let shard = entry.value();
            if key >= &shard.key_range_start && key < &shard.key_range_end {
                return Some(shard.clone());
            }
        }
        None
    }

    pub fn update_document_count(&self, shard_id: u32, increment: usize) -> bool {
        if let Some(mut shard) = self.shards.get_mut(&shard_id) {
            shard.document_count += increment;
            true
        } else {
            false
        }
    }

    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    pub fn total_documents(&self) -> usize {
        self.shards.iter().map(|s| s.document_count).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_creation() {
        let ss = ShardingStrategy::new(8);
        let shard_id = ss.create_shard(1, "a".to_string(), "m".to_string());
        assert_eq!(ss.shard_count(), 1);
    }

    #[test]
    fn test_key_routing() {
        let ss = ShardingStrategy::new(8);
        ss.create_shard(1, "a".to_string(), "m".to_string());
        ss.create_shard(2, "m".to_string(), "z".to_string());
        let shard = ss.get_shard_for_key("mx").unwrap();
        assert_eq!(shard.node_id, 2);
    }
}
