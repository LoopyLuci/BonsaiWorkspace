//! Language Compatibility Graph Analysis
//!
//! Builds and analyzes a graph of language conversion compatibility.
//! Identifies language clusters, bridges, and relationships.

use polyglot_pong_common::*;
use std::collections::{HashMap, HashSet};

/// Language compatibility graph.
#[derive(Debug, Clone)]
pub struct LanguageGraph {
    pub nodes: Vec<Language>,
    pub edges: HashMap<(Language, Language), f32>, // fidelity scores
    pub node_index: HashMap<Language, usize>,
}

impl LanguageGraph {
    /// Create a new graph with language list.
    pub fn new(languages: Vec<Language>) -> Self {
        let node_index = languages
            .iter()
            .enumerate()
            .map(|(i, lang)| (lang.clone(), i))
            .collect();

        Self {
            nodes: languages,
            edges: HashMap::new(),
            node_index,
        }
    }

    /// Add or update an edge (fidelity between two languages).
    pub fn add_edge(&mut self, src: &Language, tgt: &Language, fidelity: f32) {
        self.edges.insert((src.clone(), tgt.clone()), fidelity);
    }

    /// Get fidelity between two languages.
    pub fn get_fidelity(&self, src: &Language, tgt: &Language) -> Option<f32> {
        self.edges.get(&(src.clone(), tgt.clone())).copied()
    }

    /// Update graph from test results.
    pub fn update_from_results(&mut self, results: &[TestResult]) {
        // In production: extract (src, tgt) from results and compute fidelity
        // For now: placeholder
        for result in results {
            if result.success {
                // self.add_edge(&result.source_lang, &result.target_lang, 1.0);
            }
        }
    }

    /// Identify bridge languages (high centrality).
    pub fn bridge_languages(&self) -> Vec<Language> {
        let mut centrality: HashMap<Language, f32> = HashMap::new();

        // Compute betweenness centrality (simplified)
        for lang in &self.nodes {
            let mut count = 0;
            let mut total_fidelity = 0.0;

            for (i, src) in self.nodes.iter().enumerate() {
                for (j, tgt) in self.nodes.iter().enumerate() {
                    if i != j && lang != src && lang != tgt {
                        if let Some(fidelity) = self.get_fidelity(src, tgt) {
                            count += 1;
                            total_fidelity += fidelity;
                        }
                    }
                }
            }

            if count > 0 {
                centrality.insert(lang.clone(), total_fidelity / count as f32);
            }
        }

        let mut bridges: Vec<_> = centrality
            .into_iter()
            .map(|(lang, score)| (lang, score))
            .collect();
        bridges.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        bridges.into_iter().map(|(lang, _)| lang).collect()
    }

    /// Detect language clusters using simple threshold-based grouping.
    pub fn detect_clusters(&self, threshold: f32) -> Vec<LanguageCluster> {
        let mut clusters = Vec::new();
        let mut assigned = HashSet::new();

        for seed_lang in &self.nodes {
            if assigned.contains(seed_lang) {
                continue;
            }

            let mut cluster = vec![seed_lang.clone()];
            assigned.insert(seed_lang.clone());

            // Find similar languages
            for other_lang in &self.nodes {
                if !assigned.contains(other_lang) {
                    // Check average fidelity to cluster
                    let mut total_fidelity = 0.0;
                    let mut count = 0;

                    for member in &cluster {
                        if let Some(fidelity) = self.get_fidelity(member, other_lang) {
                            total_fidelity += fidelity;
                            count += 1;
                        }
                    }

                    if count > 0 && total_fidelity / count as f32 > threshold {
                        cluster.push(other_lang.clone());
                        assigned.insert(other_lang.clone());
                    }
                }
            }

            if !cluster.is_empty() {
                let centroid = cluster[0].clone();
                clusters.push(LanguageCluster {
                    name: format!("Cluster-{}", clusters.len() + 1),
                    members: cluster,
                    centroid_language: centroid,
                    avg_internal_fidelity: 0.8, // Placeholder
                });
            }
        }

        clusters
    }

