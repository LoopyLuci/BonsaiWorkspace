use core_ir::{LairFunction, LairNode};
use std::collections::{HashMap, HashSet};

pub struct HotspotProfiler {
    call_counts: HashMap<String, u64>,
}

impl HotspotProfiler {
    pub fn new() -> Self {
        Self { call_counts: HashMap::new() }
    }
    
    pub fn analyze(&mut self, func: &LairFunction) -> HashSet<usize> {
        let mut hotspots = HashSet::new();
        for (i, node) in func.body.iter().enumerate() {
            if let LairNode::Call { function, .. } = node {
                let count = self.call_counts.entry(function.clone()).or_insert(0);
                *count += 1;
                if *count > 10 {
                    hotspots.insert(i);
                }
            }
        }
        hotspots
    }
}
