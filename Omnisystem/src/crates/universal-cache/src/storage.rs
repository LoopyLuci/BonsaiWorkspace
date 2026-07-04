//! Storage Tier Abstraction

/// Trait for storage tiers (memory, disk, remote)
pub trait StorageTier: Send + Sync {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>>;
    fn set(&self, key: Vec<u8>, value: Vec<u8>);
    fn delete(&self, key: &[u8]);
}

pub struct MemoryTier;
pub struct DiskTier;
pub struct RemoteTier;
