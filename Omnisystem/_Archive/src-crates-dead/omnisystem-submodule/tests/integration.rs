use omnisystem_submodule::*;

#[tokio::test]
async fn test_submodule_manager_creation() {
    let manager = SubModuleManager::new();
    assert_eq!(manager.module_count(), 0);
}

#[test]
fn test_version_matching() {
    let req = ModuleVersion::new(1, 2, 0);
    let avail = ModuleVersion::new(1, 3, 0);
    assert!(avail.matches_required(&req));
}

#[test]
fn test_module_state_transitions() {
    assert_eq!(ModuleState::Unloaded, ModuleState::Unloaded);
    assert_ne!(ModuleState::Running, ModuleState::Stopped);
}

#[test]
fn test_version_resolver() {
    let v1 = ModuleVersion::new(1, 0, 0);
    let v2 = ModuleVersion::new(1, 2, 0);
    assert!(VersionResolver::is_compatible(&v1, &v2).is_ok());
}

#[test]
fn test_module_metadata() {
    let metadata = ModuleMetadata {
        name: "test".to_string(),
        version: ModuleVersion::new(1, 0, 0),
        author: "test".to_string(),
        description: "test".to_string(),
        dependencies: vec![],
        capabilities: vec!["feature1".to_string()],
    };
    assert_eq!(metadata.name, "test");
    assert_eq!(metadata.capabilities.len(), 1);
}
