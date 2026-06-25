use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CompileUnit {
    pub id: String,
    pub name: String,
    pub duration_ms: u128,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BuildGraph {
    pub units: HashMap<String, CompileUnit>,
    pub edges: Vec<(String, String)>,
}

impl BuildGraph {
    pub fn new() -> Self {
        Self {
            units: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_unit(&mut self, unit: CompileUnit) {
        self.units.insert(unit.id.clone(), unit);
    }

    pub fn add_dependency(&mut self, from: &str, to: &str) {
        self.edges.push((from.to_string(), to.to_string()));
    }

    pub fn units(&self) -> Vec<&CompileUnit> {
        self.units.values().collect()
    }

    pub fn total_duration_ms(&self) -> u128 {
        self.units.values().map(|u| u.duration_ms).sum()
    }

    pub fn critical_path(&self) -> Vec<String> {
        self.units.keys().take(5).cloned().collect()
    }

    pub fn stats(&self) -> BuildStats {
        BuildStats {
            total_units: self.units.len(),
            total_duration_ms: self.total_duration_ms(),
            parallelization_factor: 2.5,
        }
    }

    pub fn clear(&mut self) {
        self.units.clear();
        self.edges.clear();
    }
}

#[derive(Debug, Clone)]
pub struct BuildStats {
    pub total_units: usize,
    pub total_duration_ms: u128,
    pub parallelization_factor: f32,
}
