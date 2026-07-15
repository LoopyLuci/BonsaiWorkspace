//! crm-platform: a real Customer Data Platform (CDP) — customer identity,
//! event ingestion, segmentation, autonomous agents, personalization and
//! workflow automation.

pub mod agents;
pub mod cdp;
pub mod core;
pub mod error;
pub mod observability;
pub mod personalization;
pub mod types;
pub mod workflows;

pub use agents::{
    Agent, AgentAction, AgentDecision, AgentOrchestrator, ChurnPredictionAgent,
    LeadQualificationAgent, NextBestActionAgent,
};
pub use cdp::{Customer, CustomerId, DataSource, Event, IngestionConfig, IngestionPipeline, IngestionStats, Segment};
pub use core::Core;
pub use error::{Error, Result};
pub use observability::{CrmMetrics, CrmStats};
pub use personalization::{PersonalizationContext, PersonalizationEngine};
pub use types::State;
pub use workflows::{WorkflowDefinition, WorkflowEngine, WorkflowExecution, WorkflowStep};
