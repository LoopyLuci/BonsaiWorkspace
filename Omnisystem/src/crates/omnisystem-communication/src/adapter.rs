use crate::Result;

pub trait ProtocolAdapter: Send + Sync {
    fn encode(&self, data: &[u8]) -> Result<Vec<u8>>;
    fn decode(&self, data: &[u8]) -> Result<Vec<u8>>;
}

pub struct NoOpAdapter;

impl ProtocolAdapter for NoOpAdapter {
    fn encode(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn decode(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_noop_adapter() {
        let adapter = NoOpAdapter;
        let data = vec![1, 2, 3];
        assert_eq!(adapter.encode(&data).unwrap(), data);
    }
}
