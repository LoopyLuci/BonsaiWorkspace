use crate::{ModuleMetadata, ModuleState, Result};
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait SubModule: Send + Sync {
    fn metadata(&self) -> &ModuleMetadata;
    fn state(&self) -> ModuleState;

    async fn initialize(&mut self) -> Result<()>;
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn unload(&mut self) -> Result<()>;

    async fn hot_reload(&mut self) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, _msg: &str) -> Result<String> {
        Ok(String::new())
    }

    fn get_state(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    fn set_state(&mut self, _state: HashMap<String, String>) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ModuleVersion;

    struct TestModule {
        state: ModuleState,
        metadata: ModuleMetadata,
    }

    #[async_trait]
    impl SubModule for TestModule {
        fn metadata(&self) -> &ModuleMetadata {
            &self.metadata
        }

        fn state(&self) -> ModuleState {
            self.state
        }

        async fn initialize(&mut self) -> Result<()> {
            self.state = ModuleState::Initialized;
            Ok(())
        }

        async fn start(&mut self) -> Result<()> {
            self.state = ModuleState::Running;
            Ok(())
        }

        async fn stop(&mut self) -> Result<()> {
            self.state = ModuleState::Stopped;
            Ok(())
        }

        async fn unload(&mut self) -> Result<()> {
            self.state = ModuleState::Unloaded;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_module_lifecycle() {
        let mut module = TestModule {
            state: ModuleState::Unloaded,
            metadata: ModuleMetadata {
                name: "test".to_string(),
                version: ModuleVersion::new(1, 0, 0),
                author: "test".to_string(),
                description: "test".to_string(),
                dependencies: vec![],
                capabilities: vec![],
            },
        };

        assert_eq!(module.state(), ModuleState::Unloaded);
        module.initialize().await.unwrap();
        assert_eq!(module.state(), ModuleState::Initialized);
    }
}
