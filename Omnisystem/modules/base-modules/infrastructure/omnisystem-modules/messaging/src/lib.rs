//! Omnisystem Messaging Module v1.0.0
//! Sovereign SMTP/IMAP/P2P email with encryption and spam filtering

pub mod module;

pub use module::{MessagingModule, MessagingConfig};
pub use omnisystem_core::{Error, Result};

pub fn create_module() -> Result<MessagingModule> {
    MessagingModule::new(MessagingConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_module() {
        let module = create_module().unwrap();
        assert_eq!(module.name(), "omnisystem-messaging");
    }
}
