//! Core compositor primitives: element identifiers, per-element layer
//! state (transform/opacity/blend mode), and aggregated composition stats.

use crate::{BlendMode, Transform};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a scene element or scene
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ElementId(pub Uuid);

impl ElementId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ElementId {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-element compositing layer state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub transform: Transform,
    pub opacity: f32,
    pub blend_mode: BlendMode,
}

impl Layer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transform: Transform::identity(),
            opacity: 1.0,
            blend_mode: BlendMode::default(),
        }
    }
}

/// Aggregated compositor statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompositionStats {
    pub frames_rendered: u64,
    pub active_layers: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_id_unique() {
        assert_ne!(ElementId::new(), ElementId::new());
    }

    #[test]
    fn test_layer_defaults() {
        let layer = Layer::new("default");
        assert_eq!(layer.name, "default");
        assert_eq!(layer.opacity, 1.0);
        assert_eq!(layer.blend_mode, BlendMode::Alpha);
    }

    #[test]
    fn test_composition_stats_default() {
        let stats = CompositionStats::default();
        assert_eq!(stats.frames_rendered, 0);
        assert_eq!(stats.active_layers, 0);
    }
}
