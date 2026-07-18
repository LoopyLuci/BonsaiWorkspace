//! BMN Compositor CLI - builds a scene and renders one frame

use bmn_compositor::{Compositor, CompositorConfig, Scene, SceneElement, SceneElementType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut compositor = Compositor::new(CompositorConfig::new(1920, 1080));
    compositor.initialize().await?;

    {
        let scene_graph = compositor.scene_graph();
        let mut graph = scene_graph.write().await;
        let mut scene = Scene::new("Main", 1920, 1080, 60);
        scene.add_element(SceneElement::new("Webcam", SceneElementType::CameraCapture));
        graph.add_scene(scene);
    }

    let frame = compositor.render().await?;
    println!(
        "rendered frame: {}x{} format={:?} bytes={}",
        frame.width,
        frame.height,
        frame.format,
        frame.size_bytes()
    );

    let stats = compositor.get_stats().await;
    println!("frames rendered: {}", stats.frames_rendered);

    compositor.shutdown().await?;
    Ok(())
}
