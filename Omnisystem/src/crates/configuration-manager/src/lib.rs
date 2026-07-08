//! Configuration Manager — an in-memory, concurrent record store.

pub mod error;
pub mod manager;
pub mod types;

pub use error::{Error, Result};
pub use manager::Manager;
pub use types::Record;
