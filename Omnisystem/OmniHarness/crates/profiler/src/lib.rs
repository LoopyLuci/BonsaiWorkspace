//! Performance profiling: allocation tracking, async operation metrics,
//! hot-path detection, flamegraph capture, benchmarking, and trend analysis.

pub mod allocation;
pub mod async_metrics;
pub mod benchmarks;
pub mod flamegraph;
pub mod hotpath;
pub mod metrics;
pub mod trending;

pub use allocation::AllocationTracker;
pub use async_metrics::AsyncMetrics;
pub use benchmarks::{setup_criterion, BenchmarkResult, BenchmarkRunner};
pub use flamegraph::FlameGraphProfiler;
pub use hotpath::HotPathDetector;
pub use metrics::{AggregateMetrics, LatencyPercentiles, OperationMetrics};
pub use trending::PerformanceTrend;
