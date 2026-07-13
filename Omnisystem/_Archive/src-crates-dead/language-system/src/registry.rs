//! Language registration and discovery system

use crate::frontend::LanguageFrontend;
use std::collections::HashMap;
use std::sync::RwLock;

/// Entry in the language registry
#[derive(Clone)]
pub struct LanguageRegistration {
    pub name: String,
    pub factory: fn() -> Box<dyn LanguageFrontend>,
}

inventory::collect!(LanguageRegistration);

/// Global language registry
pub struct LanguageRegistry {
    ext_map: RwLock<HashMap<String, String>>, // extension -> language name
}

impl LanguageRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        let mut ext_map = HashMap::new();

        // Discover all registered languages
        for item in inventory::iter::<LanguageRegistration>() {
            let frontend = (item.factory)();
            for ext in frontend.file_extensions() {
                ext_map.insert(ext.to_string(), item.name.clone());
            }
        }

        Self {
            ext_map: RwLock::new(ext_map),
        }
    }

    /// Get a language frontend by name
    pub fn get(&self, name: &str) -> Option<Box<dyn LanguageFrontend>> {
        // Find in inventory
        for item in inventory::iter::<LanguageRegistration>() {
            if item.name == name {
                return Some((item.factory)());
            }
        }

        None
    }

    /// Get language by file extension
    pub fn get_by_extension(&self, ext: &str) -> Option<String> {
        self.ext_map.read().unwrap().get(ext).cloned()
    }

    /// List all registered languages
    pub fn list_all(&self) -> Vec<String> {
        inventory::iter::<LanguageRegistration>()
            .map(|item| item.name.clone())
            .collect()
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}
