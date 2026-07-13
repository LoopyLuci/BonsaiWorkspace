use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugRecord {
    pub description: String,
    pub confidence: f64,
    pub times_encountered: u32,
}

pub struct SurvivalDatabase {
    bugs: HashMap<String, BugRecord>,
}

impl SurvivalDatabase {
    pub fn new() -> Self {
        Self {
            bugs: HashMap::new(),
        }
    }

    pub fn insert(&mut self, signature: String, record: BugRecord) {
        self.bugs.insert(signature, record);
    }

    pub fn get(&self, signature: &str) -> Option<BugRecord> {
        self.bugs.get(signature).cloned()
    }

    pub fn len(&self) -> usize {
        self.bugs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bugs.is_empty()
    }
}

impl Default for SurvivalDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_creation() {
        let db = SurvivalDatabase::new();
        assert!(db.is_empty());
    }

    #[test]
    fn test_insert_bug() {
        let mut db = SurvivalDatabase::new();
        db.insert(
            "sig1".to_string(),
            BugRecord {
                description: "test".to_string(),
                confidence: 0.9,
                times_encountered: 1,
            },
        );
        assert_eq!(db.len(), 1);
    }
}
