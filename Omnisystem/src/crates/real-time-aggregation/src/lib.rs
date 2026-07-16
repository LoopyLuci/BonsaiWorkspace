mod error;
mod types;
mod aggregator;

pub use error::{AggregationError, AggregationResult};
pub use types::{TimeSeriesMetric, AggregatedValue, PercentileValue, Rollup, DownsampledData, MetricBucket};
pub use aggregator::RealTimeAggregator;
