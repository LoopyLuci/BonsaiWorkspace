mod error;
mod types;
mod engine;

pub use error::{AnalyticsError, AnalyticsResult};
pub use types::{DataPoint, Aggregation, TimeSeriesTrend, StatisticalSummary};
pub use engine::AnalyticsEngine;
