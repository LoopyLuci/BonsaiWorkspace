//! Navigation Components
//!
//! A real navigation stack: push routes, go back, peek at the current
//! route, and inspect history. This replaces the earlier generic
//! "Component Library" placeholder with logic actually specific to this
//! crate's name.
#![warn(missing_docs)]
pub mod error;
pub mod types;
pub use error::{Error, Result};
pub use types::*;
use tracing::info;

/// A single entry in the navigation history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    /// Path of the route, e.g. "/settings/profile".
    pub path: String,
    /// Optional human-readable title for this route.
    pub title: String,
}

impl Route {
    /// Creates a new route with the given path and title.
    pub fn new(path: impl Into<String>, title: impl Into<String>) -> Self {
        Self { path: path.into(), title: title.into() }
    }
}

/// A navigation stack. `push` moves forward to a new route (recording
/// history); `back` pops the current route and returns to the previous one.
#[derive(Debug, Clone)]
pub struct NavigationStack {
    stack: Vec<Route>,
}

impl NavigationStack {
    /// Creates a stack starting at the given root route.
    pub fn new(root: Route) -> Self {
        info!("Init");
        Self { stack: vec![root] }
    }

    /// Pushes a new route onto the stack, making it current.
    pub fn push(&mut self, route: Route) {
        self.stack.push(route);
    }

    /// Pops the current route and returns to the previous one. Returns
    /// `Err` if already at the root (nothing to go back to).
    pub fn back(&mut self) -> Result<Route> {
        if self.stack.len() <= 1 {
            return Err(Error::Other("already at the root route".to_string()));
        }
        self.stack.pop();
        Ok(self.current().clone())
    }

    /// The currently active route.
    pub fn current(&self) -> &Route {
        self.stack.last().expect("stack always has at least the root route")
    }

    /// True if there is history to go back to.
    pub fn can_go_back(&self) -> bool {
        self.stack.len() > 1
    }

    /// Depth of the navigation stack (1 == just the root).
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Full history, root first, current last.
    pub fn history(&self) -> &[Route] {
        &self.stack
    }
}

impl Default for NavigationStack {
    fn default() -> Self {
        Self::new(Route::new("/", "Home"))
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
        let nav = NavigationStack::default();
        assert_eq!(nav.current().path, "/");
        assert_eq!(nav.depth(), 1);
        assert!(!nav.can_go_back());
    }

    #[test]
    fn test_push_and_back() {
        let mut nav = NavigationStack::default();
        nav.push(Route::new("/settings", "Settings"));
        assert_eq!(nav.current().path, "/settings");
        assert!(nav.can_go_back());

        let back_to = nav.back().unwrap();
        assert_eq!(back_to.path, "/");
        assert!(!nav.can_go_back());
    }

    #[test]
    fn test_back_at_root_errors() {
        let mut nav = NavigationStack::default();
        assert!(nav.back().is_err());
    }

    #[test]
    fn test_history() {
        let mut nav = NavigationStack::default();
        nav.push(Route::new("/a", "A"));
        nav.push(Route::new("/b", "B"));
        let paths: Vec<_> = nav.history().iter().map(|r| r.path.as_str()).collect();
        assert_eq!(paths, vec!["/", "/a", "/b"]);
    }

    #[tokio::test]
    async fn test_init() {
        assert!(init().await.is_ok());
    }
}
