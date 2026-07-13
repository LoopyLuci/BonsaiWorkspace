//! FreeLLMAPI Orchestrator - a small self-hosted cluster manager for FreeLLMAPI
//! gateway nodes: node registry with health tracking and pluggable load-balancing
//! strategies (round robin, least connections, weighted random).

pub mod cluster;
pub mod load_balancer;
pub mod node;

pub use cluster::ClusterManager;
pub use load_balancer::{BalancingStrategy, LoadBalancer};
pub use node::{Node, NodeStatus};
