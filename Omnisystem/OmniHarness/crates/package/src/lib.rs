pub mod manifest;
pub mod reader;
pub mod writer;

pub use manifest::{AdapterInfo, BaseModelInfo, KnowledgeModuleRef, PackageManifest};
pub use reader::PackageReader;
pub use writer::PackageWriter;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("invalid package: {0}")]
    Invalid(String),
    #[error("entry not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, PackageError>;
