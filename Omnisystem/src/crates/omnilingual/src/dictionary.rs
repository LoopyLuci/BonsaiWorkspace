use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct Dictionary {
    source_lang: String,
    target_lang: String,
    entries: Arc<DashMap<String, Vec<String>>>,
}

impl Dictionary {
    pub fn new(source_lang: String, target_lang: String) -> Self {
        Self {
            source_lang,
            target_lang,
            entries: Arc::new(DashMap::new()),
        }
    }

    pub fn add_entry(&self, source: String, targets: Vec<String>) -> Result<()> {
        self.entries.insert(source, targets);
        tracing::info!("Dictionary entry added");
        Ok(())
    }

    pub fn lookup(&self, word: &str) -> Option<Vec<String>> {
        self.entries.get(word).map(|entry| entry.value().clone())
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dictionary() {
        let dict = Dictionary::new("en".to_string(), "es".to_string());
        dict.add_entry("hello".to_string(), vec!["hola".to_string()]).unwrap();
        assert_eq!(dict.lookup("hello"), Some(vec!["hola".to_string()]));
    }
}
