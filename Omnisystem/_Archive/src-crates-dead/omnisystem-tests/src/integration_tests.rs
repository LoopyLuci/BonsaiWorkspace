use omnisystem_integration::*;

pub async fn test_full_system() -> Result<()> {
    let config = OmnisystemConfig::default();
    let orch = ModuleOrchestrator::new();
    let registry = ServiceRegistry::new();
    
    orch.register_module("test-module".to_string()).await?;
    registry.register("test-service".to_string(), "1.0.0".to_string())?;
    orch.start_module("test-module").await?;
    
    assert_eq!(orch.module_count(), 1);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_integration() {
        assert!(test_full_system().await.is_ok());
    }
}
