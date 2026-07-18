//! Model-trainer: an in-memory ML model lifecycle orchestrator -- dataset
//! management with train/test splitting, a model registry, a simplified
//! training loop that tracks per-epoch progress history, and a validator
//! with real accuracy/RMSE-loss math over prediction/actual arrays.
//!
//! Note: [`trainer::Trainer::train`] simulates training progress (a
//! monotonically decreasing synthetic loss curve) rather than performing
//! real gradient descent over the dataset -- there is no from-scratch ML
//! training engine here. Use [`validation::Validator`] for real accuracy/
//! loss computation against actual prediction data.

pub mod dataset;
pub mod error;
pub mod model;
pub mod trainer;
pub mod validation;

pub use dataset::{Dataset, Sample};
pub use error::{Error, Result, TrainerError};
pub use model::{Model, ModelType};
pub use trainer::{Trainer, TrainingRecord};
pub use validation::Validator;
