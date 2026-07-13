use crate::Result;

pub trait SearchConnector: Send + Sync {
    fn connect(&self) -> Result<()>;
    fn execute_query(&self, query: &str) -> Result<Vec<String>>;
    fn disconnect(&self) -> Result<()>;
}

pub struct LocalConnector;

impl SearchConnector for LocalConnector {
    fn connect(&self) -> Result<()> {
        tracing::info!("Connected to local search");
        Ok(())
    }

    fn execute_query(&self, query: &str) -> Result<Vec<String>> {
        tracing::info!("Executing local query: {}", query);
        Ok(vec![])
    }

    fn disconnect(&self) -> Result<()> {
        tracing::info!("Disconnected from local search");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_connector() {
        let connector = LocalConnector;
        assert!(connector.connect().is_ok());
        assert!(connector.disconnect().is_ok());
    }
}
