//! BMN Compositor
//!
//! A scene-graph based compositing engine: scenes/elements/layers
//! ([`scene`], [`core`]), transforms ([`transform`]), blend modes
//! ([`blending`]), effects/transitions ([`effect`]), and the render loop
//! ([`renderer`]) that turns a [`scene::Scene`] into a
//! [`bmn_common::frame::VideoFrame`].

pub mod blending;
pub mod core;
pub mod effect;
pub mod renderer;
pub mod scene;
pub mod transform;

pub use blending::BlendMode;
pub use core::{CompositionStats, ElementId, Layer};
pub use effect::{Effect, EffectType, Transition};
pub use renderer::{Compositor, CompositorConfig, RenderTarget};
pub use scene::{Scene, SceneElement, SceneElementType, SceneGraph};
pub use transform::Transform;
