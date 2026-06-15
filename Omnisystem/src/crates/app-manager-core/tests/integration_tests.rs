//! Comprehensive integration and stress tests

use app_manager_core::*;
use std::sync::Arc;
use std::thread;

#[test]
fn test_concurrent_app_registry_stress() {
    let registry = Arc::new(AppRegistry::new());
    let mut handles = vec![];

    for i in 0..100 {
        let reg = Arc::clone(&registry);
        let handle = thread::spawn(move || {
            let publisher = PublisherId::new();
            let mut manifest = AppManifest::new(
                format!("TestApp{}", i),
                semver::Version::new(1, 0, 0),
                publisher,
            );
            manifest.icon_url = "icon.png".to_string();

            let app = RegisteredApp::new(manifest);
            reg.register(app).expect("Failed to register app");
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(registry.count(), 100);
}

#[test]
fn test_concurrent_app_registry_lookups() {
    let registry = Arc::new(AppRegistry::new());

    // Register initial apps
    for i in 0..50 {
        let publisher = PublisherId::new();
        let mut manifest = AppManifest::new(
            format!("App{}", i),
            semver::Version::new(1, 0, 0),
            publisher,
        );
        manifest.icon_url = "icon.png".to_string();
        let app = RegisteredApp::new(manifest);
        registry.register(app).unwrap();
    }

    let mut handles = vec![];

    for i in 0..50 {
        let reg = Arc::clone(&registry);
        let handle = thread::spawn(move || {
            // Concurrent reads
            for _ in 0..100 {
                let _ = reg.get_by_name(&format!("App{}", i));
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(registry.count(), 50);
}

#[test]
fn test_registry_unregister_nonexistent() {
    let registry = AppRegistry::new();
    let app_id = AppId::new();

    let result = registry.unregister(&app_id);
    assert!(result.is_err());
}

#[test]
fn test_app_manifest_extreme_metadata() {
    let publisher = PublisherId::new();
    let mut manifest = AppManifest::new(
        "TestApp".to_string(),
        semver::Version::new(1, 0, 0),
        publisher,
    );
    manifest.icon_url = "icon.png".to_string();

    // Add large amounts of metadata
    for i in 0..1000 {
        manifest.metadata.insert(
            format!("key{}", i),
            serde_json::json!(format!("value{}", i)),
        );
    }

    assert!(manifest.validate().is_ok());
    assert_eq!(manifest.metadata.len(), 1000);

    let json = manifest.to_json().unwrap();
    let deserialized = AppManifest::from_json(&json).unwrap();
    assert_eq!(deserialized.metadata.len(), 1000);
}

#[test]
fn test_module_manifest_with_many_dependencies() {
    let app_id = AppId::new();
    let mut manifest = ModuleManifest::new(
        app_id,
        "ComplexModule".to_string(),
        semver::Version::new(1, 0, 0),
        ModuleType::Service,
    );
    manifest.add_entry_point("main".to_string(), "/module/main.so".to_string());
    manifest.file_hash = "abc123".to_string();

    // Add many dependencies
    for i in 0..100 {
        let dep = ModuleDependency::new(
            format!("dep{}", i),
            VersionConstraint::Caret(semver::Version::new(1, 0, 0)),
        );
        manifest.add_dependency(dep);
    }

    assert!(manifest.validate().is_ok());
    assert_eq!(manifest.dependencies.len(), 100);
}

#[test]
fn test_version_constraint_all_types() {
    let tests = vec![
        ("^1.2.3", "1.2.3", true),
        ("^1.2.3", "1.5.0", true),
        ("^1.2.3", "2.0.0", false),
        ("~1.2.3", "1.2.4", true),
        ("~1.2.3", "1.3.0", false),
        (">=1.2.3", "2.0.0", true),
        ("<=2.0.0", "1.0.0", true),
        (">1.0.0", "1.0.0", false),
        ("<2.0.0", "1.9.9", true),
        ("=1.2.3", "1.2.3", true),
        ("=1.2.3", "1.2.4", false),
    ];

    for (constraint_str, version_str, expected) in tests {
        let constraint = VersionConstraint::parse(constraint_str)
            .unwrap_or_else(|_| panic!("Failed to parse {}", constraint_str));
        let version = semver::Version::parse(version_str)
            .unwrap_or_else(|_| panic!("Failed to parse {}", version_str));
        assert_eq!(
            constraint.satisfies(&version),
            expected,
            "Constraint {} vs {} failed",
            constraint_str,
            version_str
        );
    }
}

#[test]
fn test_search_index_with_overlapping_tags() {
    use std::sync::Arc;
    use dashmap::DashMap;

    let apps_map = Arc::new(DashMap::new());
    let index = SearchIndex::new(apps_map.clone());

    // Create multiple apps with overlapping tags
    for i in 0..10 {
        let app = RegisteredApp::new({
            let publisher = PublisherId::new();
            let mut m = AppManifest::new(
                format!("App{}", i),
                semver::Version::new(1, 0, 0),
                publisher,
            );
            m.icon_url = "icon.png".to_string();
            m.tags = vec!["common".to_string(), format!("tag{}", i % 3)];
            m
        });
        apps_map.insert(app.manifest.id.clone(), app.clone());
        index.index(&app).unwrap();
    }

    let common_results = index.search_by_tag("common");
    assert_eq!(common_results.len(), 10);

    let tag0_results = index.search_by_tag("tag0");
    assert_eq!(tag0_results.len(), 4); // Apps 0, 3, 6, 9
}

#[test]
fn test_permission_all_categories() {
    let categories = vec![
        PermissionCategory::FileSystem,
        PermissionCategory::Network,
        PermissionCategory::Process,
        PermissionCategory::Hardware,
        PermissionCategory::Memory,
        PermissionCategory::GPU,
        PermissionCategory::Audio,
        PermissionCategory::Video,
        PermissionCategory::Camera,
        PermissionCategory::Microphone,
        PermissionCategory::Geolocation,
    ];

    for category in categories {
        let perm = Permission::new(
            "test".to_string(),
            "Test".to_string(),
            category,
            RiskLevel::High,
        );
        assert_eq!(perm.category, category);
    }
}

#[test]
fn test_installation_record_concurrent_updates() {
    let app_id = AppId::new();
    let record = Arc::new(std::sync::Mutex::new(
        InstallationRecord::new(
            app_id,
            semver::Version::new(1, 0, 0),
            std::path::PathBuf::from("/test"),
        ),
    ));

    let mut handles = vec![];

    for _ in 0..10 {
        let rec = Arc::clone(&record);
        let handle = thread::spawn(move || {
            let mut r = rec.lock().unwrap();
            r.mark_in_progress();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(record.lock().unwrap().status, InstallationStatus::InProgress);
}

#[test]
fn test_user_review_all_ratings() {
    let app_id = AppId::new();

    for rating in 1..=5 {
        let review = UserReview::new(
            app_id.clone(),
            "user".to_string(),
            rating,
            "Title".to_string(),
            "Content".to_string(),
        );
        assert!(review.is_some());
    }

    // Test invalid ratings
    assert!(UserReview::new(
        app_id.clone(),
        "user".to_string(),
        0,
        "Title".to_string(),
        "Content".to_string()
    )
    .is_none());

    assert!(UserReview::new(
        app_id,
        "user".to_string(),
        6,
        "Title".to_string(),
        "Content".to_string()
    )
    .is_none());
}

#[test]
fn test_module_registry_lookup_by_app() {
    let registry = ModuleRegistry::new();
    let app_id = AppId::new();

    // Register multiple modules for the same app
    for i in 0..10 {
        let mut manifest = ModuleManifest::new(
            app_id.clone(),
            format!("Module{}", i),
            semver::Version::new(1, 0, 0),
            ModuleType::Library,
        );
        manifest.add_entry_point("main".to_string(), format!("/modules/{}/main.so", i));
        manifest.file_hash = format!("hash{}", i);

        let module = RegisteredModule::new(manifest);
        registry.register(module).unwrap();
    }

    let modules = registry.get_by_app(&app_id);
    assert_eq!(modules.len(), 10);
}

#[test]
fn test_json_roundtrip_all_types() {
    // App manifest
    let publisher = PublisherId::new();
    let mut app_manifest = AppManifest::new(
        "TestApp".to_string(),
        semver::Version::new(1, 2, 3),
        publisher,
    );
    app_manifest.icon_url = "icon.png".to_string();

    let app_json = app_manifest.to_json().unwrap();
    let app_deserialized = AppManifest::from_json(&app_json).unwrap();
    assert_eq!(app_manifest.id, app_deserialized.id);

    // Module manifest
    let app_id = AppId::new();
    let mut module_manifest = ModuleManifest::new(
        app_id,
        "TestModule".to_string(),
        semver::Version::new(2, 1, 0),
        ModuleType::Service,
    );
    module_manifest.add_entry_point("main".to_string(), "/module/main.so".to_string());
    module_manifest.file_hash = "hash123".to_string();

    let mod_json = module_manifest.to_json().unwrap();
    let mod_deserialized = ModuleManifest::from_json(&mod_json).unwrap();
    assert_eq!(module_manifest.id, mod_deserialized.id);
}

#[test]
fn test_concurrent_mixed_operations() {
    let registry = Arc::new(AppRegistry::new());
    let mut handles = vec![];

    for i in 0..50 {
        let reg = Arc::clone(&registry);
        let handle = thread::spawn(move || {
            if i % 3 == 0 {
                // Register
                let publisher = PublisherId::new();
                let mut manifest = AppManifest::new(
                    format!("App{}", i),
                    semver::Version::new(1, 0, 0),
                    publisher,
                );
                manifest.icon_url = "icon.png".to_string();
                let app = RegisteredApp::new(manifest);
                let _ = reg.register(app);
            } else if i % 3 == 1 {
                // Lookup
                let _ = reg.get_by_name(&format!("App{}", i - 1));
            } else {
                // Count
                let _ = reg.count();
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert!(registry.count() > 0);
}
