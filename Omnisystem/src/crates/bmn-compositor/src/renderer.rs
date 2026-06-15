// Compositor — main rendering engine

use crate::{Scene, SceneGraph, CompositionStats};
use bmn_common::{
    error::{BmnResult, BmnError},
    frame::{VideoFrame, PixelFormat},
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Render target configuration
#[derive(Debug, Clone)]
pub struct RenderTarget {
    pub width: u32,
    pub height: u32,
    pub pixel_format: PixelFormat,
    pub hdr: bool,
}

impl RenderTarget {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixel_format: PixelFormat::RGBA,
            hdr: false,
        }
    }

    pub fn with_hdr(mut self) -> Self {
        self.hdr = true;
        self.pixel_format = PixelFormat::P010; // 10-bit format
        self
    }

    pub fn frame_size(&self) -> usize {
        match self.pixel_format {
            PixelFormat::RGBA => (self.width * self.height * 4) as usize,
            PixelFormat::BGRA => (self.width * self.height * 4) as usize,
            PixelFormat::YUV420 => {
                let y = (self.width * self.height) as usize;
                let uv = ((self.width * self.height) / 4) as usize;
                y + 2 * uv
            }
            PixelFormat::NV12 => {
                let y = (self.width * self.height) as usize;
                let uv = ((self.width * self.height) / 2) as usize;
                y + uv
            }
            PixelFormat::P010 => {
                let y = (self.width * self.height * 2) as usize;
                let uv = ((self.width * self.height) / 2) as usize;
                y + uv
            }
        }
    }
}

/// Compositor configuration
#[derive(Debug, Clone)]
pub struct CompositorConfig {
    pub render_target: RenderTarget,
    pub use_gpu: bool,
    pub max_layers: u32,
    pub enable_hdr: bool,
}

impl CompositorConfig {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            render_target: RenderTarget::new(width, height),
            use_gpu: true,
            max_layers: 100,
            enable_hdr: false,
        }
    }

    pub fn with_gpu(mut self, enabled: bool) -> Self {
        self.use_gpu = enabled;
        self
    }

    pub fn with_hdr(mut self) -> Self {
        self.enable_hdr = true;
        self.render_target = self.render_target.clone().with_hdr();
        self
    }
}

/// Main compositor
pub struct Compositor {
    config: CompositorConfig,
    scene_graph: Arc<RwLock<SceneGraph>>,
    stats: Arc<RwLock<CompositionStats>>,
    initialized: bool,
}

impl Compositor {
    pub fn new(config: CompositorConfig) -> Self {
        Self {
            config,
            scene_graph: Arc::new(RwLock::new(SceneGraph::new())),
            stats: Arc::new(RwLock::new(CompositionStats::default())),
            initialized: false,
        }
    }

    pub async fn initialize(&mut self) -> BmnResult<()> {
        // Platform-specific GPU initialization would happen here
        #[cfg(feature = "vulkan")]
        {
            // Vulkan initialization
            tracing::info!("Initializing Vulkan compositor");
        }

        self.initialized = true;
        Ok(())
    }

    pub async fn shutdown(&mut self) -> BmnResult<()> {
        // Cleanup GPU resources
        self.initialized = false;
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn config(&self) -> &CompositorConfig {
        &self.config
    }

    pub fn scene_graph(&self) -> Arc<RwLock<SceneGraph>> {
        self.scene_graph.clone()
    }

    /// Render the active scene to a frame
    pub async fn render(&self) -> BmnResult<VideoFrame> {
        if !self.initialized {
            return Err(BmnError::internal("Compositor not initialized"));
        }

        let graph = self.scene_graph.read().await;
        let scene = graph
            .get_active_scene()
            .ok_or_else(|| BmnError::internal("No active scene"))?;

        // Create render frame
        let frame_size = self.config.render_target.frame_size();
        let frame_data = vec![0u8; frame_size];

        // In a real implementation, this would render the scene to GPU memory
        // For now, we return a placeholder frame
        let frame = VideoFrame {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            width: self.config.render_target.width,
            height: self.config.render_target.height,
            format: self.config.render_target.pixel_format,
            data: Arc::new(bytes::Bytes::from(frame_data)),
        };

        // Update stats
        let mut stats = self.stats.write().await;
        stats.frames_rendered += 1;

        Ok(frame)
    }

    pub async fn get_stats(&self) -> CompositionStats {
        self.stats.read().await.clone()
    }

    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = CompositionStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compositor_config() {
        let config = CompositorConfig::new(1920, 1080);
        assert_eq!(config.render_target.width, 1920);
        assert_eq!(config.render_target.height, 1080);
        assert!(config.use_gpu);
    }

    #[test]
    fn test_render_target_frame_size() {
        let target = RenderTarget::new(1920, 1080);
        let size = target.frame_size();
        assert_eq!(size, 1920 * 1080 * 4); // RGBA
    }

    #[tokio::test]
    async fn test_compositor_initialization() {
        let mut compositor = Compositor::new(CompositorConfig::new(1920, 1080));
        assert!(!compositor.is_initialized());

        compositor.initialize().await.unwrap();
        assert!(compositor.is_initialized());

        compositor.shutdown().await.unwrap();
        assert!(!compositor.is_initialized());
    }

    #[tokio::test]
    async fn test_compositor_stats() {
        let compositor = Compositor::new(CompositorConfig::new(1920, 1080));
        let stats = compositor.get_stats().await;

        assert_eq!(stats.frames_rendered, 0);
        assert_eq!(stats.active_layers, 0);
    }

    #[test]
    fn test_compositor_with_hdr() {
        let config = CompositorConfig::new(1920, 1080).with_hdr();
        assert!(config.enable_hdr);
        assert_eq!(config.render_target.pixel_format, PixelFormat::P010);
    }
}
