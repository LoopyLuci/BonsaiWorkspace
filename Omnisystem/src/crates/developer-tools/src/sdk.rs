use crate::{DevToolError, DevToolResult, Sdk};
use dashmap::DashMap;
use std::sync::Arc;

pub struct SdkGenerator {
    sdks: Arc<DashMap<String, Sdk>>,
}

impl SdkGenerator {
    pub fn new() -> Self {
        Self {
            sdks: Arc::new(DashMap::new()),
        }
    }

    pub async fn generate(&self, sdk: &Sdk) -> DevToolResult<()> {
        self.sdks.insert(sdk.sdk_name.clone(), sdk.clone());
        Ok(())
    }

    pub async fn get_sdk(&self, name: &str) -> DevToolResult<Sdk> {
        self.sdks
            .get(name)
            .map(|entry| entry.clone())
            .ok_or(DevToolError::GenerationFailed)
    }

    pub fn sdk_count(&self) -> usize {
        self.sdks.len()
    }
}

impl Default for SdkGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_sdk() {
        let gen = SdkGenerator::new();
        let sdk = Sdk {
            sdk_name: "python-sdk".to_string(),
            language: "python".to_string(),
            version: "1.0.0".to_string(),
        };

        gen.generate(&sdk).await.unwrap();
        assert_eq!(gen.sdk_count(), 1);
    }

    #[tokio::test]
    async fn test_get_sdk() {
        let gen = SdkGenerator::new();
        let sdk = Sdk {
            sdk_name: "python-sdk".to_string(),
            language: "python".to_string(),
            version: "1.0.0".to_string(),
        };

        gen.generate(&sdk).await.unwrap();
        let retrieved = gen.get_sdk("python-sdk").await.unwrap();
        assert_eq!(retrieved.language, "python");
    }
}
