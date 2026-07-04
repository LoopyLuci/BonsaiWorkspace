use crate::{Result, NetworkError};
use dashmap::DashMap;
use std::sync::Arc;

pub struct IPStack {
    arp_table: Arc<DashMap<String, String>>,
    interfaces: Arc<DashMap<String, String>>,
}

impl IPStack {
    pub fn new() -> Self {
        Self {
            arp_table: Arc::new(DashMap::new()),
            interfaces: Arc::new(DashMap::new()),
        }
    }

    pub fn resolve_arp(&self, ip: &str) -> Result<String> {
        self.arp_table
            .get(ip)
            .map(|ref_| ref_.value().clone())
            .ok_or_else(|| NetworkError::PacketError(format!("ARP lookup failed: {}", ip)))
    }

    pub fn add_arp_entry(&self, ip: String, mac: String) -> Result<()> {
        self.arp_table.insert(ip, mac);
        tracing::info!("ARP entry added");
        Ok(())
    }

    pub fn add_interface(&self, name: String, ip: String) -> Result<()> {
        self.interfaces.insert(name, ip);
        Ok(())
    }

    pub fn arp_table_size(&self) -> usize {
        self.arp_table.len()
    }
}

impl Default for IPStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arp_table() {
        let stack = IPStack::new();
        assert!(stack.add_arp_entry("192.168.1.1".to_string(), "aa:bb:cc:dd:ee:ff".to_string()).is_ok());
        assert!(stack.resolve_arp("192.168.1.1").is_ok());
    }
}
