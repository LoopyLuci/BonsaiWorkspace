//! omnisystem-printer-core
//!
//! Universal 3D printer abstraction: capabilities, configuration, state machine,
//! type identification, and the async [`UniversalPrinter`] trait implemented by
//! concrete printer drivers.

pub mod capabilities;
pub mod config;
pub mod core;
pub mod error;
pub mod printer_trait;
pub mod printer_types;
pub mod state;
pub mod types;

pub use capabilities::PrinterCapabilities;
pub use config::{MaterialProfile, PIDTuning, PrinterConfig};
pub use core::Core;
pub use error::{Error, PrinterResult, Result};
pub use printer_trait::{DiagnosticsReport, UniversalPrinter};
pub use printer_types::{ManufacturerBrand, PrinterIdentity, PrinterInfo, PrinterType};
pub use state::{PrinterState, PrinterStatus};
pub use types::State;
