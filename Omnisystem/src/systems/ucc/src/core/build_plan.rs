//! Build plan - execution plan for compilation

use crate::core::{CompilationUnit, DependencyGraph};
use std::time::Duration;

/// Build execution plan
#[derive(Debug, Clone)]
pub struct BuildPlan {
    pub units: Vec<CompilationUnit>,
    pub dependency_graph: DependencyGraph,
    pub build_order: Vec<Vec<String>>, // Parallelizable waves
    pub critical_path_duration: Duration,
    pub estimated_total_duration: Duration,
    pub parallelization_potential: f32,
}

impl BuildPlan {
    /// Create a new build plan
    pub fn new(
        units: Vec<CompilationUnit>,
        dependency_graph: DependencyGraph,
    ) -> Result<Self, String> {
        let build_order = dependency_graph.parallelizable_units();
        let critical_path_duration =
            Duration::from_millis(dependency_graph.critical_path_duration_ms() as u64);
        let estimated_total_duration =
            Duration::from_millis(dependency_graph.estimated_total_time_ms() as u64);

        let parallelization_potential = if estimated_total_duration.as_millis() > 0 {
            estimated_total_duration.as_millis() as f32
                / critical_path_duration.as_millis().max(1) as f32
        } else {
            1.0
        };

        Ok(Self {
            units,
            dependency_graph,
            build_order,
            critical_path_duration,
            estimated_total_duration,
            parallelization_potential,
        })
    }

    pub fn unit_count(&self) -> usize {
        self.units.len()
    }

    pub fn wave_count(&self) -> usize {
        self.build_order.len()
    }

    pub fn max_parallelism(&self) -> usize {
        self.build_order.iter().map(|wave| wave.len()).max().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::Language;
    use std::path::PathBuf;

    #[test]
    fn test_build_plan_creation() {
        let mut graph = DependencyGraph::new();
        let unit = CompilationUnit::new("test", "Test", Language::Rust, PathBuf::from("."));
        graph.add_unit(unit.clone());

        let plan = BuildPlan::new(vec![unit], graph).unwrap();
        assert_eq!(plan.unit_count(), 1);
    }
}
