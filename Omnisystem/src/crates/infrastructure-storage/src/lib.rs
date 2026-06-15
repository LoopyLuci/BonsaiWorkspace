pub mod error;
pub mod types;
pub mod traits;
pub mod object_storage;
pub mod block_storage;
pub mod file_storage;

pub use error::{StorageError, StorageResult};
pub use types::*;
pub use traits::*;
pub use object_storage::InMemoryObjectStorage;
pub use block_storage::InMemoryBlockStorage;
pub use file_storage::InMemoryFileStorage;
