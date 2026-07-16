use crate::segmentation::Segmenter;
use crate::{Dictionary, Result};
use dashmap::DashMap;
use std::sync::Arc;

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

    /// Translate `text` word-by-word using the dictionary registered for
    /// `source_lang` -> `target_lang` (via [`Translator::register_dictionary`]
    /// with a `"{source_lang}→{target_lang}"` key). Words with no dictionary
    /// entry (or when no dictionary is registered for the pair) are left
    /// untranslated rather than fabricated. Results are cached by
    /// (source_lang, target_lang, text).
    pub fn translate(&self, source_lang: &str, target_lang: &str, text: &str) -> Result<String> {
        let cache_key = format!("{}→{}:{}", source_lang, target_lang, text);

        if let Some(cached) = self.translation_cache.get(&cache_key) {
            return Ok(cached.value().clone());
        }

        let lang_pair = format!("{}→{}", source_lang, target_lang);
        let translated = match self.dictionaries.get(&lang_pair) {
            Some(dict) => Segmenter::segment_words(text)
                .into_iter()
                .map(|word| {
                    let lookup_key: String = word
                        .chars()
                        .filter(|c| c.is_alphanumeric())
                        .collect::<String>()
                        .to_lowercase();
                    dict.lookup(&lookup_key)
                        .and_then(|candidates| candidates.into_iter().next())
                        .unwrap_or(word)
                })
                .collect::<Vec<_>>()
                .join(" "),
            None => text.to_string(),
        };

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

    #[test]
    fn test_translator_leaves_text_untranslated_without_a_dictionary() {
        let translator = Translator::new();
        let result = translator.translate("en", "es", "hello").unwrap();
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_translator_uses_registered_dictionary() {
        let translator = Translator::new();
        let dict = Dictionary::new("en".to_string(), "es".to_string());
        dict.add_entry("hello".to_string(), vec!["hola".to_string()])
            .unwrap();
        dict.add_entry("world".to_string(), vec!["mundo".to_string()])
            .unwrap();
        translator
            .register_dictionary("en→es".to_string(), dict)
            .unwrap();

        let result = translator.translate("en", "es", "hello world").unwrap();
        assert_eq!(result, "hola mundo");
    }

    #[test]
    fn test_translator_caches_results() {
        let translator = Translator::new();
        translator.translate("en", "es", "hello").unwrap();
        assert_eq!(translator.cache_size(), 1);
        translator.translate("en", "es", "hello").unwrap();
        assert_eq!(translator.cache_size(), 1, "repeated call should hit the cache, not grow it");
    }
}
