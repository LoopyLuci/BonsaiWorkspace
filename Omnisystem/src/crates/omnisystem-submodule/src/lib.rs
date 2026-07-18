//! omnisystem-submodule: a hot-swappable submodule system. Modules are
//! trait objects with a lifecycle (initialize -> start -> stop -> unload),
//! versioned metadata with dependency declarations, and semver-style
//! version compatibility resolution.

mod error;
mod manager;
mod module;
mod types;
mod versioning;

pub use error::{Result, SubModuleError};
pub use manager::SubModuleManager;
pub use module::SubModule;
pub use types::{DependencyMode, HotReloadConfig, ModuleDependency, ModuleMetadata, ModuleState, ModuleVersion};
pub use versioning::VersionResolver;
