use std::collections::HashMap;

pub struct RoutePath {
    pub nodes: Vec<u32>,
    pub cost: u32,
    pub active: bool,
}

pub struct AetherRouting {
    routes: HashMap<u32, Vec<RoutePath>>,
    neighbors: HashMap<u32, Vec<u32>>,
}

impl AetherRouting {
    pub fn new() -> Self {
        AetherRouting {
            routes: HashMap::new(),
            neighbors: HashMap::new(),
        }
    }

    pub fn add_neighbor(&mut self, node: u32, neighbor: u32) {
        self.neighbors.entry(node).or_insert_with(Vec::new).push(neighbor);
    }

    pub fn find_route(&self, dest: u32) -> Option<&RoutePath> {
        self.routes.get(&dest).and_then(|paths| paths.iter().find(|p| p.active))
    }

    pub fn add_route(&mut self, dest: u32, path: RoutePath) {
        self.routes.entry(dest).or_insert_with(Vec::new).push(path);
    }

    pub fn heal_route(&mut self, dest: u32) {
        if let Some(paths) = self.routes.get_mut(&dest) {
            paths.sort_by_key(|p| p.cost);
            if !paths.is_empty() {
                paths[0].active = true;
            }
        }
    }

    pub fn get_path_cost(&self, dest: u32) -> Option<u32> {
        self.find_route(dest).map(|p| p.cost)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing() {
        let mut rt = AetherRouting::new();
        rt.add_route(2, RoutePath { nodes: vec![1, 2], cost: 100, active: true });
        assert!(rt.find_route(2).is_some());
    }

    #[test]
    fn test_neighbors() {
        let mut rt = AetherRouting::new();
        rt.add_neighbor(1, 2);
        assert!(rt.neighbors.get(&1).unwrap().contains(&2));
    }
}
