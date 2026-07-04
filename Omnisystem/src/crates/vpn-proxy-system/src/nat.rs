//! NAT Traversal (STUN/TURN/ICE)

use std::net::SocketAddr;

pub struct StunServer {
    pub address: SocketAddr,
}

impl StunServer {
    pub fn new(address: SocketAddr) -> Self {
        Self { address }
    }

    pub fn get_mapped_address(&self, local_addr: SocketAddr) -> Result<SocketAddr, String> {
        // Stub: return local address as mapped
        Ok(local_addr)
    }
}

pub struct TurnServer {
    pub address: SocketAddr,
    pub username: String,
    pub password: String,
}

impl TurnServer {
    pub fn new(address: SocketAddr, username: String, password: String) -> Self {
        Self {
            address,
            username,
            password,
        }
    }

    pub fn allocate_relay(&self) -> Result<SocketAddr, String> {
        Ok(self.address)
    }
}

pub struct IceCandidate {
    pub address: SocketAddr,
    pub transport: String,
    pub priority: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stun_server() {
        let addr: SocketAddr = "1.1.1.1:3478".parse().unwrap();
        let stun = StunServer::new(addr);
        let local: SocketAddr = "192.168.1.1:51820".parse().unwrap();
        assert!(stun.get_mapped_address(local).is_ok());
    }
}
