use omnisystem_data::*;

#[test]
fn test_multi_layer_cache() {
    let cache = MultiLayerCache::new();
    cache.set("key1".to_string(), "value1".to_string());
    assert!(cache.get("key1").is_ok());
}

#[test]
fn test_data_storage() {
    let storage = DataStorage::new();
    storage.save("id1".to_string(), vec![1, 2, 3]).unwrap();
    let loaded = storage.load("id1").unwrap();
    assert_eq!(loaded.id, "id1");
}

#[test]
fn test_cache_miss() {
    let cache = MultiLayerCache::new();
    assert!(cache.get("nonexistent").is_err());
}

#[test]
fn test_storage_delete() {
    let storage = DataStorage::new();
    storage.save("id1".to_string(), vec![1, 2]).unwrap();
    assert!(storage.delete("id1").is_ok());
    assert!(storage.load("id1").is_err());
}

#[test]
fn test_storage_not_found() {
    let storage = DataStorage::new();
    assert!(storage.load("nonexistent").is_err());
}
