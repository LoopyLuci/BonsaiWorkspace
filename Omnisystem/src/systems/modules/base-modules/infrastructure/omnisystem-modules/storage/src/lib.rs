//! Omnisystem Storage Module v1.0.0
//! Content-addressed storage with distributed replication and P2P distribution

pub mod module;

pub use module::{StorageModule, StorageConfig};
pub use omnisystem_core::{Error, Result};

pub fn create_module() -> Result<StorageModule> {
    StorageModule::new(StorageConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_module() {
        let module = create_module().unwrap();
        assert_eq!(module.name(), "omnisystem-storage");
    }
}
