//! Customer Data Platform (CDP) Core

pub mod customer;
pub mod ingestion;

pub use customer::{Customer, CustomerId, Event, Segment};
pub use ingestion::{IngestionPipeline, IngestionConfig, RawEvent, DataSource, IngestionStats};
