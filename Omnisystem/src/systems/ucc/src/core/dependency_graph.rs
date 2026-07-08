//! Dependency graph for build units

use crate::core::CompilationUnit;
use std::collections::{HashMap, VecDeque};

/// Directed acyclic graph of compilation units
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    units: HashMap<String, CompilationUnit>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            units: HashMap::new(),
        }
    }

    /// Add a compilation unit to the graph
    pub fn add_unit(&mut self, unit: CompilationUnit) {
        self.units.insert(unit.id.clone(), unit);
    }

    /// Get a unit by ID
    pub fn get_unit(&self, id: &str) -> Option<&CompilationUnit> {
        self.units.get(id)
    }

    /// Get all units
    pub fn units(&self) -> impl Iterator<Item = &CompilationUnit> {
        self.units.values()
    }

    /// Get all unit IDs
    pub fn unit_ids(&self) -> Vec<String> {
        self.units.keys().cloned().collect()
    }

    /// Compute topological sort (for build order)
    pub fn topological_sort(&self) -> Result<Vec<String>, String> {
        let mut sorted = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();

        for unit_id in self.units.keys() {
            if !visited.contains(unit_id) {
                self.visit(unit_id, &mut visited, &mut visiting, &mut sorted)?;
            }
        }

        sorted.reverse();
        Ok(sorted)
    }

    fn visit(
        &self,
        unit_id: &str,
        visited: &mut std::collections::HashSet<String>,
        visiting: &mut std::collections::HashSet<String>,
        sorted: &mut Vec<String>,
    ) -> Result<(), String> {
        if visited.contains(unit_id) {
            return Ok(());
        }

        if visiting.contains(unit_id) {
            return Err(format!("Circular dependency detected involving {}", unit_id));
        }

        visiting.insert(unit_id.to_string());

        if let Some(unit) = self.units.get(unit_id) {
            for dep_id in &unit.dependencies {
                self.visit(dep_id, visited, visiting, sorted)?;
            }
        }

        visiting.remove(unit_id);
        visited.insert(unit_id.to_string());
        sorted.push(unit_id.to_string());

        Ok(())
    }

    /// Find all units that depend on a given unit
    pub fn find_dependents(&self, unit_id: &str) -> Vec<String> {
        self.units
            .values()
            .filter(|unit| unit.dependencies.contains(unit_id))
            .map(|unit| unit.id.clone())
            .collect()
    }

    /// Get the critical path (longest path from root to leaf)
    pub fn critical_path(&self) -> Vec<String> {
        let mut max_path = Vec::new();

        for unit in self.units.values() {
            if unit.is_independent() {
                let path = self.longest_path(&unit.id);
                if path.len() > max_path.len() {
                    max_path = path;
                }
            }
        }

        max_path
    }

    fn longest_path(&self, unit_id: &str) -> Vec<String> {
        let mut max_path = vec![unit_id.to_string()];

        if let Some(unit) = self.units.get(unit_id) {
            for dep_id in &unit.dependencies {
                let sub_path = self.longest_path(dep_id);
                if sub_path.len() + 1 > max_path.len() {
                    max_path = sub_path;
                    max_path.push(unit_id.to_string());
                }
            }
        }

        max_path
    }

    /// Estimate critical path time
    pub fn critical_path_duration_ms(&self) -> u128 {
        self.critical_path()
            .iter()
            .map(|id| {
                self.units
                    .get(id)
                    .map(|u| u.estimated_duration_ms)
                    .unwrap_or(0)
            })
            .sum()
    }

    /// Get units that can be compiled in parallel
    pub fn parallelizable_units(&self) -> Vec<Vec<String>> {
        let order = match self.topological_sort() {
            Ok(order) => order,
            Err(_) => return vec![],
        };

        let mut levels = Vec::new();
        let mut processed = std::collections::HashSet::new();

        for unit_id in order {
            if processed.contains(&unit_id) {
                continue;
            }

            let unit = match self.units.get(&unit_id) {
                Some(u) => u,
                None => continue,
            };

            if unit.dependencies.iter().all(|d| processed.contains(d)) {
                let level = vec![unit_id.clone()];
                processed.insert(unit_id);
                levels.push(level);
            }
        }

        levels
    }

    /// Check if graph is valid (no cycles)
    pub fn is_valid(&self) -> bool {
        self.topological_sort().is_ok()
    }

    /// Get total number of units
    pub fn unit_count(&self) -> usize {
        self.units.len()
    }

    /// Calculate estimated total compilation time (with infinite parallelism)
    pub fn estimated_total_time_ms(&self) -> u128 {
        self.units.values().map(|u| u.estimated_duration_ms).sum()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::Language;
    use std::path::PathBuf;

    #[test]
    fn test_dependency_graph_creation() {
        let graph = DependencyGraph::new();
        assert_eq!(graph.unit_count(), 0);
    }

    #[test]
    fn test_add_units() {
        let mut graph = DependencyGraph::new();
        let unit = CompilationUnit::new("test", "Test", Language::Rust, PathBuf::from("."));
        graph.add_unit(unit);
        assert_eq!(graph.unit_count(), 1);
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = DependencyGraph::new();

        let mut core = CompilationUnit::new("core", "Core", Language::Rust, PathBuf::from("."));
        graph.add_unit(core);

        let mut lib = CompilationUnit::new("lib", "Lib", Language::Rust, PathBuf::from("."));
        lib.add_dependency("core");
        graph.add_unit(lib);

        let mut main = CompilationUnit::new("main", "Main", Language::Rust, PathBuf::from("."));
        main.add_dependency("lib");
        graph.add_unit(main);

        let sorted = graph.topological_sort().unwrap();
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0], "core");
        assert_eq!(sorted[1], "lib");
        assert_eq!(sorted[2], "main");
    }

    #[test]
    fn test_is_valid() {
        let mut graph = DependencyGraph::new();
        let unit = CompilationUnit::new("test", "Test", Language::Rust, PathBuf::from("."));
        graph.add_unit(unit);
        assert!(graph.is_valid());
    }
}
