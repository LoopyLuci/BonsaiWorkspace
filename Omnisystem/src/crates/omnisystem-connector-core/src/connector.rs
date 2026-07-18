use serde::{Deserialize, Serialize};

pub trait Connectable: Send + Sync + Serialize + for<'de> Deserialize<'de> {
    fn type_id() -> u128;
    fn schema() -> Schema;

    fn validate(&self) -> crate::Result<()> {
        Ok(())
    }

    fn memory_size(&self) -> usize;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub type_id: u128,
    pub name: String,
    pub version: (u32, u32, u32),
    pub estimated_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorStatus {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let schema = Schema {
            type_id: 123,
            name: "test".to_string(),
            version: (1, 0, 0),
            estimated_size: 100,
        };
        assert_eq!(schema.type_id, 123);
        assert_eq!(schema.name, "test");
        assert_eq!(schema.version, (1, 0, 0));
    }

    #[test]
    fn test_connector_status_different() {
        assert_ne!(ConnectorStatus::Connected, ConnectorStatus::Disconnected);
    }

    #[test]
    fn test_connector_status_eq() {
        assert_eq!(ConnectorStatus::Connected, ConnectorStatus::Connected);
    }
}
