//! Component Library
#![warn(missing_docs)]
pub mod error;
pub mod types;
pub use error::{Error, Result};
pub use types::*;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Props {
    pub id: String,
    pub class: String,
    pub disabled: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: "component".to_string(),
            class: String::new(),
            disabled: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Component {
    props: Props,
}

impl Component {
    pub fn new(props: Props) -> Self { info!("Init"); Self { props } }
    pub fn render(&self) -> String { format!("<div id=\"{}\" class=\"{}\">Component</div>", self.props.id, self.props.class) }
    pub fn props(&self) -> &Props { &self.props }
    pub fn update_props(&mut self, props: Props) { self.props = props; }
}

impl Default for Component {
    fn default() -> Self { Self::new(Props::default()) }
}

pub async fn init() -> Result<()> { info!("Init"); Ok(()) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_creation() { let _ = Component::new(Props::default()); }
    #[test]
    fn test_render() { assert!(Component::new(Props::default()).render().contains("Component")); }
    #[test]
    fn test_props() { assert_eq!(Component::new(Props::default()).props().id, "component"); }
    #[test]
    fn test_update() { let mut c = Component::new(Props::default()); c.update_props(Props { id: "new".into(), ..Default::default() }); assert_eq!(c.props().id, "new"); }
    #[tokio::test]
    async fn test_init() { assert!(init().await.is_ok()); }
    #[test]
    fn test_default() { let _ = Component::default(); }
}
