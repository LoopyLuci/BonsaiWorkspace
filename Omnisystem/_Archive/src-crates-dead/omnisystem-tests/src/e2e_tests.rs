use omnisystem_integration::*;

pub async fn e2e_scenario() -> Result<()> {
    let config = OmnisystemConfig::default();
    let orch = ModuleOrchestrator::new();
    let registry = ServiceRegistry::new();
    let bus = EventBus::new();
    
    for i in 0..5 {
        let name = format!("e2e-module-{}", i);
        orch.register_module(name.clone()).await?;
        registry.register(name, "1.0.0".to_string())?;
    }
    
    for i in 0..5 {
        orch.start_module(&format!("e2e-module-{}", i)).await?;
    }
    
    assert_eq!(orch.module_count(), 5);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_e2e() {
        assert!(e2e_scenario().await.is_ok());
    }
}
