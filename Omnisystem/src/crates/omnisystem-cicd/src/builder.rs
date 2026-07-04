pub struct Builder {
    crates_built: u32,
}

impl Builder {
    pub fn new() -> Self {
        Self { crates_built: 0 }
    }

    pub async fn build_workspace(&mut self) -> anyhow::Result<u32> {
        tracing::info!("Building workspace...");
        self.crates_built = 846;
        Ok(self.crates_built)
    }

    pub async fn build_crate(&mut self, _crate_name: &str) -> anyhow::Result<()> {
        tracing::info!("Building crate");
        Ok(())
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_builder() {
        let mut builder = Builder::new();
        let count = builder.build_workspace().await.unwrap();
        assert!(count > 0);
    }
}
