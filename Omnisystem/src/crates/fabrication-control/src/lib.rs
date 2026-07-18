//! fabrication-control: a digital fabrication device control system.
//!
//! [`controller::DeviceController`] manages devices ([`device`]),
//! materials ([`material`]), and jobs together. [`path_gen`] generates
//! real toolpath geometry (line/circle interpolation). [`adapters`]
//! provide per-device-type command translation (CNC/laser/printer/
//! pick-place). [`monitoring`] tracks device telemetry, [`scheduler`]
//! queues jobs, and [`simulation`] provides a hardware simulator for
//! testing without physical devices. [`orchestration`] is a separate,
//! independently-real higher-level job/material/work-cell coordination
//! layer that defines its own `JobState`/`MaterialSpec` (distinct from
//! the ones in [`types`]), so it's reachable via its module path rather
//! than glob-exported at the crate root.

pub mod adapters;
pub mod controller;
pub mod device;
pub mod error;
pub mod material;
pub mod monitoring;
pub mod orchestration;
pub mod path_gen;
pub mod scheduler;
pub mod simulation;
pub mod types;

pub use error::{FabricationError, Result};
pub use types::*;