    /// Export as Graphviz DOT format.
    pub fn to_graphviz(&self) -> String {
        let mut dot = String::from("digraph LanguageCompatibility {\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box];\n");

        // Add nodes
        for lang in &self.nodes {
            dot.push_str(&format!("  \"{}\";\n", lang));
        }

        // Add edges
        for ((src, tgt), fidelity) in &self.edges {
            let color = if *fidelity > 0.9 {
                "green"
            } else if *fidelity > 0.7 {
                "yellow"
            } else if *fidelity > 0.5 {
                "orange"
            } else {
                "red"
            };

            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{:.2}\", color=\"{}\"];\n",
                src, tgt, fidelity, color
            ));
        }

        dot.push_str("}\n");
        dot
    }

    /// Compute average fidelity.
    pub fn average_fidelity(&self) -> f32 {
        if self.edges.is_empty() {
            return 0.0;
        }

        let sum: f32 = self.edges.values().sum();
        sum / self.edges.len() as f32
    }

    /// Get statistics about the graph.
    pub fn statistics(&self) -> GraphStatistics {
        GraphStatistics {
            num_nodes: self.nodes.len(),
            num_edges: self.edges.len(),
            avg_fidelity: self.average_fidelity(),
            max_fidelity: self.edges.values().copied().fold(0.0, f32::max),
            min_fidelity: self.edges.values().copied().fold(1.0, f32::min),
        }
    }
}

/// Language cluster.
#[derive(Debug, Clone)]
pub struct LanguageCluster {
    pub name: String,
    pub members: Vec<Language>,
    pub centroid_language: Language,
    pub avg_internal_fidelity: f32,
}

/// Graph statistics.
#[derive(Debug, Clone)]
pub struct GraphStatistics {
    pub num_nodes: usize,
    pub num_edges: usize,
    pub avg_fidelity: f32,
    pub max_fidelity: f32,
    pub min_fidelity: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let langs = vec!["Rust".into(), "Python".into(), "Go".into()];
        let graph = LanguageGraph::new(langs);
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 0);
    }

    #[test]
    fn test_add_edge() {
        let langs = vec!["Rust".into(), "Python".into()];
        let mut graph = LanguageGraph::new(langs);

        graph.add_edge(&"Rust".into(), &"Python".into(), 0.85);
        assert_eq!(graph.get_fidelity(&"Rust".into(), &"Python".into()), Some(0.85));
    }

    #[test]
    fn test_graphviz_export() {
        let langs = vec!["Rust".into(), "Python".into()];
        let mut graph = LanguageGraph::new(langs);
        graph.add_edge(&"Rust".into(), &"Python".into(), 0.85);

        let dot = graph.to_graphviz();
        assert!(dot.contains("digraph"));
        assert!(dot.contains("Rust"));
        assert!(dot.contains("Python"));
        assert!(dot.contains("0.85"));
    }

    #[test]
    fn test_cluster_detection() {
        let langs = vec!["Rust".into(), "C".into(), "Python".into(), "Haskell".into()];
        let mut graph = LanguageGraph::new(langs);

        // High fidelity within C-like languages
        graph.add_edge(&"Rust".into(), &"C".into(), 0.95);
        graph.add_edge(&"C".into(), &"Rust".into(), 0.95);

        // Low fidelity between C-like and functional
        graph.add_edge(&"Rust".into(), &"Haskell".into(), 0.3);
        graph.add_edge(&"Haskell".into(), &"Rust".into(), 0.3);

        let clusters = graph.detect_clusters(0.8);
        assert!(clusters.len() > 0);
    }

    #[test]
    fn test_graph_statistics() {
        let langs = vec!["A".into(), "B".into()];
        let mut graph = LanguageGraph::new(langs);
        graph.add_edge(&"A".into(), &"B".into(), 0.7);
        graph.add_edge(&"B".into(), &"A".into(), 0.8);

        let stats = graph.statistics();
        assert_eq!(stats.num_nodes, 2);
        assert_eq!(stats.num_edges, 2);
        assert!(stats.avg_fidelity > 0.7 && stats.avg_fidelity < 0.8);
    }
}
