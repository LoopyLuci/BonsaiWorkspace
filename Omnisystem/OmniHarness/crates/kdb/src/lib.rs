pub mod manager;
pub mod module;
pub mod retriever;
pub mod store;

pub use manager::KdbManager;
pub use module::{LoadedModule, ModuleInfo, ModuleManifest};
pub use retriever::KdbRetriever;
pub use store::KdbStore;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum KdbError {
    #[error("module not found: {0}")]
    NotFound(String),
    #[error("dimension mismatch: expected {expected}, got {got}")]
    DimMismatch { expected: usize, got: usize },
    #[error("hnsw error: {0}")]
    Hnsw(#[from] hnsw::HnswError),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid module: {0}")]
    Invalid(String),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
}

pub type Result<T> = std::result::Result<T, KdbError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retriever_creation() {
        let retriever = KdbRetriever::new(768, 10);
        assert!(retriever.is_empty());
    }

    #[test]
    fn test_module_list() {
        let retriever = KdbRetriever::new(768, 10);
        let modules = retriever.list_modules();
        assert_eq!(modules.len(), 0);
    }
}
