//! Storage Tier Abstraction

use dashmap::DashMap;

/// Trait for storage tiers (memory, disk, remote)
pub trait StorageTier: Send + Sync {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>>;
    fn set(&self, key: Vec<u8>, value: Vec<u8>);
    fn delete(&self, key: &[u8]);
}

/// In-memory storage tier backed by a concurrent hash map
#[derive(Default)]
pub struct MemoryTier {
    data: DashMap<Vec<u8>, Vec<u8>>,
}

impl MemoryTier {
    pub fn new() -> Self {
        Self {
            data: DashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl StorageTier for MemoryTier {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.get(key).map(|v| v.clone())
    }

    fn set(&self, key: Vec<u8>, value: Vec<u8>) {
        self.data.insert(key, value);
    }

    fn delete(&self, key: &[u8]) {
        self.data.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_tier_set_get_delete() {
        let tier = MemoryTier::new();
        tier.set(b"key".to_vec(), b"value".to_vec());
        assert_eq!(tier.get(b"key"), Some(b"value".to_vec()));
        assert_eq!(tier.len(), 1);

        tier.delete(b"key");
        assert_eq!(tier.get(b"key"), None);
        assert!(tier.is_empty());
    }
}
