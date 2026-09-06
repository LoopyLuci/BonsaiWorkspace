//! Modal Component Library
//!
//! A real modal stack manager: open modals by id (stacking on top of each
//! other, like nested dialogs), close by id or close the topmost, and query
//! what is currently on top / how deep the stack is. This replaces the
//! earlier generic "Component Library" placeholder with logic actually
//! specific to this crate's name.
#![warn(missing_docs)]
pub mod error;
pub mod types;
pub use error::{Error, Result};
pub use types::*;
use tracing::info;

/// A single open modal, identified by id, with a z-index derived from its
/// stack position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenModal {
    /// Unique identifier for the modal.
    pub id: String,
    /// Stacking order: higher values render on top.
    pub z_index: u32,
}

/// Manages a stack of open modals.
#[derive(Debug, Clone, Default)]
pub struct ModalStack {
    stack: Vec<String>,
}

impl ModalStack {
    /// Creates an empty modal stack.
    pub fn new() -> Self {
        info!("Init");
        Self { stack: Vec::new() }
    }

    /// Opens `id`, pushing it to the top of the stack. Returns an error if
    /// that id is already open (re-opening should `close` first).
    pub fn open(&mut self, id: &str) -> Result<()> {
        if self.stack.iter().any(|m| m == id) {
            return Err(Error::Other(format!("modal '{id}' is already open")));
        }
        self.stack.push(id.to_string());
        Ok(())
    }

    /// Closes `id` wherever it is in the stack (not just the top). Returns
    /// true if it was found and removed.
    pub fn close(&mut self, id: &str) -> bool {
        if let Some(pos) = self.stack.iter().position(|m| m == id) {
            self.stack.remove(pos);
            true
        } else {
            false
        }
    }

    /// Closes the topmost modal, returning its id.
    pub fn close_top(&mut self) -> Option<String> {
        self.stack.pop()
    }

    /// The id of the topmost (currently focused) modal, if any is open.
    pub fn top(&self) -> Option<&str> {
        self.stack.last().map(String::as_str)
    }

    /// True if `id` is anywhere in the stack.
    pub fn is_open(&self, id: &str) -> bool {
        self.stack.iter().any(|m| m == id)
    }

    /// Number of currently open modals.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Full stack, bottom first, with derived z-indices (10, 20, 30, ...).
    pub fn stack(&self) -> Vec<OpenModal> {
        self.stack
            .iter()
            .enumerate()
            .map(|(i, id)| OpenModal { id: id.clone(), z_index: (i as u32 + 1) * 10 })
            .collect()
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
        let modals = ModalStack::new();
        assert_eq!(modals.depth(), 0);
        assert_eq!(modals.top(), None);
    }

    #[test]
    fn test_open_and_top() {
        let mut modals = ModalStack::new();
        modals.open("confirm-delete").unwrap();
        modals.open("nested-warning").unwrap();
        assert_eq!(modals.top(), Some("nested-warning"));
        assert_eq!(modals.depth(), 2);
    }

    #[test]
    fn test_open_duplicate_errors() {
        let mut modals = ModalStack::new();
        modals.open("confirm-delete").unwrap();
        assert!(modals.open("confirm-delete").is_err());
    }

    #[test]
    fn test_close_top() {
        let mut modals = ModalStack::new();
        modals.open("a").unwrap();
        modals.open("b").unwrap();
        assert_eq!(modals.close_top(), Some("b".to_string()));
        assert_eq!(modals.top(), Some("a"));
    }

    #[test]
    fn test_close_by_id_from_middle() {
        let mut modals = ModalStack::new();
        modals.open("a").unwrap();
        modals.open("b").unwrap();
        modals.open("c").unwrap();
        assert!(modals.close("b"));
        assert!(!modals.is_open("b"));
        assert_eq!(modals.top(), Some("c"));
        assert_eq!(modals.depth(), 2);
    }

    #[test]
    fn test_z_index_ordering() {
        let mut modals = ModalStack::new();
        modals.open("a").unwrap();
        modals.open("b").unwrap();
        let stack = modals.stack();
        assert_eq!(stack[0].id, "a");
        assert_eq!(stack[0].z_index, 10);
        assert_eq!(stack[1].id, "b");
        assert_eq!(stack[1].z_index, 20);
    }

    #[tokio::test]
    async fn test_init() {
        assert!(init().await.is_ok());
    }
}
