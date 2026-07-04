//! Magic DNS Integration
//!
//! Automatic DNS resolution for mesh network nodes.
//! Provides split-DNS with fallthrough to upstream resolvers.

use crate::coordination::NetworkState;
use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::Arc;

/// DNS Record types
#[derive(Clone, Debug)]
pub enum DNSRecordType {
    A,      // IPv4
    AAAA,   // IPv6
    CNAME,  // Alias
    MX,     // Mail exchange
    SRV,    // Service
}

/// DNS Record
#[derive(Clone, Debug)]
pub struct DNSRecord {
    pub name: String,
    pub record_type: DNSRecordType,
    pub value: String,
    pub ttl: u32,
}

/// Magic DNS resolver
pub struct MagicDNS {
    state: Arc<NetworkState>,
    records: Arc<DashMap<String, Vec<DNSRecord>>>,
    upstream_resolvers: Arc<dashmap::DashSet<String>>,
}

impl MagicDNS {
    pub fn new(state: Arc<NetworkState>) -> Self {
        Self {
            state,
            records: Arc::new(DashMap::new()),
            upstream_resolvers: Arc::new(dashmap::DashSet::new()),
        }
    }

    /// Add upstream resolver for fallthrough
    pub fn add_upstream_resolver(&self, resolver: String) {
        self.upstream_resolvers.insert(resolver);
    }

    /// Resolve domain name to mesh node IP
    pub fn resolve(&self, domain: &str) -> Option<IpAddr> {
        // Try local mesh names first
        if let Some(node_id) = self.state.dns_names.get(domain) {
            if let Some(node) = self.state.get_node(node_id.value()) {
                if node.online {
                    return node.ipv4.or(node.ipv6);
                }
            }
        }

        // Try DNS records
        if let Some(records) = self.records.get(domain) {
            for record in records.iter() {
                match record.record_type {
                    DNSRecordType::A => {
                        if let Ok(ip) = record.value.parse::<IpAddr>() {
                            if ip.is_ipv4() {
                                return Some(ip);
                            }
                        }
                    }
                    DNSRecordType::AAAA => {
                        if let Ok(ip) = record.value.parse::<IpAddr>() {
                            if ip.is_ipv6() {
                                return Some(ip);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        None
    }

    /// Reverse DNS lookup (IP to domain)
    pub fn reverse_lookup(&self, ip: IpAddr) -> Option<String> {
        for entry in self.state.nodes.iter() {
            let node = entry.value();
            if node.ipv4 == Some(ip) || node.ipv6 == Some(ip) {
                return Some(node.hostname.clone());
            }
        }
        None
    }

    /// Add custom DNS record
    pub fn add_record(&self, record: DNSRecord) {
        self.records
            .entry(record.name.clone())
            .or_insert_with(Vec::new)
            .push(record);
    }

    /// Remove DNS record
    pub fn remove_record(&self, domain: &str, value: &str) {
        if let Some(mut records) = self.records.get_mut(domain) {
            records.retain(|r| r.value != value);
        }
    }

    /// Resolve with fallthrough
    pub fn resolve_with_fallthrough(&self, domain: &str) -> Option<IpAddr> {
        // First try mesh
        if let Some(ip) = self.resolve(domain) {
            return Some(ip);
        }

        // Then try custom records
        if let Some(records) = self.records.get(domain) {
            for record in records.iter() {
                if let Ok(ip) = record.value.parse::<IpAddr>() {
                    return Some(ip);
                }
            }
        }

        // Upstream would go here (stub)
        None
    }

    /// Get all registered names
    pub fn list_names(&self) -> Vec<String> {
        self.state
            .dns_names
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Auto-populate from mesh nodes
    pub fn sync_from_mesh(&self) {
        for node in self.state.list_nodes() {
            // Register node name
            if !node.name.is_empty() {
                if let Some(ipv4) = node.ipv4 {
                    self.add_record(DNSRecord {
                        name: node.name.clone(),
                        record_type: DNSRecordType::A,
                        value: ipv4.to_string(),
                        ttl: 3600,
                    });
                }
                if let Some(ipv6) = node.ipv6 {
                    self.add_record(DNSRecord {
                        name: node.name.clone(),
                        record_type: DNSRecordType::AAAA,
                        value: ipv6.to_string(),
                        ttl: 3600,
                    });
                }
            }

            // Register hostname
            if !node.hostname.is_empty() && node.hostname != "unknown" {
                if let Some(ipv4) = node.ipv4 {
                    self.add_record(DNSRecord {
                        name: node.hostname.clone(),
                        record_type: DNSRecordType::A,
                        value: ipv4.to_string(),
                        ttl: 3600,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordination::MeshNode;

    #[test]
    fn test_magic_dns_creation() {
        let state = Arc::new(crate::coordination::NetworkState::new(vec![0u8; 32]));
        let dns = MagicDNS::new(state);
        assert_eq!(dns.list_names().len(), 0);
    }

    #[test]
    fn test_add_custom_record() {
        let state = Arc::new(crate::coordination::NetworkState::new(vec![0u8; 32]));
        let dns = MagicDNS::new(state);

        let record = DNSRecord {
            name: "example.local".to_string(),
            record_type: DNSRecordType::A,
            value: "10.0.0.1".to_string(),
            ttl: 3600,
        };
        dns.add_record(record);
        assert_eq!(dns.records.len(), 1);
    }

    #[test]
    fn test_resolve_custom_record() {
        let state = Arc::new(crate::coordination::NetworkState::new(vec![0u8; 32]));
        let dns = MagicDNS::new(state);

        let record = DNSRecord {
            name: "test.local".to_string(),
            record_type: DNSRecordType::A,
            value: "10.0.0.100".to_string(),
            ttl: 3600,
        };
        dns.add_record(record);

        let resolved = dns.resolve("test.local");
        assert!(resolved.is_some());
    }

    #[test]
    fn test_sync_from_mesh() {
        let state = Arc::new(crate::coordination::NetworkState::new(vec![0u8; 32]));
        let mut node = MeshNode::new(vec![1u8; 32], "mydevice".to_string());
        node.ipv4 = "10.0.0.1".parse().ok();

        state.register_node(node).unwrap();

        let dns = MagicDNS::new(state);
        dns.sync_from_mesh();

        let resolved = dns.resolve("mydevice");
        assert!(resolved.is_some());
    }

    #[test]
    fn test_reverse_lookup() {
        let state = Arc::new(crate::coordination::NetworkState::new(vec![0u8; 32]));
        let mut node = MeshNode::new(vec![1u8; 32], "device1".to_string());
        node.ipv4 = "10.0.0.1".parse().ok();
        node.hostname = "laptop.local".to_string();

        state.register_node(node).unwrap();

        let dns = MagicDNS::new(state);
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let hostname = dns.reverse_lookup(ip);
        assert_eq!(hostname, Some("laptop.local".to_string()));
    }
}
