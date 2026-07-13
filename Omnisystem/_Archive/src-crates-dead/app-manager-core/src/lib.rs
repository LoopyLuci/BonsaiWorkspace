//! App Manager Core - Data Models & Registry System
//!
//! Provides the foundation for the Omnisystem app management ecosystem.

pub mod app;
pub mod module;
pub mod permission;
pub mod registry;
pub mod error;
pub mod models;
pub mod dependency;
pub mod discovery;
pub mod search;
pub mod resolver;

pub use app::{AppId, AppManifest, RegisteredApp, PublisherId};
pub use module::{ModuleId, ModuleManifest, RegisteredModule, ModuleType, ModuleStatus};
pub use permission::{Permission, PermissionCategory, RiskLevel};
pub use registry::{AppRegistry, ModuleRegistry, SearchIndex};
pub use error::{AppManagerError, AppManagerResult};
pub use dependency::{Dependency, VersionConstraint, ModuleDependency};
pub use discovery::{AppDiscoveryService, DiscoveryFilter};
pub use search::{SearchEngine, SearchResult};
pub use resolver::{DependencyResolver, ResolutionResult};
pub use models::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        let _ = std::mem::size_of::<AppManifest>();
        let _ = std::mem::size_of::<ModuleManifest>();
        let _ = std::mem::size_of::<Permission>();
    }
}
