mod error;
mod types;
mod processor;

pub use error::{AnalyticsError, AnalyticsResult};
pub use types::{DataRecord, Pipeline, TransformationRule, RuleType, AggregationResult, QueryResult, DataSchema};
pub use processor::AnalyticsProcessor;
