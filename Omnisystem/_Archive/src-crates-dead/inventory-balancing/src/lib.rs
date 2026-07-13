mod error;
mod manager;
mod types;

pub use error::{Error, Result};
pub use manager::Manager;
pub use types::Record;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create() {
        let mgr = Manager::new();
        let rec = mgr.create().await.unwrap();
        assert!(!rec.id.to_string().is_empty());
        assert_eq!(mgr.count(), 1);
    }

    #[tokio::test]
    async fn test_get() {
        let mgr = Manager::new();
        let rec = mgr.create().await.unwrap();
        let fetched = mgr.get(rec.id).await.unwrap();
        assert_eq!(rec.id, fetched.id);
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let mgr = Manager::new();
        let result = mgr.get(uuid::Uuid::new_v4()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multi() {
        let mgr = Manager::new();
        for _ in 0..10 {
            mgr.create().await.unwrap();
        }
        assert_eq!(mgr.count(), 10);
    }
}
