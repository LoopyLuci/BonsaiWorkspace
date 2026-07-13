//! Feature UI Module
#![warn(missing_docs)]
pub mod error;
pub mod types;
pub use error::{Error, Result};
pub use types::*;
use tracing::info;

#[derive(Debug, Clone)]
pub struct UI {
    visible: bool,
    data: String,
}

impl UI {
    pub fn new() -> Self { info!("Init"); Self { visible: true, data: String::new() } }
    pub fn render(&self) -> String { if self.visible { format!("<div>{}</div>", self.data) } else { String::new() } }
    pub fn update(&mut self, data: String) -> Result<()> { self.data = data; Ok(()) }
    pub fn toggle(&mut self) { self.visible = !self.visible; }
}

impl Default for UI {
    fn default() -> Self { Self::new() }
}

pub async fn init() -> Result<()> { info!("Init"); Ok(()) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_creation() { let _ = UI::new(); }
    #[test]
    fn test_render() { assert!(!UI::new().render().is_empty()); }
    #[test]
    fn test_update() { let mut u = UI::new(); assert!(u.update("data".into()).is_ok()); }
    #[test]
    fn test_toggle() { let mut u = UI::new(); u.toggle(); assert!(!u.visible); }
    #[tokio::test]
    async fn test_init() { assert!(init().await.is_ok()); }
}
