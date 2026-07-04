//! Omnisystem Bonsai Ecosystem Module v1.0.0
//! Desktop launcher, UOSC runtime, and orchestration

pub mod module;

pub use module::{BonsaiEcosystemModule, BonsaiConfig};
pub use omnisystem_core::{Error, Result};

pub fn create_module() -> Result<BonsaiEcosystemModule> {
    BonsaiEcosystemModule::new(BonsaiConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_module() {
        let module = create_module().unwrap();
        assert_eq!(module.name(), "omnisystem-bonsai-ecosystem");
    }
}
