//! Omnifile-explorer: an in-memory virtual file system, a metadata
//! indexer/searcher, and an explorer facade with navigation and
//! bookmarks -- entirely in-process, no real disk access.

pub mod error;
pub mod explorer;
pub mod filesystem;
pub mod indexer;
pub mod search;

pub use error::{Error, FileError, Result};
pub use explorer::Explorer;
pub use filesystem::{FileMetadata, VirtualFileSystem};
pub use indexer::{FileIndexer, IndexEntry};
pub use search::FileSearchEngine;
