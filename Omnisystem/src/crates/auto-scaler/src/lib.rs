//! auto-scaler: policy-driven autoscaling decisions.
//!
//! Services register a [`ScalingPolicy`] describing replica bounds and
//! target utilization; the [`scaler::AutoScaler`] consumes recorded
//! [`MetricSnapshot`]s to recommend scale-up/scale-down decisions and to
//! forecast near-term replica demand from request-count history.

mod error;
mod scaler;
mod types;

pub use error::{ScalingError, ScalingResult};
pub use scaler::AutoScaler;
pub use types::{DemandPrediction, MetricSnapshot, ScalingAction, ScalingDecision, ScalingPolicy};
