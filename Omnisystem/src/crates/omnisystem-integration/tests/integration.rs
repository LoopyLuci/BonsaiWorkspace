use omnisystem_integration::*;

#[tokio::test]
async fn test_full_orchestration() {
    let orch = ModuleOrchestrator::new();
    orch.register_module("module1".to_string()).await.unwrap();
    orch.start_module("module1").await.unwrap();
    
    let module = orch.get_module("module1");
    assert!(module.is_some());
    assert!(module.unwrap().initialized);
}

#[test]
fn test_service_registry() {
    let registry = ServiceRegistry::new();
    registry.register("service1".to_string(), "1.0.0".to_string()).unwrap();
    assert_eq!(registry.list_services().len(), 1);
}

#[test]
fn test_event_bus() {
    let bus = EventBus::new();
    assert_eq!(bus.event_count(), 0);
}

#[test]
fn test_configuration() {
    let config = OmnisystemConfig::default();
    assert_eq!(config.max_modules, 1000);
}

#[test]
fn test_health_check_full() {
    let health = HealthCheck::check(10, 10);
    assert_eq!(health.status, "healthy");
}
