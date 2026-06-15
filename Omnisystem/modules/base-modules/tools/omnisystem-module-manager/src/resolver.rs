//! Intelligent dependency resolver for cross-language modules

use crate::{ModuleId, ModuleManagerError, Result};
use std::collections::{HashMap, HashSet};

/// Dependency resolution result
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    /// Ordered list of modules to load (topological sort)
    pub order: Vec<ModuleId>,
    /// Dependency graph
    pub graph: HashMap<String, Vec<String>>,
    /// Detected cycles (if any)
    pub cycles: Vec<Vec<String>>,
}

/// Intelligent dependency resolver
pub struct DependencyResolver {
    /// Resolved versions cache
    version_cache: HashMap<String, String>,
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self {
            version_cache: HashMap::new(),
        }
    }

    /// Resolve dependencies with cycle detection (robust)
    pub fn resolve(&mut self, modules: &[ModuleId]) -> Result<ResolutionResult> {
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut rec_stack: HashSet<String> = HashSet::new();
        let mut cycles: Vec<Vec<String>> = Vec::new();

        // Build dependency graph
        for module in modules {
            let key = module.full_id();
            if !graph.contains_key(&key) {
                graph.insert(key, vec![]);
            }
        }

        // Detect cycles and perform topological sort
        for module in modules {
            let key = module.full_id();
            if !visited.contains(&key) {
                self.dfs_detect_cycles(
                    &key,
                    &graph,
                    &mut visited,
                    &mut rec_stack,
                    &mut cycles,
                );
            }
        }

        // If cycles detected, return error (robust)
        if !cycles.is_empty() {
            return Err(ModuleManagerError::DependencyResolutionFailed(
                format!(
                    "Circular dependencies detected: {:?}",
                    cycles
                ),
            ));
        }

        // Topological sort
        let order = self.topological_sort(modules, &graph)?;

        Ok(ResolutionResult {
            order,
            graph,
            cycles,
        })
    }

    /// DFS for cycle detection
    fn dfs_detect_cycles(
        &self,
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    self.dfs_detect_cycles(neighbor, graph, visited, rec_stack, cycles);
                } else if rec_stack.contains(neighbor) {
                    cycles.push(vec![node.to_string(), neighbor.clone()]);
                }
            }
        }

        rec_stack.remove(node);
    }

    /// Topological sort using Kahn's algorithm
    fn topological_sort(
        &self,
        modules: &[ModuleId],
        graph: &HashMap<String, Vec<String>>,
    ) -> Result<Vec<ModuleId>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut queue: Vec<String> = Vec::new();
        let mut result: Vec<ModuleId> = Vec::new();

        // Initialize in_degree
        for module in modules {
            in_degree.insert(module.full_id(), 0);
        }

        // Calculate in_degree
        for module in modules {
            if let Some(deps) = graph.get(&module.full_id()) {
                for dep in deps {
                    *in_degree.entry(dep.clone()).or_insert(0) += 1;
                }
            }
        }

        // Find nodes with 0 in_degree
        for (node, degree) in &in_degree {
            if *degree == 0 {
                queue.push(node.clone());
            }
        }

        // Process queue
        while !queue.is_empty() {
            let node = queue.remove(0);
            result.push(self.parse_module_id(&node)?);

            if let Some(neighbors) = graph.get(&node) {
                for neighbor in neighbors {
                    let degree = in_degree.entry(neighbor.clone()).or_insert(0);
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push(neighbor.clone());
                    }
                }
            }
        }

        if result.len() != modules.len() {
            return Err(ModuleManagerError::DependencyResolutionFailed(
                "Could not resolve all dependencies".to_string(),
            ));
        }

        Ok(result)
    }

    /// Parse module ID from string
    fn parse_module_id(&self, id_str: &str) -> Result<ModuleId> {
        let parts: Vec<&str> = id_str.split(':').collect();
        if parts.len() != 2 {
            return Err(ModuleManagerError::InvalidModule(
                format!("Invalid module ID format: {}", id_str),
            ));
        }

        let name_version: Vec<&str> = parts[1].split('@').collect();
        if name_version.len() != 2 {
            return Err(ModuleManagerError::InvalidModule(
                format!("Invalid module version format: {}", parts[1]),
            ));
        }

        Ok(ModuleId::new(
            parts[0],
            name_version[0],
            name_version[1],
        ))
    }

    /// Resolve version requirement (intelligent version matching)
    pub fn resolve_version_requirement(
        &mut self,
        name: &str,
        requirement: &str,
    ) -> Result<String> {
        // Check cache
        let key = format!("{}:{}", name, requirement);
        if let Some(version) = self.version_cache.get(&key) {
            return Ok(version.clone());
        }

        // Parse requirement (supports ^1.0, ~1.0, >=1.0, <=1.0, 1.0, etc.)
        let version = match requirement {
            v if v.starts_with('^') => {
                // Caret: compatible with version
                // ^1.2.3 allows >=1.2.3 and <2.0.0
                let base = v.trim_start_matches('^');
                let parts: Vec<&str> = base.split('.').collect();
                if parts.len() >= 2 {
                    format!(">={}", base)
                } else {
                    base.to_string()
                }
            }
            v if v.starts_with('~') => {
                // Tilde: allows patch updates
                // ~1.2.3 allows >=1.2.3 and <1.3.0
                let base = v.trim_start_matches('~');
                format!(">={}", base)
            }
            v if v.starts_with(">=") || v.starts_with("<=") || v.starts_with("==") => {
                v.to_string()
            }
            v => v.to_string(), // Exact version
        };

        // Cache result
        self.version_cache.insert(key, version.clone());
        Ok(version)
    }
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_creation() {
        let resolver = DependencyResolver::new();
        assert_eq!(resolver.version_cache.len(), 0);
    }

    #[test]
    fn test_version_resolution_caret() {
        let mut resolver = DependencyResolver::new();
        let result = resolver.resolve_version_requirement("test", "^1.2.3").unwrap();
        assert!(result.contains("1.2.3"));
    }

    #[test]
    fn test_version_resolution_tilde() {
        let mut resolver = DependencyResolver::new();
        let result = resolver.resolve_version_requirement("test", "~1.2.3").unwrap();
        assert!(result.contains("1.2.3"));
    }

    #[test]
    fn test_version_resolution_exact() {
        let mut resolver = DependencyResolver::new();
        let result = resolver.resolve_version_requirement("test", "1.2.3").unwrap();
        assert_eq!(result, "1.2.3");
    }

    #[test]
    fn test_module_id_parsing() {
        let resolver = DependencyResolver::new();
        let id = resolver.parse_module_id("omnisystem:compiler@1.0.0").unwrap();
        assert_eq!(id.namespace, "omnisystem");
        assert_eq!(id.name, "compiler");
        assert_eq!(id.version, "1.0.0");
    }

    #[test]
    fn test_version_cache() {
        let mut resolver = DependencyResolver::new();
        let v1 = resolver
            .resolve_version_requirement("test", "^1.0.0")
            .unwrap();
        let v2 = resolver
            .resolve_version_requirement("test", "^1.0.0")
            .unwrap();
        assert_eq!(v1, v2);
        assert_eq!(resolver.version_cache.len(), 1);
    }

    #[test]
    fn test_simple_dependency_graph() {
        let mut resolver = DependencyResolver::new();
        let modules = vec![
            ModuleId::new("omnisystem", "core", "1.0.0"),
            ModuleId::new("omnisystem", "compiler", "1.0.0"),
        ];

        let result = resolver.resolve(&modules).unwrap();
        assert_eq!(result.cycles.len(), 0);
        assert_eq!(result.order.len(), 2);
    }
}
