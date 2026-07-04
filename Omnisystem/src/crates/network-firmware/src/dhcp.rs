use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct DHCPServer {
    ip_pool: Arc<DashMap<String, bool>>,
    leases: Arc<DashMap<String, String>>,
}

impl DHCPServer {
    pub fn new(start_ip: &str, count: u32) -> Self {
        let pool = Arc::new(DashMap::new());
        let base: Vec<u8> = start_ip.split('.').filter_map(|s| s.parse().ok()).collect();
        
        if base.len() == 4 {
            for i in 0..count {
                let ip = format!("{}.{}.{}.{}", base[0], base[1], base[2], base[3] + (i as u8));
                pool.insert(ip, false);
            }
        }

        Self {
            ip_pool: pool,
            leases: Arc::new(DashMap::new()),
        }
    }

    pub fn request_ip(&self, mac: &str) -> Option<String> {
        for mut entry in self.ip_pool.iter_mut() {
            if !*entry.value() {
                *entry.value_mut() = true;
                let ip = entry.key().clone();
                self.leases.insert(mac.to_string(), ip.clone());
                tracing::info!("IP assigned: {}", ip);
                return Some(ip);
            }
        }
        None
    }

    pub fn release_ip(&self, mac: &str) -> Result<()> {
        if let Some((_, ip)) = self.leases.remove(mac) {
            if let Some(mut entry) = self.ip_pool.get_mut(&ip) {
                *entry.value_mut() = false;
            }
            Ok(())
        } else {
            Err(crate::NetworkError::PacketError("Lease not found".to_string()))
        }
    }

    pub fn active_leases(&self) -> usize {
        self.leases.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dhcp_server() {
        let server = DHCPServer::new("192.168.1.100", 10);
        let ip = server.request_ip("aa:bb:cc:dd:ee:ff");
        assert!(ip.is_some());
        assert_eq!(server.active_leases(), 1);
    }

    #[test]
    fn test_release_ip() {
        let server = DHCPServer::new("192.168.1.100", 10);
        let _ip = server.request_ip("aa:bb:cc:dd:ee:ff");
        assert!(server.release_ip("aa:bb:cc:dd:ee:ff").is_ok());
        assert_eq!(server.active_leases(), 0);
    }
}
