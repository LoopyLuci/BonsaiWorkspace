//! BMN Sources
//!
//! Concrete capture-source implementations (display, camera, audio, and a
//! virtual camera) built on the [`bmn_common::Source`] trait. Each source
//! manages its own lifecycle/health state and produces synthetic test
//! frames of the correct shape (platform capture backends are stubbed with
//! honest "not yet implemented" errors — see [`platform`]).

pub mod audio;
pub mod base;
pub mod camera;
pub mod display;
pub mod platform;

pub use audio::AudioSource;
pub use base::{NoopSource, SourceHealth, SourceState};
pub use camera::CameraSource;
pub use display::DisplaySource;
pub use platform::VirtualCameraSource;

/// Identity/config shared by all concrete source types in this crate
#[derive(Debug, Clone)]
pub struct SourceConfig {
    pub id: String,
    pub name: String,
    pub source_type: String,
}

impl SourceConfig {
    pub fn new(name: impl Into<String>, source_type: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            source_type: source_type.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_config_new() {
        let config = SourceConfig::new("Display 1", "display");
        assert_eq!(config.name, "Display 1");
        assert_eq!(config.source_type, "display");
        assert!(!config.id.is_empty());
    }

    #[test]
    fn test_source_config_unique_ids() {
        let a = SourceConfig::new("a", "display");
        let b = SourceConfig::new("b", "display");
        assert_ne!(a.id, b.id);
    }
}
