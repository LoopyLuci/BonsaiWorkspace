//! omnisystem-cluster: distributed cluster management primitives.
//!
//! Provides Raft-like consensus, leader election, cluster membership,
//! state replication, RBAC, TLS/mTLS config, audit logging, backups,
//! compliance tracking, multi-region config, GPU/SIMD workload helpers
//! and a task scheduler.

pub mod audit;
pub mod backup;
pub mod compliance;
pub mod consensus;
pub mod core;
pub mod encryption_at_rest;
pub mod error;
pub mod gpu_acceleration;
pub mod leader_election;
pub mod membership;
pub mod multi_region;
pub mod rbac;
pub mod replication;
pub mod scheduling;
pub mod simd_optimization;
pub mod state_machine;
pub mod tls;
pub mod types;
pub mod voting;

pub use audit::*;
pub use backup::*;
pub use compliance::*;
pub use consensus::*;
pub use core::*;
pub use encryption_at_rest::*;
pub use error::{ClusterError, Result};
pub use gpu_acceleration::*;
pub use leader_election::*;
pub use membership::*;
pub use multi_region::*;
pub use rbac::*;
pub use replication::*;
pub use scheduling::*;
pub use simd_optimization::*;
pub use state_machine::*;
pub use tls::*;
pub use types::*;
pub use voting::*;
