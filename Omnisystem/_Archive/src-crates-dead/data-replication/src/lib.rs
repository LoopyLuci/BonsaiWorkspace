mod error;
mod types;
mod manager;

pub use error::{ReplicationError, ReplicationResult};
pub use types::{Replica, ReplicaRole, ReplicationLog, ConflictRecord, ResolutionStrategy, ReplicationLag, SyncState, SyncStatus};
pub use manager::ReplicationManager;
