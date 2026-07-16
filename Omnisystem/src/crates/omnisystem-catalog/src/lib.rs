//! omnisystem-catalog: a searchable module catalog.
//!
//! [`ModuleCatalog`] stores [`CatalogEntry`] records and supports
//! keyword/tag/author-filtered search with real relevance scoring via
//! [`search::SearchEngine`]. [`storage::MemoryCatalogStorage`] is a real
//! (if non-persistent) implementation of the [`storage::CatalogStorage`]
//! trait for callers that want a storage abstraction independent of
//! `ModuleCatalog`'s own internal indexing.

pub mod catalog;
pub mod error;
pub mod search;
pub mod storage;
pub mod types;

pub use catalog::ModuleCatalog;
pub use error::{CatalogError, Result};
pub use search::SearchEngine;
pub use storage::{CatalogStorage, MemoryCatalogStorage};
pub use types::{CatalogEntry, ModuleInfo, SearchQuery, SearchResult};
