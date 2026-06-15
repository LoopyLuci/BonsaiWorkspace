//! Knowledge graph data structures and operations

use crate::Triple;
use std::collections::{HashMap, HashSet};

pub struct KnowledgeGraph {
    subjects: HashMap<String, HashSet<(String, String)>>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            subjects: HashMap::new(),
        }
    }

    pub fn add_triple(&mut self, triple: Triple) {
        self.subjects
            .entry(triple.subject)
            .or_insert_with(HashSet::new)
            .insert((triple.predicate, triple.object));
    }
}
