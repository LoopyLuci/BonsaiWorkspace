use crate::{Dictionary, Result};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;

pub struct Translator {
    dictionaries: Arc<DashMap<String, Dictionary>>,
    translation_cache: Arc<DashMap<String, String>>,
}

impl Translator {
    pub fn new() -> Self {
        Self {
            dictionaries: Arc::new(DashMap::new()),
            translation_cache: Arc::new(DashMap::new()),
        }
    }

    pub fn register_dictionary(&self, lang_pair: String, dict: Dictionary) -> Result<()> {
        self.dictionaries.insert(lang_pair, dict);
        Ok(())
    }

    pub fn translate(&self, source_lang: &str, target_lang: &str, text: &str) -> Result<String> {
        let cache_key = format!("{}→{}:{}", source_lang, target_lang, text);
        
        if let Some(cached) = self.translation_cache.get(&cache_key) {
            return Ok(cached.value().clone());
        }

        let translated = format!("{} [translated from {} to {}]", text, source_lang, target_lang);
        self.translation_cache.insert(cache_key, translated.clone());
        
        tracing::info!("Translation completed");
        Ok(translated)
    }

    pub fn clear_cache(&self) {
        self.translation_cache.clear();
    }

    pub fn cache_size(&self) -> usize {
        self.translation_cache.len()
    }
}

impl Default for Translator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_translator() {
        let translator = Translator::new();
        let result = translator.translate("en", "es", "hello").unwrap();
        assert!(result.contains("hello"));
    }
}
