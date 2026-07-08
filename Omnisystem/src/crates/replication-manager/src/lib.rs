mod error;
mod types;
mod replication;

pub use error::{ReplicationError, ReplicationResult};
pub use types::{ReplicationConfig, ReplicationMode, ReplicaStatus, FailoverEvent, ConsistencyCheck, ReplicationMetrics, FailoverPolicy};
pub use replication::ReplicationManager;
