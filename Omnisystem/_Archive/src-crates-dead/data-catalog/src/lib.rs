mod error;
mod types;
mod catalog;

pub use error::{CatalogError, CatalogResult};
pub use types::{Dataset, DatasetMetadata, DatasetTag, DatasetOwnership, SearchResult};
pub use catalog::DataCatalog;
