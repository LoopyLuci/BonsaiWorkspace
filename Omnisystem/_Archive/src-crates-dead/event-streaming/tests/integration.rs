use event_streaming::*;

#[test]
fn test_integration_crud() {
    let manager = Manager::new();

    // Create
    let req = CreateRequest {
        created_by: "integration_test".to_string(),
    };
    let record = manager.create(req).expect("Failed to create");
    let id = record.id;

    // Read
    let result = manager.get(id).expect("Failed to get");
    assert!(result.is_some());

    // Update
    let update_req = UpdateRequest {
        updated_by: "updated".to_string(),
    };
    let updated = manager.update(id, update_req).expect("Failed to update");
    assert_eq!(updated.updated_by, "updated");

    // Delete
    manager.delete(id).expect("Failed to delete");
    let result = manager.get(id).expect("Failed to get after delete");
    assert!(result.is_none());
}

#[test]
fn test_integration_list() {
    let manager = Manager::new();

    for i in 0..10 {
        let req = CreateRequest {
            created_by: format!("user{}", i),
        };
        manager.create(req).expect("Failed to create");
    }

    let items = manager.list();
    assert_eq!(items.len(), 10);
}

#[test]
fn test_integration_concurrent() {
    let manager = std::sync::Arc::new(Manager::new());
    let mut handles = vec![];

    for i in 0..5 {
        let manager_clone = manager.clone();
        let handle = std::thread::spawn(move || {
            let req = CreateRequest {
                created_by: format!("thread{}", i),
            };
            manager_clone.create(req).expect("Failed to create in thread");
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    assert_eq!(manager.count(), 5);
}
