use crate::{AppId, ModuleNode, Result, AppManagerError, ModuleState};
use dashmap::DashMap;
use std::sync::Arc;
use std::collections::{HashSet, VecDeque};

pub struct DependencyGraph {
    nodes: Arc<DashMap<AppId, ModuleNode>>,
    edges: Arc<DashMap<(AppId, AppId), ()>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        DependencyGraph {
            nodes: Arc::new(DashMap::new()),
            edges: Arc::new(DashMap::new()),
        }
    }

    pub fn add_module(&self, node: ModuleNode) -> Result<()> {
        self.nodes.insert(node.id.clone(), node);
        tracing::debug!("Added module to dependency graph");
        Ok(())
    }

    pub fn remove_module(&self, app_id: &AppId) -> Result<()> {
        if let Some((_, node)) = self.nodes.remove(app_id) {
            for dependent in &node.dependents {
                self.edges.remove(&(dependent.clone(), app_id.clone()));
            }
            tracing::debug!("Removed module from dependency graph");
        }
        Ok(())
    }

    pub fn add_dependency(&self, from: &AppId, to: &AppId) -> Result<()> {
        self.edges.insert((from.clone(), to.clone()), ());
        if let Some(mut node) = self.nodes.get_mut(to) {
            if !node.dependents.contains(from) {
                node.dependents.push(from.clone());
            }
        }
        Ok(())
    }

    pub fn resolve_dependencies(&self, app_id: &AppId) -> Result<Vec<AppId>> {
        let mut resolved = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(app_id.clone());

        while let Some(current) = queue.pop_front() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            if let Some(node) = self.nodes.get(&current) {
                for dep in &node.dependencies {
                    if !dep.optional {
                        queue.push_back(dep.app_id.clone());
                    }
                }
            }
        }

        for app in visited {
            if &app != app_id {
                resolved.push(app);
            }
        }

        Ok(resolved)
    }

    pub fn detect_circular_dependencies(&self) -> Result<Vec<Vec<AppId>>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node in self.nodes.iter() {
            if !visited.contains(node.key()) {
                self.dfs_cycle(&node.key().clone(), &mut visited, &mut rec_stack, &mut path, &mut cycles);
            }
        }

        if !cycles.is_empty() {
            return Err(AppManagerError::CircularDependency(
                format!("{} cycles detected", cycles.len())
            ));
        }

        Ok(cycles)
    }

    fn dfs_cycle(
        &self,
        app_id: &AppId,
        visited: &mut HashSet<AppId>,
        rec_stack: &mut HashSet<AppId>,
        path: &mut Vec<AppId>,
        cycles: &mut Vec<Vec<AppId>>,
    ) {
        visited.insert(app_id.clone());
        rec_stack.insert(app_id.clone());
        path.push(app_id.clone());

        if let Some(node) = self.nodes.get(app_id) {
            for dep in &node.dependencies {
                if !visited.contains(&dep.app_id) {
                    self.dfs_cycle(&dep.app_id, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(&dep.app_id) {
                    let cycle_start = path.iter().position(|x| x == &dep.app_id).unwrap();
                    cycles.push(path[cycle_start..].to_vec());
                }
            }
        }

        rec_stack.remove(app_id);
        path.pop();
    }

    pub fn topological_sort(&self) -> Result<Vec<AppId>> {
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        for node in self.nodes.iter() {
            if !visited.contains(node.key()) {
                self.topo_dfs(node.key(), &mut visited, &mut visiting, &mut sorted)?;
            }
        }

        sorted.reverse();
        Ok(sorted)
    }

    fn topo_dfs(
        &self,
        app_id: &AppId,
        visited: &mut HashSet<AppId>,
        visiting: &mut HashSet<AppId>,
        sorted: &mut Vec<AppId>,
    ) -> Result<()> {
        if visiting.contains(app_id) {
            return Err(AppManagerError::CircularDependency(format!("At {}", app_id)));
        }

        if visited.contains(app_id) {
            return Ok(());
        }

        visiting.insert(app_id.clone());

        if let Some(node) = self.nodes.get(app_id) {
            for dep in &node.dependencies {
                self.topo_dfs(&dep.app_id, visited, visiting, sorted)?;
            }
        }

        visiting.remove(app_id);
        visited.insert(app_id.clone());
        sorted.push(app_id.clone());

        Ok(())
    }

    pub fn find_orphaned_modules(&self) -> Result<Vec<AppId>> {
        let mut orphaned = Vec::new();

        for node in self.nodes.iter() {
            if node.dependents.is_empty() && node.state != ModuleState::Loaded && node.state != ModuleState::Running {
                orphaned.push(node.key().clone());
            }
        }

        Ok(orphaned)
    }

    pub fn get_dependents(&self, app_id: &AppId) -> Result<Vec<AppId>> {
        if let Some(node) = self.nodes.get(app_id) {
            Ok(node.dependents.clone())
        } else {
            Err(AppManagerError::AppNotFound(app_id.to_string()))
        }
    }

    pub fn get_module(&self, app_id: &AppId) -> Result<ModuleNode> {
        self.nodes
            .get(app_id)
            .map(|r| r.clone())
            .ok_or_else(|| AppManagerError::AppNotFound(app_id.to_string()))
    }

    pub fn list_all_modules(&self) -> Result<Vec<ModuleNode>> {
        Ok(self.nodes.iter().map(|r| r.value().clone()).collect())
    }

    pub fn module_count(&self) -> usize {
        self.nodes.len()
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
    use crate::{Version, ModuleState, Dependency};

    fn make_node(id: &str) -> ModuleNode {
        ModuleNode {
            id: AppId::new(id).unwrap(),
            version: Version::new(1, 0, 0),
            dependencies: Vec::new(),
            dependents: Vec::new(),
            conflicts: Vec::new(),
            state: ModuleState::Loaded,
        }
    }

    #[test]
    fn test_add_and_get_module() {
        let graph = DependencyGraph::new();
        let node = make_node("test-app");
        graph.add_module(node.clone()).unwrap();
        let retrieved = graph.get_module(&node.id).unwrap();
        assert_eq!(retrieved.id, node.id);
    }

    #[test]
    fn test_topological_sort() {
        let graph = DependencyGraph::new();
        graph.add_module(make_node("a")).unwrap();
        graph.add_module(make_node("b")).unwrap();
        graph.add_module(make_node("c")).unwrap();

        let sorted = graph.topological_sort().unwrap();
        assert_eq!(sorted.len(), 3);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let graph = DependencyGraph::new();
        let mut node_a = make_node("a");
        let mut node_b = make_node("b");

        node_a.dependencies.push(Dependency {
            app_id: AppId::new("b").unwrap(),
            version_constraint: crate::VersionConstraint::AtLeast(Version::new(1, 0, 0)),
            optional: false,
            dev_only: false,
        });

        node_b.dependencies.push(Dependency {
            app_id: AppId::new("a").unwrap(),
            version_constraint: crate::VersionConstraint::AtLeast(Version::new(1, 0, 0)),
            optional: false,
            dev_only: false,
        });

        graph.add_module(node_a).unwrap();
        graph.add_module(node_b).unwrap();

        let result = graph.detect_circular_dependencies();
        assert!(result.is_err());
    }

    #[test]
    fn test_module_count() {
        let graph = DependencyGraph::new();
        graph.add_module(make_node("a")).unwrap();
        graph.add_module(make_node("b")).unwrap();
        assert_eq!(graph.module_count(), 2);
    }

    #[test]
    fn test_remove_module() {
        let graph = DependencyGraph::new();
        let node = make_node("test");
        graph.add_module(node.clone()).unwrap();
        graph.remove_module(&node.id).unwrap();
        assert_eq!(graph.module_count(), 0);
    }
}
