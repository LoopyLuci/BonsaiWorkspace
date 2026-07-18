//! aion-agents: a multi-agent system toolkit.
//!
//! [`agent::Agent`] perceives, decides (via [`decision::DecisionEngine`],
//! dispatched on its configured [`DecisionType`]), and executes actions.
//! Around it: [`coordination`] for agent registration/broadcast,
//! [`consensus`] quorum voting, [`trust`] reputation tracking,
//! [`learning`]/[`learning_policy`]/[`learning_qlearn`] (three
//! independent learning approaches: a simple key/value knowledge base,
//! policy-gradient RL, and tabular Q-learning), [`knowledge_graph`],
//! [`reasoning`] (symbolic/causal/meta reasoning and planning),
//! [`swarm`]/[`swarm_formations`]/[`collective_behavior`]/[`foraging`]
//! for swarm intelligence, and [`behavior`] for pluggable behavior
//! trees.

pub mod agent;
pub mod behavior;
pub mod collective_behavior;
pub mod consensus;
pub mod coordination;
pub mod decision;
pub mod error;
pub mod foraging;
pub mod knowledge_graph;
pub mod learning;
pub mod learning_policy;
pub mod learning_qlearn;
pub mod reasoning;
pub mod swarm;
pub mod swarm_formations;
pub mod trust;
pub mod types;

pub use error::{AgentError, Result};
pub use types::*;
