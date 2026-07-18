//! App Manager Repository
//!
//! Orchestrates fetching, caching, and validating Omnisystem app packages
//! from GitHub releases, a marketplace HTTP API, or the local filesystem.

pub mod error;
pub mod github_fetcher;
pub mod local_loader;
pub mod marketplace;
pub mod package_validator;
pub mod repository;
pub mod ull_wrapper;

pub use error::{RepositoryError, Result};
pub use github_fetcher::{GitHubFetcher, GitHubRelease};
pub use local_loader::LocalLoader;
pub use marketplace::{AppListing, Marketplace};
pub use package_validator::PackageValidator;
pub use repository::{Repository, RepositoryConfig};
pub use ull_wrapper::register_with_ull;
