use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct ProtocolCoordinator {
    active_protocols: Arc<DashMap<String, ProtocolSession>>,
}

#[derive(Debug, Clone)]
pub struct ProtocolSession {
    pub protocol: String,
    pub device_count: u32,
    pub active: bool,
}

impl ProtocolCoordinator {
    pub fn new() -> Self {
        Self {
            active_protocols: Arc::new(DashMap::new()),
        }
    }

    pub fn start_protocol(&self, protocol: String, device_count: u32) -> Result<()> {
        let session = ProtocolSession {
            protocol: protocol.clone(),
            device_count,
            active: true,
        };
        self.active_protocols.insert(protocol, session);
        tracing::info!("Protocol session started");
        Ok(())
    }

    pub fn stop_protocol(&self, protocol: &str) -> Result<()> {
        self.active_protocols.remove(protocol);
        Ok(())
    }

    pub fn protocol_count(&self) -> usize {
        self.active_protocols.len()
    }

    pub fn total_devices(&self) -> u32 {
        self.active_protocols
            .iter()
            .map(|entry| entry.value().device_count)
            .sum()
    }
}

impl Default for ProtocolCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator() {
        let coord = ProtocolCoordinator::new();
        assert!(coord.start_protocol("zigbee".to_string(), 10).is_ok());
        assert_eq!(coord.protocol_count(), 1);
    }
}
