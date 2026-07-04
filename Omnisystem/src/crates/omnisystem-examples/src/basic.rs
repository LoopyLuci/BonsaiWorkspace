use omnisystem_integration::*;

pub async fn hello_world() -> Result<()> {
    let config = OmnisystemConfig::default();
    tracing::info!("Omnisystem {} starting", config.version);
    
    let orch = ModuleOrchestrator::new();
    orch.register_module("main".to_string()).await?;
    orch.start_module("main").await?;
    
    let count = orch.module_count();
    tracing::info!("System ready: {} modules active", count);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_hello_world() {
        assert!(hello_world().await.is_ok());
    }
}
