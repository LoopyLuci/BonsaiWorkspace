use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectorId(Uuid);

impl ConnectorId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_name(name: &str) -> Self {
        let hash = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        let mut hasher = hash;
        name.hash(&mut hasher);
        let bytes = hasher.finish().to_le_bytes();
        let mut uuid_bytes = [0u8; 16];
        uuid_bytes[..8].copy_from_slice(&bytes);
        Self(Uuid::from_bytes(uuid_bytes))
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl Default for ConnectorId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConnectorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferingMode {
    Unbounded,
    Bounded(usize),
    Ring(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityLevel {
    Memory,
    AsyncDurable,
    SyncDurable,
    Replicated(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMode {
    None,
    Snappy,
    Gzip,
    Zstd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderingGuarantee {
    None,
    FIFO,
    CausalOrder,
    TotalOrder,
}

#[derive(Debug, Clone)]
pub struct ConnectorConfig {
    pub buffering: BufferingMode,
    pub timeout_ms: u64,
    pub durability: DurabilityLevel,
    pub compression: CompressionMode,
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self {
            buffering: BufferingMode::Bounded(10000),
            timeout_ms: 5000,
            durability: DurabilityLevel::AsyncDurable,
            compression: CompressionMode::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorType {
    RequestReply,
    PubSub,
    Stream,
    Broadcast,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connector_id_new_unique() {
        let id1 = ConnectorId::new();
        let id2 = ConnectorId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_connector_id_from_name_deterministic() {
        let id1 = ConnectorId::from_name("test");
        let id2 = ConnectorId::from_name("test");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_connector_id_display() {
        let id = ConnectorId::new();
        let s = id.to_string();
        assert!(!s.is_empty());
    }

    #[test]
    fn test_buffering_modes() {
        assert_ne!(BufferingMode::Unbounded, BufferingMode::Bounded(100));
        assert_ne!(BufferingMode::Bounded(100), BufferingMode::Ring(100));
    }

    #[test]
    fn test_durability_levels() {
        assert_eq!(DurabilityLevel::Memory, DurabilityLevel::Memory);
        assert_ne!(DurabilityLevel::Memory, DurabilityLevel::AsyncDurable);
    }

    #[test]
    fn test_compression_modes() {
        assert_ne!(CompressionMode::None, CompressionMode::Snappy);
    }

    #[test]
    fn test_ordering_guarantees() {
        assert_ne!(OrderingGuarantee::FIFO, OrderingGuarantee::TotalOrder);
    }

    #[test]
    fn test_connector_config_default() {
        let config = ConnectorConfig::default();
        assert_eq!(config.timeout_ms, 5000);
        assert!(matches!(config.buffering, BufferingMode::Bounded(10000)));
        assert_eq!(config.durability, DurabilityLevel::AsyncDurable);
    }

    #[test]
    fn test_connector_config_custom() {
        let config = ConnectorConfig {
            buffering: BufferingMode::Ring(5000),
            timeout_ms: 2000,
            durability: DurabilityLevel::SyncDurable,
            compression: CompressionMode::Gzip,
        };
        assert_eq!(config.timeout_ms, 2000);
    }

    #[test]
    fn test_connector_types() {
        assert_ne!(ConnectorType::RequestReply, ConnectorType::PubSub);
        assert_ne!(ConnectorType::Stream, ConnectorType::Broadcast);
    }
}
