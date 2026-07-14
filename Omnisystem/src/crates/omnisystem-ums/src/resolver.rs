// Module Resolver - handles dependency resolution and loading order

use crate::registry::{ModuleRegistry, RegistryEntry};
use anyhow::{anyhow, Result};

/// Module Resolver - determines correct load order and resolves dependencies
pub struct ModuleResolver {
    registry: ModuleRegistry,
}

impl ModuleResolver {
    pub fn new(registry: ModuleRegistry) -> Self {
        Self { registry }
    }

    /// Resolve load order for modules
    /// Returns modules in dependency order (dependencies first)
    pub fn resolve_load_order(&self, module_names: &[&str]) -> Result<Vec<String>> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();

        for name in module_names {
            self._resolve_recursive(name, &mut result, &mut visited, &mut visiting)?;
        }

        Ok(result)
    }

    fn _resolve_recursive(
        &self,
        name: &str,
        result: &mut Vec<String>,
        visited: &mut std::collections::HashSet<String>,
        visiting: &mut std::collections::HashSet<String>,
    ) -> Result<()> {
        if visited.contains(name) {
            return Ok(()); // Already processed
        }

        if visiting.contains(name) {
            return Err(anyhow!("Circular dependency detected involving: {}", name));
        }

        visiting.insert(name.to_string());

        let entry = self
            .registry
            .get_by_name(name)
            .ok_or_else(|| anyhow!("Module not found: {}", name))?;

        // Process dependencies first
        for dep in &entry.info.dependencies {
            self._resolve_recursive(dep, result, visited, visiting)?;
        }

        visiting.remove(name);
        visited.insert(name.to_string());
        result.push(name.to_string());

        Ok(())
    }

    /// Resolve dependencies for a module
    pub fn resolve_dependencies(&self, module_name: &str) -> Result<Vec<RegistryEntry>> {
        let order = self.resolve_load_order(&[module_name])?;

        let mut result = Vec::new();
        for name in order {
            if let Some(entry) = self.registry.get_by_name(&name) {
                result.push(entry);
            }
        }

        Ok(result)
    }

    /// Check if dependencies are satisfied
    pub fn check_dependencies(&self, module_name: &str) -> Result<bool> {
        let entry = self
            .registry
            .get_by_name(module_name)
            .ok_or_else(|| anyhow!("Module not found: {}", module_name))?;

        // Check each dependency exists
        for dep in &entry.info.dependencies {
            if !self.registry.exists(dep) {
                return Err(anyhow!(
                    "Dependency not found: {} requires {}",
                    module_name,
                    dep
                ));
            }
        }

        Ok(true)
    }

    /// Get all modules needed for a module to function
    pub fn get_required_modules(&self, module_name: &str) -> Result<Vec<RegistryEntry>> {
        let order = self.resolve_load_order(&[module_name])?;

        let mut result = Vec::new();
        for name in order {
            if let Some(entry) = self.registry.get_by_name(&name) {
                result.push(entry);
            }
        }

        Ok(result)
    }

    /// Get module load groups (modules that can be loaded concurrently)
    /// Useful for parallel module initialization
    pub fn get_load_groups(&self, module_names: &[&str]) -> Result<Vec<Vec<String>>> {
        let order = self.resolve_load_order(module_names)?;

        // Build dependency graph
        let mut levels: Vec<Vec<String>> = Vec::new();
        let mut processed = std::collections::HashSet::new();

        for name in order {
            let entry = self
                .registry
                .get_by_name(&name)
                .ok_or_else(|| anyhow!("Module not found: {}", name))?;

            // Check if all dependencies are processed
            let mut level = 0;
            for dep in &entry.info.dependencies {
                if processed.contains(dep) {
                    level = level.max(
                        levels
                            .iter()
                            .position(|group| group.contains(dep))
                            .unwrap_or(0)
                            + 1,
                    );
                }
            }

            // Add to appropriate level
            while levels.len() <= level {
                levels.push(Vec::new());
            }
            levels[level].push(name.clone());
            processed.insert(name);
        }

        Ok(levels)
    }

    /// Validate module graph (check for cycles, missing deps)
    pub fn validate(&self) -> Result<()> {
        let all_modules: Vec<String> = self
            .registry
            .all()
            .iter()
            .map(|e| e.info.name.clone())
            .collect();

        for module_name in all_modules {
            // Try to resolve - will detect cycles
            self.resolve_load_order(&[&module_name])?;

            // Check dependencies exist
            self.check_dependencies(&module_name)?;
        }

        Ok(())
    }
}

impl Clone for ModuleResolver {
    fn clone(&self) -> Self {
        Self {
            registry: self.registry.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{ModuleId, ModuleInfo};

    #[test]
    fn test_simple_dependency_resolution() {
        let registry = ModuleRegistry::new();

        // Create modules: A -> B -> C
        let c_info = ModuleInfo {
            id: ModuleId::from_name("C"),
            name: "C".to_string(),
            version: "1.0.0".to_string(),
            description: "Module C".to_string(),
            author: "test".to_string(),
            dependencies: vec![],
            capabilities: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            interface_version: "1.0".to_string(),
            phase: 1,
            source_path: "/umd/C".to_string(),
            canonical_path: "/sylva/C".to_string(),
            spec_path: "/axiom/C".to_string(),
            metadata: Default::default(),
        };

        let b_info = ModuleInfo {
            id: ModuleId::from_name("B"),
            name: "B".to_string(),
            version: "1.0.0".to_string(),
            description: "Module B".to_string(),
            author: "test".to_string(),
            dependencies: vec!["C".to_string()],
            capabilities: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            interface_version: "1.0".to_string(),
            phase: 1,
            source_path: "/umd/B".to_string(),
            canonical_path: "/sylva/B".to_string(),
            spec_path: "/axiom/B".to_string(),
            metadata: Default::default(),
        };

        let a_info = ModuleInfo {
            id: ModuleId::from_name("A"),
            name: "A".to_string(),
            version: "1.0.0".to_string(),
            description: "Module A".to_string(),
            author: "test".to_string(),
            dependencies: vec!["B".to_string()],
            capabilities: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            interface_version: "1.0".to_string(),
            phase: 1,
            source_path: "/umd/A".to_string(),
            canonical_path: "/sylva/A".to_string(),
            spec_path: "/axiom/A".to_string(),
            metadata: Default::default(),
        };

        registry.register(c_info).unwrap();
        registry.register(b_info).unwrap();
        registry.register(a_info).unwrap();

        let resolver = ModuleResolver::new(registry);
        let order = resolver.resolve_load_order(&["A"]).unwrap();

        assert_eq!(order, vec!["C", "B", "A"]);
    }
}
