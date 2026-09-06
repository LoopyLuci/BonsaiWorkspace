//! Keyboard Shortcuts System
//!
//! A real keybinding registry: register a key combination (e.g. "ctrl+s")
//! against a named action, look it up, list all bindings, and detect
//! conflicts. This replaces the earlier generic "Component Library"
//! placeholder with logic actually specific to this crate's name.
#![warn(missing_docs)]
pub mod error;
pub mod types;
pub use error::{Error, Result};
pub use types::*;

use std::collections::HashMap;
use tracing::info;

/// Normalizes a raw key-combo string (e.g. "Ctrl+S", "CTRL + s") into a
/// canonical form ("ctrl+s") so lookups are case/spacing-insensitive.
fn normalize(combo: &str) -> String {
    combo
        .split('+')
        .map(|part| part.trim().to_lowercase())
        .collect::<Vec<_>>()
        .join("+")
}

/// A single registered keybinding.
#[derive(Debug, Clone)]
pub struct Binding {
    /// Canonicalized key combination, e.g. "ctrl+shift+p".
    pub combo: String,
    /// Name of the action bound to this combination.
    pub action: String,
}

/// Registry of keyboard shortcuts, mapping key combinations to action names.
#[derive(Debug, Clone, Default)]
pub struct KeymapRegistry {
    bindings: HashMap<String, String>,
}

impl KeymapRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        info!("Init");
        Self { bindings: HashMap::new() }
    }

    /// Registers `action` under `combo`. Returns the action that previously
    /// occupied that combination, if any (i.e. this call overwrote it).
    pub fn register(&mut self, combo: &str, action: &str) -> Result<Option<String>> {
        if combo.trim().is_empty() {
            return Err(Error::Other("key combination cannot be empty".to_string()));
        }
        Ok(self.bindings.insert(normalize(combo), action.to_string()))
    }

    /// Removes the binding for `combo`, returning its action if one existed.
    pub fn unregister(&mut self, combo: &str) -> Option<String> {
        self.bindings.remove(&normalize(combo))
    }

    /// Looks up the action bound to `combo`, if any.
    pub fn lookup(&self, combo: &str) -> Option<&str> {
        self.bindings.get(&normalize(combo)).map(String::as_str)
    }

    /// Returns true if `combo` is already bound to some action.
    pub fn is_bound(&self, combo: &str) -> bool {
        self.bindings.contains_key(&normalize(combo))
    }

    /// Lists all registered bindings, sorted by key combination.
    pub fn list(&self) -> Vec<Binding> {
        let mut out: Vec<Binding> = self
            .bindings
            .iter()
            .map(|(combo, action)| Binding { combo: combo.clone(), action: action.clone() })
            .collect();
        out.sort_by(|a, b| a.combo.cmp(&b.combo));
        out
    }

    /// Number of registered bindings.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// True if no bindings are registered.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

/// Initializes the module (kept for parity with the rest of the workspace's
/// component crates, which all expose an async `init()`).
pub async fn init() -> Result<()> {
    info!("Init");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation() {
        let registry = KeymapRegistry::new();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_register_and_lookup() {
        let mut registry = KeymapRegistry::new();
        registry.register("Ctrl+S", "save").unwrap();
        assert_eq!(registry.lookup("ctrl+s"), Some("save"));
        assert_eq!(registry.lookup("CTRL + S"), Some("save"));
    }

    #[test]
    fn test_overwrite_returns_previous() {
        let mut registry = KeymapRegistry::new();
        registry.register("ctrl+s", "save").unwrap();
        let previous = registry.register("ctrl+s", "save-as").unwrap();
        assert_eq!(previous, Some("save".to_string()));
        assert_eq!(registry.lookup("ctrl+s"), Some("save-as"));
    }

    #[test]
    fn test_unregister() {
        let mut registry = KeymapRegistry::new();
        registry.register("ctrl+s", "save").unwrap();
        assert_eq!(registry.unregister("ctrl+s"), Some("save".to_string()));
        assert!(!registry.is_bound("ctrl+s"));
    }

    #[test]
    fn test_empty_combo_rejected() {
        let mut registry = KeymapRegistry::new();
        assert!(registry.register("  ", "save").is_err());
    }

    #[test]
    fn test_list_sorted() {
        let mut registry = KeymapRegistry::new();
        registry.register("ctrl+z", "undo").unwrap();
        registry.register("ctrl+a", "select-all").unwrap();
        let list = registry.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].combo, "ctrl+a");
        assert_eq!(list[1].combo, "ctrl+z");
    }

    #[tokio::test]
    async fn test_init() {
        assert!(init().await.is_ok());
    }
}
