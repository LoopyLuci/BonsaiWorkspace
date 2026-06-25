use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub task: String,
    pub current: u64,
    pub total: u64,
    pub message: String,
}

impl ProgressUpdate {
    pub fn percentage(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.current as f32 / self.total as f32) * 100.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProgressNode {
    pub id: String,
    pub name: String,
    pub current: u64,
    pub total: u64,
    pub children: Vec<ProgressNode>,
}

impl ProgressNode {
    pub fn percentage(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.current as f32 / self.total as f32) * 100.0
        }
    }

    pub fn aggregate_percentage(&self) -> f32 {
        if self.children.is_empty() {
            self.percentage()
        } else {
            let sum: f32 = self.children.iter().map(|c| c.aggregate_percentage()).sum();
            sum / self.children.len() as f32
        }
    }
}

/// Real-time progress tracking
pub struct ProgressTracker {
    updates: VecDeque<ProgressUpdate>,
    tree: Option<ProgressNode>,
    max_history: usize,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            updates: VecDeque::with_capacity(1000),
            tree: None,
            max_history: 100,
        }
    }

    /// Record a progress update
    pub fn update(&mut self, update: ProgressUpdate) {
        self.updates.push_back(update);
        if self.updates.len() > self.max_history {
            self.updates.pop_front();
        }
    }

    /// Set progress tree
    pub fn set_tree(&mut self, tree: ProgressNode) {
        self.tree = Some(tree);
    }

    /// Get current progress tree
    pub fn get_tree(&self) -> Option<&ProgressNode> {
        self.tree.as_ref()
    }

    /// Get recent updates
    pub fn recent_updates(&self, count: usize) -> Vec<ProgressUpdate> {
        self.updates
            .iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Get all updates
    pub fn all_updates(&self) -> Vec<ProgressUpdate> {
        self.updates.iter().cloned().collect()
    }

    /// Clear history
    pub fn clear(&mut self) {
        self.updates.clear();
        self.tree = None;
    }
}

pub struct ProgressBuilder {
    root: ProgressNode,
    id_counter: usize,
}

impl ProgressBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            root: ProgressNode {
                id: "root".to_string(),
                name: name.into(),
                current: 0,
                total: 0,
                children: Vec::new(),
            },
            id_counter: 0,
        }
    }

    pub fn add_child(mut self, mut child: ProgressNode) -> Self {
        if child.id.is_empty() {
            child.id = format!("node_{}", self.id_counter);
            self.id_counter += 1;
        }
        self.root.children.push(child);
        self
    }

    pub fn build(self) -> ProgressNode {
        self.root
    }
}
