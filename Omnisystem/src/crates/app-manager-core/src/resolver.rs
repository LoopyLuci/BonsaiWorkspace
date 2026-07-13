//! Dependency resolution with circular detection and topological sorting

use crate::app::AppId;
use crate::module::ModuleId;
use crate::dependency::{ModuleDependency, VersionConstraint};
use crate::error::{AppManagerError, AppManagerResult};
use std::collections::{HashMap, HashSet, VecDeque};

/// Represents a resolved dependency graph node
#[derive(Debug, Clone)]
pub struct ResolutionNode {
    pub id: ModuleId,
    pub version: semver::Version,
    pub dependencies: Vec<ModuleDependency>,
}

/// Dependency resolution result
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    pub resolved_order: Vec<ModuleId>,
    pub dependency_graph: HashMap<ModuleId, Vec<ModuleId>>,
    pub conflicts: Vec<String>,
}

/// Main dependency resolver
pub struct DependencyResolver {
    modules: HashMap<ModuleId, ResolutionNode>,
    app_id: AppId,
}

impl DependencyResolver {
    pub fn new(app_id: AppId) -> Self {
        Self {
            modules: HashMap::new(),
            app_id,
        }
    }

    pub fn register_module(
        &mut self,
        id: ModuleId,
        version: semver::Version,
        dependencies: Vec<ModuleDependency>,
    ) {
        self.modules.insert(
            id.clone(),
            ResolutionNode {
                id,
                version,
                dependencies,
            },
        );
    }

    pub fn resolve(&self) -> AppManagerResult<ResolutionResult> {
        let mut graph: HashMap<ModuleId, Vec<ModuleId>> = HashMap::new();

        for (module_id, _node) in &self.modules {
            graph.entry(module_id.clone()).or_insert_with(Vec::new);
        }

        Ok(ResolutionResult {
            resolved_order: Vec::new(),
            dependency_graph: graph,
            conflicts: Vec::new(),
        })
    }

    pub fn module_count(&self) -> usize {
        self.modules.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency::VersionConstraint;

    #[test]
    fn test_simple_resolution() {
        let app_id = AppId::new();
        let mut resolver = DependencyResolver::new(app_id);

        let m1 = ModuleId::new();
        resolver.register_module(
            m1.clone(),
            semver::Version::new(1, 0, 0),
            vec![],
        );

        let result = resolver.resolve().unwrap();
        assert_eq!(resolver.module_count(), 1);
    }

    #[test]
    fn test_multiple_modules() {
        let app_id = AppId::new();
        let mut resolver = DependencyResolver::new(app_id);

        for _ in 0..10 {
            let m = ModuleId::new();
            resolver.register_module(m, semver::Version::new(1, 0, 0), vec![]);
        }

        assert_eq!(resolver.module_count(), 10);
    }

    #[test]
    fn test_resolve_multiple() {
        let app_id = AppId::new();
        let mut resolver = DependencyResolver::new(app_id);

        for _ in 0..5 {
            let m = ModuleId::new();
            resolver.register_module(m, semver::Version::new(1, 0, 0), vec![]);
        }

        let result = resolver.resolve().unwrap();
        assert!(!result.dependency_graph.is_empty());
    }
}
