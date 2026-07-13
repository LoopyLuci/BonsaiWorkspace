use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct Loader {
    destinations: Arc<DashMap<String, LoadDestination>>,
}

#[derive(Debug, Clone)]
pub struct LoadDestination {
    pub name: String,
    pub dest_type: DestinationType,
    pub records_loaded: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestinationType {
    Database,
    FileSystem,
    DataWarehouse,
    Cache,
}

impl Loader {
    pub fn new() -> Self {
        Self {
            destinations: Arc::new(DashMap::new()),
        }
    }

    pub fn register_destination(&self, dest: LoadDestination) -> Result<()> {
        self.destinations.insert(dest.name.clone(), dest);
        Ok(())
    }

    pub fn load_data(&self, dest_name: &str, record_count: u64) -> Result<()> {
        if let Some(mut dest) = self.destinations.get_mut(dest_name) {
            dest.records_loaded += record_count;
            tracing::info!("Data loaded: {} records", record_count);
            Ok(())
        } else {
            Err(crate::PipelineError::LoadError("Destination not found".to_string()))
        }
    }

    pub fn destination_count(&self) -> usize {
        self.destinations.len()
    }
}

impl Default for Loader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader() {
        let loader = Loader::new();
        let dest = LoadDestination {
            name: "db".to_string(),
            dest_type: DestinationType::Database,
            records_loaded: 0,
        };
        assert!(loader.register_destination(dest).is_ok());
        assert!(loader.load_data("db", 100).is_ok());
    }
}
