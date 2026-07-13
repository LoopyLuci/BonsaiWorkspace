use crate::Result;
use async_trait::async_trait;

#[async_trait]
pub trait HealthProbe: Send + Sync {
    async fn probe(&self) -> Result<bool>;
}

pub struct SimplProbe;

#[async_trait]
impl HealthProbe for SimplProbe {
    async fn probe(&self) -> Result<bool> {
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_probe() {
        let probe = SimplProbe;
        assert!(probe.probe().await.unwrap());
    }
}
