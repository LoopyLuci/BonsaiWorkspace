use omnisystem_catalog::*;
use std::collections::HashMap;

#[test]
fn test_catalog_creation() {
    let catalog = ModuleCatalog::new();
    assert_eq!(catalog.count(), 0);
}

#[test]
fn test_search_engine() {
    let tokens = SearchEngine::tokenize("Hello World Test");
    assert!(tokens.len() >= 2);
}

#[test]
fn test_catalog_operations() {
    let catalog = ModuleCatalog::new();
    assert_eq!(catalog.list_all().len(), 0);
}

#[test]
fn test_catalog_register_search() {
    let catalog = ModuleCatalog::new();
    let entry = CatalogEntry {
        id: "test".to_string(),
        name: "test-module".to_string(),
        version: "1.0.0".to_string(),
        author: "test".to_string(),
        description: "A test module".to_string(),
        tags: vec!["test".to_string()],
        metadata: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    catalog.register(entry).unwrap();
    assert_eq!(catalog.count(), 1);

    let query = SearchQuery {
        keyword: Some("test".to_string()),
        tags: vec![],
        author: None,
        limit: 10,
        offset: 0,
    };
    let results = catalog.search(&query).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_catalog_entry_get() {
    let catalog = ModuleCatalog::new();
    let entry = CatalogEntry {
        id: "entry1".to_string(),
        name: "Module 1".to_string(),
        version: "2.0.0".to_string(),
        author: "test".to_string(),
        description: "Module".to_string(),
        tags: vec![],
        metadata: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    catalog.register(entry).unwrap();
    let retrieved = catalog.get("entry1").unwrap();
    assert_eq!(retrieved.name, "Module 1");
}
