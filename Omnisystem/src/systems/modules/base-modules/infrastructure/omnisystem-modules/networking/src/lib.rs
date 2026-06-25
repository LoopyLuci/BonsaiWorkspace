//! Omnisystem Networking Module v1.0.0
//! P2P networking with multi-path routing and distributed coordination

pub mod module;

pub use module::{NetworkingModule, NetworkingConfig};
pub use omnisystem_core::{Error, Result};

pub fn create_module() -> Result<NetworkingModule> {
    NetworkingModule::new(NetworkingConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_module() {
        let module = create_module().unwrap();
        assert_eq!(module.name(), "omnisystem-networking");
    }
}
