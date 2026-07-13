/// Dependency graph tracking for smart re-linting.
/// Computes which files are affected when a single file changes.

use anyhow::Result;
use dashmap::DashMap;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tree_sitter::Tree;

/// Represents import/reference relationships between files.
pub struct DependencyGraph {
    /// Forward edges: file -> (imported_from, import_type)
    forward_edges: Arc<DashMap<PathBuf, Vec<(PathBuf, String)>>>,
    /// Reverse edges: imported_from -> [files that import it]
    reverse_edges: Arc<DashMap<PathBuf, Vec<PathBuf>>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            forward_edges: Arc::new(DashMap::new()),
            reverse_edges: Arc::new(DashMap::new()),
        }
    }

    /// Add an edge from `from_file` that imports `to_file`.
    pub fn add_edge(&self, from_file: &Path, to_file: &Path, import_type: &str) -> Result<()> {
        // Add to forward edges
        self.forward_edges
            .entry(from_file.to_path_buf())
            .or_insert_with(Vec::new)
            .push((to_file.to_path_buf(), import_type.to_string()));

        // Add to reverse edges
        self.reverse_edges
            .entry(to_file.to_path_buf())
            .or_insert_with(Vec::new)
            .push(from_file.to_path_buf());

        tracing::debug!(
            "Added dependency: {:?} imports {:?} ({})",
            from_file,
            to_file,
            import_type
        );

        Ok(())
    }

    /// Compute all files that depend on `changed_file` (blast radius).
    /// Returns the transitive closure of all dependents.
    pub fn compute_blast_radius(&self, changed_file: &Path) -> Result<Vec<PathBuf>> {
        let mut affected = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with direct dependents
        if let Some(dependents) = self.reverse_edges.get(changed_file) {
            for dependent in dependents.iter() {
                queue.push_back(dependent.clone());
                affected.insert(dependent.clone());
            }
        }

        // BFS to find all transitive dependents
        while let Some(current) = queue.pop_front() {
            if let Some(dependents) = self.reverse_edges.get(&current) {
                for dependent in dependents.iter() {
                    if !affected.contains(dependent) {
                        affected.insert(dependent.clone());
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }

        let mut result: Vec<_> = affected.into_iter().collect();
        result.sort();

        tracing::info!(
            "Blast radius for {:?}: {} affected files",
            changed_file,
            result.len()
        );

        Ok(result)
    }

    /// Extract imports from a parsed tree (language-aware).
    pub fn extract_imports(
        &self,
        file: &Path,
        tree: &Tree,
        source: &str,
        language: &str,
    ) -> Result<Vec<(PathBuf, String)>> {
        let mut imports = Vec::new();

        match language {
            "rust" => {
                imports.extend(self.extract_rust_imports(file, tree, source)?);
            }
            "python" => {
                imports.extend(self.extract_python_imports(file, tree, source)?);
            }
            "typescript" | "javascript" => {
                imports.extend(self.extract_js_imports(file, tree, source)?);
            }
            _ => {
                // Other languages: skip for now
            }
        }

        Ok(imports)
    }

    fn extract_rust_imports(
        &self,
        _file: &Path,
        tree: &Tree,
        source: &str,
    ) -> Result<Vec<(PathBuf, String)>> {
        let mut imports = Vec::new();
        let mut cursor = tree.walk();

        for node in cursor.named_children(&mut tree.root_node()) {
            if node.kind() == "use_declaration" {
                if let Ok(text) = node.utf8_text(source.as_bytes()) {
                    // Parse "use module::submodule;"
                    let cleaned = text
                        .trim_start_matches("use")
                        .trim_end_matches(";")
                        .trim();
                    let import_path = PathBuf::from(cleaned.replace("::", "/"));
                    imports.push((import_path, "use".to_string()));
                }
            }
        }

        Ok(imports)
    }

    fn extract_python_imports(
        &self,
        _file: &Path,
        tree: &Tree,
        source: &str,
    ) -> Result<Vec<(PathBuf, String)>> {
        let mut imports = Vec::new();
        let mut cursor = tree.walk();

        for node in cursor.named_children(&mut tree.root_node()) {
            match node.kind() {
                "import_statement" => {
                    if let Ok(text) = node.utf8_text(source.as_bytes()) {
                        let cleaned = text
                            .trim_start_matches("import")
                            .trim()
                            .split_whitespace()
                            .next()
                            .unwrap_or("");
                        let import_path = PathBuf::from(cleaned.replace(".", "/"));
                        imports.push((import_path, "import".to_string()));
                    }
                }
                "import_from_statement" => {
                    if let Ok(text) = node.utf8_text(source.as_bytes()) {
                        let cleaned = text
                            .trim_start_matches("from")
                            .split(" import ")
                            .next()
                            .unwrap_or("")
                            .trim();
                        let import_path = PathBuf::from(cleaned.replace(".", "/"));
                        imports.push((import_path, "from".to_string()));
                    }
                }
                _ => {}
            }
        }

        Ok(imports)
    }

    fn extract_js_imports(
        &self,
        _file: &Path,
        tree: &Tree,
        source: &str,
    ) -> Result<Vec<(PathBuf, String)>> {
        let mut imports = Vec::new();
        let mut cursor = tree.walk();

        for node in cursor.named_children(&mut tree.root_node()) {
            match node.kind() {
                "import_statement" => {
                    if let Ok(text) = node.utf8_text(source.as_bytes()) {
                        // Parse "import X from 'path';"
                        if let Some(start) = text.find('\'') {
                            if let Some(end) = text[start + 1..].find('\'') {
                                let path_str = &text[start + 1..start + 1 + end];
                                let import_path = PathBuf::from(path_str);
                                imports.push((import_path, "import".to_string()));
                            }
                        } else if let Some(start) = text.find('"') {
                            if let Some(end) = text[start + 1..].find('"') {
                                let path_str = &text[start + 1..start + 1 + end];
                                let import_path = PathBuf::from(path_str);
                                imports.push((import_path, "import".to_string()));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(imports)
    }

    /// Get all files that import `target_file`.
    pub fn get_dependents(&self, target_file: &Path) -> Vec<PathBuf> {
        self.reverse_edges
            .get(target_file)
            .map(|deps| deps.clone())
            .unwrap_or_default()
    }

    /// Get all files that `source_file` imports.
    pub fn get_dependencies(&self, source_file: &Path) -> Vec<(PathBuf, String)> {
        self.forward_edges
            .get(source_file)
            .map(|deps| deps.clone())
            .unwrap_or_default()
    }

    /// Clear the graph.
    pub fn clear(&self) {
        self.forward_edges.clear();
        self.reverse_edges.clear();
    }

    /// Get total number of tracked files.
    pub fn file_count(&self) -> usize {
        self.forward_edges.len().max(self.reverse_edges.len())
    }

    /// Get total number of edges.
    pub fn edge_count(&self) -> usize {
        self.forward_edges
            .iter()
            .map(|entry| entry.value().len())
            .sum()
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

    #[test]
    fn test_add_edge() {
        let graph = DependencyGraph::new();
        graph
            .add_edge(Path::new("main.rs"), Path::new("lib.rs"), "use")
            .unwrap();

        assert_eq!(graph.file_count(), 2);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_blast_radius() {
        let graph = DependencyGraph::new();
        // main.rs imports lib.rs
        graph
            .add_edge(Path::new("main.rs"), Path::new("lib.rs"), "use")
            .unwrap();
        // util.rs imports lib.rs
        graph
            .add_edge(Path::new("util.rs"), Path::new("lib.rs"), "use")
            .unwrap();

        let affected = graph.compute_blast_radius(Path::new("lib.rs")).unwrap();
        assert_eq!(affected.len(), 2);
    }

    #[test]
    fn test_transitive_dependencies() {
        let graph = DependencyGraph::new();
        // app imports main
        graph
            .add_edge(Path::new("app.rs"), Path::new("main.rs"), "use")
            .unwrap();
        // main imports lib
        graph
            .add_edge(Path::new("main.rs"), Path::new("lib.rs"), "use")
            .unwrap();

        let affected = graph.compute_blast_radius(Path::new("lib.rs")).unwrap();
        // Should find main.rs directly, and app.rs transitively
        assert_eq!(affected.len(), 2);
    }
}
