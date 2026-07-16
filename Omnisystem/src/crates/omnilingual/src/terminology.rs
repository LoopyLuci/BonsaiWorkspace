use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct TerminologyStore {
    terms: Arc<DashMap<String, TermEntry>>,
}

#[derive(Debug, Clone)]
pub struct TermEntry {
    pub source: String,
    pub target: String,
    pub domain: String,
    pub frequency: u32,
}

impl TerminologyStore {
    pub fn new() -> Self {
        Self {
            terms: Arc::new(DashMap::new()),
        }
    }

    pub fn add_term(&self, entry: TermEntry) -> Result<()> {
        let key = format!("{}::{}", entry.domain, entry.source);
        self.terms.insert(key, entry);
        tracing::info!("Term added");
        Ok(())
    }

    pub fn lookup_term(&self, domain: &str, term: &str) -> Option<TermEntry> {
        let key = format!("{}::{}", domain, term);
        self.terms.get(&key).map(|e| e.value().clone())
    }

    pub fn term_count(&self) -> usize {
        self.terms.len()
    }
}

impl Default for TerminologyStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminology() {
        let store = TerminologyStore::new();
        let entry = TermEntry {
            source: "algorithm".to_string(),
            target: "algoritmo".to_string(),
            domain: "computer_science".to_string(),
            frequency: 100,
        };
        assert!(store.add_term(entry).is_ok());
        assert_eq!(store.term_count(), 1);
    }
}
