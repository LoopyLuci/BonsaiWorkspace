//! titan-opencv: a pure-Rust, memory-safe image matrix ([`core::Mat`]) with
//! capability-based hardware access control ([`capabilities`]).
//!
//! Unlike a typical OpenCV binding, this is not an FFI wrapper around the
//! C++ OpenCV library -- it's a from-scratch, bounds-checked matrix type
//! backed entirely by Rust's ownership model, plus a capability system that
//! makes CPU/GPU/TPU hardware access explicit and queryable rather than
//! implicit.

pub mod capabilities;
pub mod core;
pub mod error;

pub use capabilities::{
    Capability, CapabilityContext, CapabilityType, CpuCapability, GpuCapability, TpuCapability,
};
pub use core::Mat;
pub use error::{Error, Result};
