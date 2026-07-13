//! HTTP CONNECT & SOCKS5 Proxy Services

use parking_lot::Mutex;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone)]
pub struct ProxyConfig {
    pub bind_address: SocketAddr,
    pub max_connections: u32,
    pub timeout_secs: u64,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8080".parse().unwrap(),
            max_connections: 1000,
            timeout_secs: 300,
        }
    }
}

pub struct ProxyServer {
    config: ProxyConfig,
    active_connections: Arc<AtomicU64>,
    total_bytes_forwarded: Arc<AtomicU64>,
}

impl ProxyServer {
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            active_connections: Arc::new(AtomicU64::new(0)),
            total_bytes_forwarded: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn can_accept(&self) -> bool {
        self.active_connections.load(Ordering::Relaxed) < self.config.max_connections as u64
    }

    pub fn add_connection(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn remove_connection(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_bytes(&self, bytes: u64) {
        self.total_bytes_forwarded.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> ProxyStats {
        ProxyStats {
            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_bytes: self.total_bytes_forwarded.load(Ordering::Relaxed),
            max_connections: self.config.max_connections as u64,
        }
    }
}

pub struct ProxyStats {
    pub active_connections: u64,
    pub total_bytes: u64,
    pub max_connections: u64,
}

pub struct Socks5Handler {
    buffer_size: usize,
}

impl Socks5Handler {
    pub fn new() -> Self {
        Self {
            buffer_size: 65536,
        }
    }

    pub fn parse_request(&self, data: &[u8]) -> Result<Socks5Request, String> {
        if data.len() < 6 {
            return Err("Request too short".to_string());
        }

        let cmd = match data[1] {
            0x01 => Socks5Command::Connect,
            0x02 => Socks5Command::Bind,
            0x03 => Socks5Command::Associate,
            _ => return Err("Unknown command".to_string()),
        };

        let addr_type = match data[3] {
            0x01 => {
                if data.len() < 10 {
                    return Err("IPv4 address too short".to_string());
                }
                let ip = IpAddr::from([data[4], data[5], data[6], data[7]]);
                let port = u16::from_be_bytes([data[8], data[9]]);
                (ip, port)
            }
            0x03 => {
                let len = data[4] as usize;
                if data.len() < 5 + len + 2 {
                    return Err("Domain name too short".to_string());
                }
                // Stub: parse domain name
                let ip: IpAddr = "127.0.0.1".parse().unwrap();
                let port = u16::from_be_bytes([
                    data[5 + len],
                    data[6 + len],
                ]);
                (ip, port)
            }
            0x04 => {
                if data.len() < 22 {
                    return Err("IPv6 address too short".to_string());
                }
                let mut bytes = [0u8; 16];
                bytes.copy_from_slice(&data[4..20]);
                let ip = IpAddr::from(bytes);
                let port = u16::from_be_bytes([data[20], data[21]]);
                (ip, port)
            }
            _ => return Err("Unknown address type".to_string()),
        };

        Ok(Socks5Request {
            command: cmd,
            address: addr_type.0,
            port: addr_type.1,
        })
    }

    pub fn create_response(&self, status: u8) -> Vec<u8> {
        vec![0x05, status, 0x00, 0x01, 127, 0, 0, 1, 0, 0]
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Socks5Command {
    Connect,
    Bind,
    Associate,
}

pub struct Socks5Request {
    pub command: Socks5Command,
    pub address: IpAddr,
    pub port: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_server() {
        let config = ProxyConfig::default();
        let server = ProxyServer::new(config);
        assert!(server.can_accept());
        server.add_connection();
        assert_eq!(server.active_connections.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_socks5_handler() {
        let handler = Socks5Handler::new();
        let response = handler.create_response(0x00);
        assert_eq!(response[0], 0x05);
    }

    #[test]
    fn test_proxy_stats() {
        let config = ProxyConfig::default();
        let server = ProxyServer::new(config);
        server.add_connection();
        server.record_bytes(1024);
        
        let stats = server.get_stats();
        assert_eq!(stats.active_connections, 1);
        assert_eq!(stats.total_bytes, 1024);
    }
}
