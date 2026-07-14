//! omnisystem-ums: the Universal Module System.
//!
//! [`module`] defines the `Module` trait and metadata types,
//! [`registry::ModuleRegistry`] tracks registered modules,
//! [`resolver::ModuleResolver`] computes dependency-ordered load order
//! (with real cycle detection), [`runtime::ModuleRuntime`] manages the
//! load/init/start/stop lifecycle and execution metrics, and [`data`]
//! manages the on-disk data layer (UMD source / generated / user data,
//! kept isolated from each other).

pub mod data;
pub mod module;
pub mod registry;
pub mod resolver;
pub mod runtime;

pub use data::{DataFolder, DataLayerManager};
pub use module::*;
pub use registry::{ModuleRegistry, RegistryEntry};
pub use resolver::ModuleResolver;
pub use runtime::{ModuleExecutor, ModuleRuntime, RuntimeMetrics};
