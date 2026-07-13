// Scene graph structures

use crate::{ElementId, Layer, Transform, BlendMode, Effect};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of scene elements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneElementType {
    // Video sources
    DisplayCapture,
    CameraCapture,
    VideoFile,
    BrowserSource,

    // Graphics
    Image,
    ColorRect,

    // Text
    Text,

    // Groups
    Group,
}

/// Scene element (source, image, text, etc.)
#[derive(Debug, Clone)]
pub struct SceneElement {
    pub id: ElementId,
    pub name: String,
    pub element_type: SceneElementType,
    pub layer: Layer,
    pub visible: bool,
    pub locked: bool,
    pub properties: HashMap<String, String>,
    pub effects: Vec<Effect>,
}

impl SceneElement {
    pub fn new(
        name: impl Into<String>,
        element_type: SceneElementType,
    ) -> Self {
        Self {
            id: ElementId::new(),
            name: name.into(),
            element_type,
            layer: Layer::new("default"),
            visible: true,
            locked: false,
            properties: HashMap::new(),
            effects: Vec::new(),
        }
    }

    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.properties.insert("width".into(), width.to_string());
        self.properties.insert("height".into(), height.to_string());
        self
    }

    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.layer.transform.position.x = x;
        self.layer.transform.position.y = y;
        self
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.layer.opacity = opacity.max(0.0).min(1.0);
        self
    }

    pub fn with_blend_mode(mut self, blend_mode: BlendMode) -> Self {
        self.layer.blend_mode = blend_mode;
        self
    }

    pub fn add_effect(mut self, effect: Effect) -> Self {
        self.effects.push(effect);
        self
    }
}

/// Scene composition
#[derive(Debug, Clone)]
pub struct Scene {
    pub id: ElementId,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub elements: Vec<SceneElement>,
    pub groups: HashMap<String, Vec<ElementId>>,
}

impl Scene {
    pub fn new(
        name: impl Into<String>,
        width: u32,
        height: u32,
        fps: u32,
    ) -> Self {
        Self {
            id: ElementId::new(),
            name: name.into(),
            width,
            height,
            fps,
            elements: Vec::new(),
            groups: HashMap::new(),
        }
    }

    pub fn add_element(&mut self, element: SceneElement) -> ElementId {
        let id = element.id;
        self.elements.push(element);
        id
    }

    pub fn remove_element(&mut self, id: ElementId) -> Option<SceneElement> {
        if let Some(pos) = self.elements.iter().position(|e| e.id == id) {
            Some(self.elements.remove(pos))
        } else {
            None
        }
    }

    pub fn get_element(&self, id: ElementId) -> Option<&SceneElement> {
        self.elements.iter().find(|e| e.id == id)
    }

    pub fn get_element_mut(&mut self, id: ElementId) -> Option<&mut SceneElement> {
        self.elements.iter_mut().find(|e| e.id == id)
    }

    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    pub fn visible_elements(&self) -> Vec<&SceneElement> {
        self.elements.iter().filter(|e| e.visible).collect()
    }

    pub fn reorder_element(&mut self, id: ElementId, new_index: usize) -> bool {
        if let Some(pos) = self.elements.iter().position(|e| e.id == id) {
            let element = self.elements.remove(pos);
            let insert_pos = new_index.min(self.elements.len());
            self.elements.insert(insert_pos, element);
            true
        } else {
            false
        }
    }
}

/// Scene graph — manages scene hierarchy
#[derive(Debug, Clone)]
pub struct SceneGraph {
    pub scenes: HashMap<ElementId, Scene>,
    pub active_scene: Option<ElementId>,
}

impl SceneGraph {
    pub fn new() -> Self {
        Self {
            scenes: HashMap::new(),
            active_scene: None,
        }
    }

    pub fn add_scene(&mut self, scene: Scene) -> ElementId {
        let id = scene.id;
        self.scenes.insert(id, scene);
        if self.active_scene.is_none() {
            self.active_scene = Some(id);
        }
        id
    }

    pub fn remove_scene(&mut self, id: ElementId) -> Option<Scene> {
        let scene = self.scenes.remove(&id);
        if self.active_scene == Some(id) {
            self.active_scene = self.scenes.keys().next().copied();
        }
        scene
    }

    pub fn get_active_scene(&self) -> Option<&Scene> {
        self.active_scene.and_then(|id| self.scenes.get(&id))
    }

    pub fn get_active_scene_mut(&mut self) -> Option<&mut Scene> {
        self.active_scene.and_then(|id| self.scenes.get_mut(&id))
    }

    pub fn set_active_scene(&mut self, id: ElementId) -> bool {
        if self.scenes.contains_key(&id) {
            self.active_scene = Some(id);
            true
        } else {
            false
        }
    }

    pub fn scene_count(&self) -> usize {
        self.scenes.len()
    }
}

impl Default for SceneGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_creation() {
        let scene = Scene::new("Test", 1920, 1080, 60);
        assert_eq!(scene.name, "Test");
        assert_eq!(scene.width, 1920);
        assert_eq!(scene.height, 1080);
        assert_eq!(scene.fps, 60);
        assert_eq!(scene.element_count(), 0);
    }

    #[test]
    fn test_scene_add_element() {
        let mut scene = Scene::new("Test", 1920, 1080, 60);
        let element = SceneElement::new("Source", SceneElementType::DisplayCapture);
        let id = element.id;

        scene.add_element(element);
        assert_eq!(scene.element_count(), 1);
        assert!(scene.get_element(id).is_some());
    }

    #[test]
    fn test_scene_remove_element() {
        let mut scene = Scene::new("Test", 1920, 1080, 60);
        let element = SceneElement::new("Source", SceneElementType::DisplayCapture);
        let id = element.id;

        scene.add_element(element);
        assert_eq!(scene.element_count(), 1);

        let removed = scene.remove_element(id);
        assert!(removed.is_some());
        assert_eq!(scene.element_count(), 0);
    }

    #[test]
    fn test_scene_graph() {
        let mut graph = SceneGraph::new();

        let scene1 = Scene::new("Scene 1", 1920, 1080, 60);
        let id1 = scene1.id;
        graph.add_scene(scene1);

        let scene2 = Scene::new("Scene 2", 1280, 720, 30);
        let id2 = scene2.id;
        graph.add_scene(scene2);

        assert_eq!(graph.scene_count(), 2);
        assert_eq!(graph.active_scene, Some(id1));

        graph.set_active_scene(id2);
        assert_eq!(graph.active_scene, Some(id2));
    }

    #[test]
    fn test_element_builder() {
        let element = SceneElement::new("Test", SceneElementType::Image)
            .with_size(1920, 1080)
            .with_position(100.0, 50.0)
            .with_opacity(0.5);

        assert_eq!(element.layer.opacity, 0.5);
        assert_eq!(element.layer.transform.position.x, 100.0);
    }
}
