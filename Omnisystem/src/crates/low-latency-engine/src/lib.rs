pub mod error;
pub mod types;
pub mod manager;
pub mod database;
pub mod api;

pub use error::{Error, Result};
pub use types::*;
pub use manager::Manager;
pub use database::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_create() {
        let manager = Manager::new();
        let req = CreateRequest {
            created_by: "test".to_string(),
        };
        let result = manager.create(req);
        assert!(result.is_ok());
    }

    #[test]
    fn test_manager_get() {
        let manager = Manager::new();
        let req = CreateRequest {
            created_by: "test".to_string(),
        };
        let record = manager.create(req).unwrap();
        let result = manager.get(record.id);
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_manager_get_not_found() {
        let manager = Manager::new();
        let id = uuid::Uuid::new_v4();
        let result = manager.get(id);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_manager_update() {
        let manager = Manager::new();
        let req = CreateRequest {
            created_by: "test".to_string(),
        };
        let record = manager.create(req).unwrap();
        let update_req = UpdateRequest {
            updated_by: "updated".to_string(),
        };
        let result = manager.update(record.id, update_req);
        assert!(result.is_ok());
    }

    #[test]
    fn test_manager_delete() {
        let manager = Manager::new();
        let req = CreateRequest {
            created_by: "test".to_string(),
        };
        let record = manager.create(req).unwrap();
        let result = manager.delete(record.id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_manager_list() {
        let manager = Manager::new();
        for i in 0..5 {
            let req = CreateRequest {
                created_by: format!("user{}", i),
            };
            manager.create(req).unwrap();
        }
        let items = manager.list();
        assert_eq!(items.len(), 5);
    }

    #[test]
    fn test_manager_count() {
        let manager = Manager::new();
        for i in 0..3 {
            let req = CreateRequest {
                created_by: format!("user{}", i),
            };
            manager.create(req).unwrap();
        }
        assert_eq!(manager.count(), 3);
    }
}
