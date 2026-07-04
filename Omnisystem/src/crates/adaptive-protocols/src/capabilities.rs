use crate::{ProtocolCapability, ProtocolError, ProtocolResult, ProtocolType};
use dashmap::DashMap;
use std::sync::Arc;

pub struct CapabilityManager {
    capabilities: Arc<DashMap<ProtocolType, ProtocolCapability>>,
}

impl CapabilityManager {
    pub fn new() -> Self {
        Self {
            capabilities: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_capability(&self, capability: &ProtocolCapability) -> ProtocolResult<()> {
        self.capabilities.insert(capability.protocol, capability.clone());
        Ok(())
    }

    pub async fn get_capability(&self, protocol: ProtocolType) -> ProtocolResult<ProtocolCapability> {
        self.capabilities
            .get(&protocol)
            .map(|entry| entry.clone())
            .ok_or(ProtocolError::CapabilityMissing)
    }

    pub async fn supports_streaming(&self, protocol: ProtocolType) -> ProtocolResult<bool> {
        self.capabilities
            .get(&protocol)
            .map(|entry| entry.supports_streaming)
            .ok_or(ProtocolError::CapabilityMissing)
    }

    pub async fn supports_multiplexing(&self, protocol: ProtocolType) -> ProtocolResult<bool> {
        self.capabilities
            .get(&protocol)
            .map(|entry| entry.supports_multiplexing)
            .ok_or(ProtocolError::CapabilityMissing)
    }

    pub async fn supports_server_push(&self, protocol: ProtocolType) -> ProtocolResult<bool> {
        self.capabilities
            .get(&protocol)
            .map(|entry| entry.supports_server_push)
            .ok_or(ProtocolError::CapabilityMissing)
    }

    pub async fn get_max_connections(&self, protocol: ProtocolType) -> ProtocolResult<u32> {
        self.capabilities
            .get(&protocol)
            .map(|entry| entry.max_connections)
            .ok_or(ProtocolError::CapabilityMissing)
    }

    pub async fn get_compatible_protocols(&self, required_streaming: bool) -> ProtocolResult<Vec<ProtocolType>> {
        let compatible: Vec<ProtocolType> = self
            .capabilities
            .iter()
            .filter(|entry| {
                if required_streaming {
                    entry.value().supports_streaming
                } else {
                    true
                }
            })
            .map(|entry| entry.key().clone())
            .collect();

        if compatible.is_empty() {
            Err(ProtocolError::CapabilityMissing)
        } else {
            Ok(compatible)
        }
    }

    pub fn capability_count(&self) -> usize {
        self.capabilities.len()
    }
}

impl Default for CapabilityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_capability() {
        let manager = CapabilityManager::new();
        let capability = ProtocolCapability {
            protocol: ProtocolType::Http2,
            supports_streaming: true,
            supports_multiplexing: true,
            supports_server_push: true,
            max_connections: 100,
            compression_supported: true,
            tls_version: "1.3".to_string(),
        };

        manager.register_capability(&capability).await.unwrap();
        assert_eq!(manager.capability_count(), 1);
    }

    #[tokio::test]
    async fn test_get_capability() {
        let manager = CapabilityManager::new();
        let capability = ProtocolCapability {
            protocol: ProtocolType::Http2,
            supports_streaming: true,
            supports_multiplexing: true,
            supports_server_push: true,
            max_connections: 100,
            compression_supported: true,
            tls_version: "1.3".to_string(),
        };

        manager.register_capability(&capability).await.unwrap();
        let retrieved = manager.get_capability(ProtocolType::Http2).await.unwrap();

        assert_eq!(retrieved.protocol, ProtocolType::Http2);
    }

    #[tokio::test]
    async fn test_supports_streaming() {
        let manager = CapabilityManager::new();
        let capability = ProtocolCapability {
            protocol: ProtocolType::Http2,
            supports_streaming: true,
            supports_multiplexing: true,
            supports_server_push: true,
            max_connections: 100,
            compression_supported: true,
            tls_version: "1.3".to_string(),
        };

        manager.register_capability(&capability).await.unwrap();
        let supports = manager.supports_streaming(ProtocolType::Http2).await.unwrap();

        assert!(supports);
    }

    #[tokio::test]
    async fn test_get_compatible_protocols() {
        let manager = CapabilityManager::new();

        manager
            .register_capability(&ProtocolCapability {
                protocol: ProtocolType::Http1,
                supports_streaming: false,
                supports_multiplexing: false,
                supports_server_push: false,
                max_connections: 1,
                compression_supported: false,
                tls_version: "1.2".to_string(),
            })
            .await
            .unwrap();

        manager
            .register_capability(&ProtocolCapability {
                protocol: ProtocolType::Http2,
                supports_streaming: true,
                supports_multiplexing: true,
                supports_server_push: true,
                max_connections: 100,
                compression_supported: true,
                tls_version: "1.3".to_string(),
            })
            .await
            .unwrap();

        let compatible = manager.get_compatible_protocols(true).await.unwrap();
        assert_eq!(compatible.len(), 1);
        assert_eq!(compatible[0], ProtocolType::Http2);
    }

    #[tokio::test]
    async fn test_capability_not_found() {
        let manager = CapabilityManager::new();
        let result = manager.get_capability(ProtocolType::Http3).await;

        assert!(result.is_err());
    }
}
