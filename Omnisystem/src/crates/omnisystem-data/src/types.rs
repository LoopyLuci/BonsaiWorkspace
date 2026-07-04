use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub key: String,
    pub value: T,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageValue {
    pub id: String,
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cache_entry() {
        let entry = CacheEntry {
            key: "k1".to_string(),
            value: "v1".to_string(),
            ttl_seconds: 3600,
        };
        assert_eq!(entry.key, "k1");
    }
}
